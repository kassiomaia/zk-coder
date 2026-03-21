//! Google web search tool (dev/test only).

use crate::tools::constants::ENV_GOOGLE_SCRAPE_ENABLED;
use crate::tools::{
    context::ToolExecutionContext, error::ToolError, traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::time::Duration;
use tauri::AppHandle;

/// Arguments for Google web search.
#[derive(Debug, Deserialize)]
pub struct GoogleSearchArgs {
    /// Search query string.
    pub query: String,
    /// Number of results to return (default: 5, max: 10).
    #[serde(default = "default_num_results")]
    pub num_results: usize,
    /// Language code (default: "en").
    #[serde(default)]
    pub language: Option<String>,
}

fn default_num_results() -> usize {
    5
}

/// Individual search result.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    /// Result title.
    pub title: String,
    /// Result URL.
    pub url: String,
    /// Result snippet/description.
    pub snippet: String,
    /// Source identifier.
    pub source: String,
}

/// Output from web search.
#[derive(Debug, Serialize)]
pub struct GoogleSearchOutput {
    /// Search query.
    pub query: String,
    /// Number of results returned.
    pub count: usize,
    /// Search results.
    pub results: Vec<SearchResult>,
    /// Warning about scraper stability.
    pub warning: String,
}

/// Dev-only keyless Google web search using HTML scraping (unstable).
pub struct GoogleScrapeTool;

impl Tool for GoogleScrapeTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("search", "google_scrape"),
            description: "Dev-only keyless Google web search using HTML scraping (unstable).",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string"
                    },
                    "num_results": {
                        "type": "integer",
                        "description": "Number of results (default: 5, max: 10)",
                        "minimum": 1,
                        "maximum": 10
                    },
                    "language": {
                        "type": "string",
                        "description": "Language code (default: en)"
                    }
                },
                "required": ["query"]
            }),
            requires_workspace: false,
            experimental: true,
            feature_flag: Some(ENV_GOOGLE_SCRAPE_ENABLED),
        }
    }

    fn execute(
        &self,
        _context: ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<std::path::PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            // Check feature flag
            if !crate::tools::constants::is_env_enabled(ENV_GOOGLE_SCRAPE_ENABLED) {
                return Err(ToolError::Disabled(format!(
                    "{}=1 is required for dev/test use",
                    ENV_GOOGLE_SCRAPE_ENABLED
                )));
            }

            let args: GoogleSearchArgs =
                crate::tools::error::parse_args("search_google_scrape", arguments)?;

            let query = args.query.trim().to_string();
            if query.is_empty() {
                return Err(ToolError::invalid_args("Query cannot be empty"));
            }

            let num_results = args.num_results.clamp(1, 10);

            // Build query parameters
            let query_params = vec![
                ("q", query.clone()),
                ("num", num_results.to_string()),
                ("hl", args.language.unwrap_or_else(|| "en".to_string())),
                ("pws", "0".to_string()),
                ("safe", "off".to_string()),
            ];

            // Build HTTP client
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(12))
                .user_agent(
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
                     (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
                )
                .build()
                .map_err(|e| ToolError::failed(format!("Failed to build HTTP client: {}", e)))?;

            // Execute search
            let response = client
                .get("https://www.google.com/search")
                .query(&query_params)
                .send()
                .await
                .map_err(|e| ToolError::from_reqwest(e))?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(ToolError::failed(format!(
                    "Google search failed with status {}: {}",
                    status.as_u16(),
                    body.chars().take(200).collect::<String>()
                )));
            }

            // Parse HTML response
            let html = response.text().await.map_err(|e| {
                ToolError::failed(format!("Failed to read response body: {}", e))
            })?;

            let results = GoogleScrapeTool::parse_google_search_results(&html, num_results);

            let output = GoogleSearchOutput {
                query,
                count: results.len(),
                results,
                warning: "Dev/test scraper: unstable markup and ToS-sensitive.".to_string(),
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

impl GoogleScrapeTool {
    /// Decode HTML entities.
    fn decode_html_entities(input: &str) -> String {
        input
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&nbsp;", " ")
    }

    /// Strip HTML tags from text.
    fn strip_html_tags(input: &str) -> String {
        let without_tags =
            Regex::new(r"<[^>]+>")
                .ok()
                .map_or_else(|| input.to_string(), |tag_re| {
                    tag_re.replace_all(input, "").to_string()
                });
        let decoded = Self::decode_html_entities(without_tags.as_ref());
        decoded.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Extract URL from Google's encoded href.
    fn extract_google_result_url(encoded_href_query: &str) -> Option<String> {
        let decoded = Self::decode_html_entities(encoded_href_query);
        let query = if decoded.starts_with("q=") {
            decoded
        } else {
            format!("q={}", decoded)
        };
        let url = reqwest::Url::parse(&format!("https://www.google.com/url?{}", query)).ok()?;
        for (key, value) in url.query_pairs() {
            if key == "q" && (value.starts_with("http://") || value.starts_with("https://"))
            {
                return Some(value.into_owned());
            }
        }
        None
    }

    /// Parse Google search results from HTML.
    fn parse_google_search_results(html: &str, limit: usize) -> Vec<SearchResult> {
        let anchor_prefix = "<a href=\"/url?q=";
        let mut results = Vec::new();
        let mut cursor = 0;

        while results.len() < limit {
            let Some(anchor_offset) = html[cursor..].find(anchor_prefix) else {
                break;
            };
            let anchor_start = cursor + anchor_offset;
            let href_start = anchor_start + anchor_prefix.len();
            let Some(href_end_rel) = html[href_start..].find('"') else {
                break;
            };
            let href_end = href_start + href_end_rel;
            let href_value = &html[href_start..href_end];
            cursor = href_end.saturating_add(1);

            let Some(url) = Self::extract_google_result_url(href_value) else {
                continue;
            };

            let window_end = (anchor_start + 5000).min(html.len());
            let window = &html[anchor_start..window_end];

            let Some(h3_start_rel) = window.find("<h3") else {
                continue;
            };
            let h3_start = h3_start_rel;
            let Some(h3_open_end_rel) = window[h3_start..].find('>') else {
                continue;
            };
            let title_start = h3_start + h3_open_end_rel + 1;
            let Some(title_end_rel) = window[title_start..].find("</h3>") else {
                continue;
            };
            let title_end = title_start + title_end_rel;
            let title = Self::strip_html_tags(&window[title_start..title_end]);
            if title.is_empty() {
                continue;
            }

            let snippet = window
                .find("VwiC3b")
                .and_then(|class_offset| window[class_offset..].find('>').map(|o| class_offset + o))
                .and_then(|snippet_start| {
                    let content_start = snippet_start + 1;
                    window[content_start..]
                        .find("</div>")
                        .map(|snippet_end_rel| (content_start, content_start + snippet_end_rel))
                })
                .map(|(start, end)| Self::strip_html_tags(&window[start..end]))
                .unwrap_or_default();

            results.push(SearchResult {
                title,
                url,
                snippet,
                source: "google_scrape".to_string(),
            });
        }

        results
    }
}
