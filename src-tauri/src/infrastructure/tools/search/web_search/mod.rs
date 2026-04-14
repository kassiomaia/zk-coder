//! Production web search tool backed by the Google Custom Search JSON API.
//!
//! The feature contract lives in `docs/tools/web_search/spec.md`. The phased
//! implementation plan is in `docs/tools/web_search/plan.md`.
//!
//! Phase 3 lands the full `Tool` impl against the injected [`SearchProvider`]
//! seam. Real HTTP, DB credentials, workspace policy, and result caching are
//! bolted on in later phases through the same entry point — this file's
//! structure does not need to change when they arrive.

pub mod credentials;
pub mod error;
pub mod provider;
pub mod sanitize;
pub mod secret;
pub mod types;

#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::time::Instant;

use serde_json::Value as JsonValue;
use tauri::AppHandle;

use crate::infrastructure::tools::constants::{is_env_enabled, ENV_WEB_SEARCH_ENABLED};
use crate::infrastructure::tools::context::ToolExecutionContext;
use crate::infrastructure::tools::error::{parse_args, ToolError};
use crate::infrastructure::tools::traits::{Tool, ToolFuture, ToolId, ToolMeta};

use self::error::WebSearchFailure;
use self::provider::{RawResult, SearchProvider, SearchRequest};
use self::sanitize::{sanitize_snippet, source_domain, strip_tracking_params};
use self::types::{
    WebSearchArgs, WebSearchOutput, WebSearchResult, PROVIDER_GOOGLE_CUSTOM_SEARCH,
};

/// Production-grade web search tool.
///
/// Credentials, workspace policy, and caching are all delegated to collaborators
/// injected through the provider — Phase 5/5a/5b replace the provider factory
/// with one that resolves real credentials and caches results. This file stays
/// stable across those phases.
pub struct WebSearchTool {
    provider: Arc<dyn SearchProvider>,
}

impl WebSearchTool {
    /// Constructs a tool instance backed by the given provider. Used by
    /// Phase-3 tests with `MockProvider` and by Phase 6 when the real
    /// `GoogleProvider` factory lands.
    #[must_use]
    pub fn new_with_provider(provider: Arc<dyn SearchProvider>) -> Self {
        Self { provider }
    }
}

impl Tool for WebSearchTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("search", "web_search"),
            description: "Searches the public web via the Google Custom Search JSON API. \
                          Returns normalized title/url/snippet/source_domain results. \
                          Gated by the CODER_LLM_TOOL_WEB_SEARCH flag and DB-stored credentials.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string.",
                        "minLength": 1,
                        "maxLength": 512
                    },
                    "num_results": {
                        "type": "integer",
                        "description": "Number of results to return (default 5, max 10).",
                        "minimum": 1,
                        "maximum": 10
                    },
                    "language": {
                        "type": "string",
                        "description": "Optional BCP-47 language tag (e.g., 'en', 'pt-BR'). \
                                        Defaults to the host system locale."
                    },
                    "site": {
                        "type": "string",
                        "description": "Optional site restriction (e.g., 'github.com'). \
                                        Must pass the active workspace's domain policy."
                    }
                },
                "required": ["query"]
            }),
            requires_workspace: false,
            experimental: false,
            feature_flag: Some(ENV_WEB_SEARCH_ENABLED),
        }
    }

    fn execute(
        &self,
        _ctx: ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<std::path::PathBuf>,
    ) -> ToolFuture {
        let provider = Arc::clone(&self.provider);
        Box::pin(async move {
            let started = Instant::now();
            let result = execute_inner(provider, arguments).await;
            let elapsed_ms = started.elapsed().as_millis();

            match result {
                Ok(output) => {
                    log::info!(
                        target: "web_search",
                        "search ok query={:?} count={} cached={} elapsed_ms={}",
                        output.query,
                        output.count,
                        output.cached,
                        elapsed_ms
                    );
                    Ok(serde_json::to_value(output).map_err(ToolError::from)?)
                }
                Err(failure) => {
                    let body = failure.into_body();
                    log::warn!(
                        target: "web_search",
                        "search failed error_code={} elapsed_ms={}",
                        body.error_code.as_str(),
                        elapsed_ms
                    );
                    Ok(serde_json::to_value(body).map_err(ToolError::from)?)
                }
            }
        })
    }
}

/// Pure (aside from the provider call) implementation of the tool's business
/// logic. Split out so unit tests can compose it with `MockProvider` without
/// reaching through the Tauri `Tool` trait.
pub(crate) async fn execute_inner(
    provider: Arc<dyn SearchProvider>,
    arguments: JsonValue,
) -> Result<WebSearchOutput, WebSearchFailure> {
    // 1. Feature flag (belt + suspenders: registry also enforces this, but the
    //    tool checks again so direct callers and tests get a structured code).
    if !is_env_enabled(ENV_WEB_SEARCH_ENABLED) {
        return Err(WebSearchFailure::DisabledByFlag);
    }

    // 2. Parse and validate arguments.
    let raw: WebSearchArgs = parse_args("search_web_search", arguments)
        .map_err(|e| WebSearchFailure::InvalidArguments(e.to_string()))?;
    let args = raw
        .into_validated()
        .map_err(|e| WebSearchFailure::InvalidArguments(e.to_string()))?;

    // 3. Issue the provider request.
    let request = SearchRequest {
        query: args.query.clone(),
        num_results: args.num_results,
        language: args.language.clone(),
        site: args.site.clone(),
    };
    let raws = provider.search(request).await?;

    // 4. Sanitize + reshape. Results that lack a usable domain are dropped.
    let results: Vec<WebSearchResult> = raws
        .into_iter()
        .filter_map(normalize_raw_result)
        .take(args.num_results)
        .collect();

    Ok(WebSearchOutput {
        query: args.query,
        count: results.len(),
        results,
        provider: PROVIDER_GOOGLE_CUSTOM_SEARCH,
        cached: false,
        filtered_by_policy: None,
    })
}

fn normalize_raw_result(raw: RawResult) -> Option<WebSearchResult> {
    let url = strip_tracking_params(&raw.url);
    let source_domain = source_domain(&url)?;
    Some(WebSearchResult {
        title: sanitize_snippet(&raw.title),
        url,
        snippet: sanitize_snippet(&raw.snippet_html),
        source_domain,
    })
}

#[cfg(test)]
pub(crate) use execute_inner as execute_inner_for_tests;
