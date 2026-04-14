//! Argument, result, and output types for the `search_web_search` tool.
//!
//! All validation is pure: no HTTP, no DB, no global state. This keeps Phase 1
//! tests fast and deterministic.

use serde::{Deserialize, Serialize};

use crate::infrastructure::tools::constants::{
    DEFAULT_WEB_SEARCH_LIMIT, MAX_WEB_SEARCH_LIMIT, MAX_WEB_SEARCH_QUERY_LEN,
};
use crate::infrastructure::tools::error::ToolError;

/// Arguments accepted by the `search_web_search` tool.
///
/// `deny_unknown_fields` mirrors the JSON-schema `additionalProperties: false`
/// surfaced to the LLM — unknown keys surface as `ToolError::InvalidArguments`
/// during `parse_args` rather than being silently dropped.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WebSearchArgs {
    /// Search query string.
    pub query: String,
    /// Number of results to return. Clamped to `[1, MAX_WEB_SEARCH_LIMIT]`.
    #[serde(default = "default_num_results")]
    pub num_results: usize,
    /// BCP-47 language tag used as Google's `hl` parameter.
    #[serde(default)]
    pub language: Option<String>,
    /// Optional site restriction (Google's `siteSearch`). Subject to the
    /// active workspace's domain policy — policy enforcement lands in Phase 5a.
    #[serde(default)]
    pub site: Option<String>,
}

const fn default_num_results() -> usize {
    DEFAULT_WEB_SEARCH_LIMIT
}

impl WebSearchArgs {
    /// Normalizes and validates user-supplied arguments.
    ///
    /// - Query is trimmed; empty queries are rejected.
    /// - Query length (characters, not bytes) is capped at
    ///   [`MAX_WEB_SEARCH_QUERY_LEN`]. Enforcing characters keeps the limit
    ///   stable across multibyte scripts.
    /// - `num_results` is clamped to `[1, MAX_WEB_SEARCH_LIMIT]`. Clamping
    ///   mirrors the spec's user-friendly intent: an LLM that asks for `0`
    ///   or `25` gets a usable value instead of an error.
    /// - Optional strings are trimmed; empty-after-trim becomes `None`.
    pub fn into_validated(mut self) -> Result<Self, ToolError> {
        let trimmed = self.query.trim();
        if trimmed.is_empty() {
            return Err(ToolError::invalid_args(
                "query must be non-empty after trim",
            ));
        }
        let char_count = trimmed.chars().count();
        if char_count > MAX_WEB_SEARCH_QUERY_LEN {
            return Err(ToolError::invalid_args(format!(
                "query length {char_count} exceeds maximum of {MAX_WEB_SEARCH_QUERY_LEN} characters"
            )));
        }
        self.query = trimmed.to_string();
        self.num_results = self.num_results.clamp(1, MAX_WEB_SEARCH_LIMIT);
        self.language = self.language.and_then(normalize_optional);
        self.site = self.site.and_then(normalize_optional);
        Ok(self)
    }
}

fn normalize_optional(s: String) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// A single normalized search result exposed to the LLM and the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebSearchResult {
    /// Result title (plain text, HTML stripped).
    pub title: String,
    /// Absolute http/https URL; tracking parameters are stripped by the tool.
    pub url: String,
    /// Result snippet (plain text, HTML stripped, entities decoded).
    pub snippet: String,
    /// Result's host, e.g. `svelte.dev`. Populated by the tool from `url`.
    pub source_domain: String,
}

/// Output returned to the LLM on a successful execution.
///
/// `filtered_by_policy` is only present when the per-workspace domain policy
/// dropped one or more results — Phase 5a lights up that branch. Until then
/// the field is omitted by default.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebSearchOutput {
    /// Echo of the normalized query string.
    pub query: String,
    /// Length of `results` after policy filtering.
    pub count: usize,
    /// The actual results.
    pub results: Vec<WebSearchResult>,
    /// Upstream provider identifier. Fixed to `google_custom_search` in MVP.
    pub provider: &'static str,
    /// `true` when served from the on-disk cache.
    pub cached: bool,
    /// `true` when workspace policy dropped one or more results. Omitted when
    /// no filtering occurred so the LLM / frontend don't branch unnecessarily.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtered_by_policy: Option<bool>,
}

/// Canonical provider identifier for the MVP.
pub const PROVIDER_GOOGLE_CUSTOM_SEARCH: &str = "google_custom_search";
