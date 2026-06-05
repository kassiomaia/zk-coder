//! `GoogleProvider` — real `SearchProvider` backed by Google Custom Search JSON API.
//!
//! - Endpoint: `GET https://www.googleapis.com/customsearch/v1`
//! - Required params: `key`, `cx`, `q`, `num`
//! - Optional params: `hl` (language), `siteSearch` (site restriction)
//!
//! Tests use `wiremock` to validate the request shape and response mapping
//! without ever hitting Google. The live integration test behind
//! `--features integration-test` exercises the real endpoint when the
//! developer's env has `CODER_LLM_TOOL_WEB_SEARCH_API_KEY` + `CODER_LLM_TOOL_WEB_SEARCH_CX`.

use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;

use super::super::secret::RedactedSecret;
use super::{RawResult, SearchProvider, SearchProviderError, SearchRequest};

/// Canonical upstream base URL. Overridable per-instance through
/// [`GoogleProvider::with_base_url`] so offline tests can point at a wiremock
/// server without compile-time trickery.
const DEFAULT_BASE_URL: &str = "https://www.googleapis.com";

/// Request timeout. Matches the spec's 10 s hard limit on a single upstream call.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Google Custom Search JSON API provider.
pub struct GoogleProvider {
    client: Client,
    api_key: RedactedSecret,
    cx: String,
    base_url: String,
}

impl GoogleProvider {
    /// Builds a provider with a default `reqwest::Client` — 10 s timeout,
    /// JSON-friendly user-agent.
    #[must_use]
    pub fn new(api_key: RedactedSecret, cx: String) -> Self {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("Coder/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self::with_client(api_key, cx, client)
    }

    /// Builds a provider around a caller-supplied client (for wiremock-backed
    /// tests that configure timeouts / redirect policies differently).
    #[must_use]
    pub fn with_client(api_key: RedactedSecret, cx: String, client: Client) -> Self {
        Self {
            client,
            api_key,
            cx,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Overrides the upstream base URL. Used exclusively by offline tests.
    #[must_use]
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.base_url = base.into();
        self
    }
}

#[async_trait]
impl SearchProvider for GoogleProvider {
    async fn search(&self, req: SearchRequest) -> Result<Vec<RawResult>, SearchProviderError> {
        let url = format!("{}/customsearch/v1", self.base_url);

        // Query parameters. `num` is always present so Google does not silently
        // truncate to its default; `hl` / `siteSearch` are only added when the
        // caller set them.
        let mut params: Vec<(&str, String)> = vec![
            ("key", self.api_key.expose().to_string()),
            ("cx", self.cx.clone()),
            ("q", req.query),
            ("num", req.num_results.to_string()),
        ];
        if let Some(hl) = req.language {
            params.push(("hl", hl));
        }
        if let Some(site) = req.site {
            params.push(("siteSearch", site));
        }

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(map_transport_error)?;

        let status = response.status();

        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(SearchProviderError::RateLimited);
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            // Google often returns a JSON body with a "API key not valid"
            // style message on 401/403. Surface it so the Settings UI shows
            // something actionable instead of a generic Unauthorized.
            let detail = read_error_message(response).await;
            log::warn!(
                target: "web_search",
                "provider unauthorized: {}",
                detail.as_deref().unwrap_or("<no detail>")
            );
            return Err(SearchProviderError::Unauthorized);
        }
        if !status.is_success() {
            let code = status.as_u16();
            let detail = read_error_message(response).await;
            let message = match detail {
                Some(m) => format!("HTTP {code}: {m}"),
                None => format!("HTTP {code}"),
            };
            return Err(SearchProviderError::Upstream(message));
        }

        let body: GoogleSearchResponse = response
            .json()
            .await
            .map_err(|e| SearchProviderError::Upstream(format!("response parse error: {e}")))?;

        Ok(body
            .items
            .unwrap_or_default()
            .into_iter()
            .map(map_item)
            .collect())
    }
}

/// Pulls the `error.message` field out of a Google API JSON error body. If
/// the body isn't JSON or doesn't carry a message, falls back to the raw
/// text (truncated) so the Settings UI still shows *something* actionable.
async fn read_error_message(response: reqwest::Response) -> Option<String> {
    let body = response.text().await.ok()?;
    if let Ok(parsed) = serde_json::from_str::<GoogleErrorEnvelope>(&body) {
        if let Some(message) = parsed.error.and_then(|e| e.message) {
            return Some(message);
        }
    }
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Cap at 240 chars so a huge HTML error page doesn't drown the banner.
    let capped: String = trimmed.chars().take(240).collect();
    Some(capped)
}

#[derive(Debug, Deserialize)]
struct GoogleErrorEnvelope {
    #[serde(default)]
    error: Option<GoogleErrorBody>,
}

#[derive(Debug, Deserialize)]
struct GoogleErrorBody {
    #[serde(default)]
    message: Option<String>,
}

fn map_transport_error(err: reqwest::Error) -> SearchProviderError {
    // Timeouts, DNS, TLS handshake errors all funnel here. The spec maps them
    // to `Upstream` at the tool layer; the provider records the more-specific
    // `Network` variant so error logs make it clear the request never reached
    // Google.
    SearchProviderError::Network(err.to_string())
}

fn map_item(item: GoogleItem) -> RawResult {
    RawResult {
        title: item.title,
        url: item.link,
        snippet_html: item.html_snippet.or(item.snippet).unwrap_or_default(),
    }
}

#[derive(Debug, Deserialize)]
struct GoogleSearchResponse {
    /// `items` is absent from the JSON response when Google returned zero results.
    #[serde(default)]
    items: Option<Vec<GoogleItem>>,
}

#[derive(Debug, Deserialize)]
struct GoogleItem {
    title: String,
    link: String,
    #[serde(default)]
    snippet: Option<String>,
    /// `htmlSnippet` carries the same text with `<b>` highlighting preserved;
    /// we prefer it so downstream sanitization can render keyword matches
    /// consistently with the frontend mockups.
    #[serde(rename = "htmlSnippet", default)]
    html_snippet: Option<String>,
}
