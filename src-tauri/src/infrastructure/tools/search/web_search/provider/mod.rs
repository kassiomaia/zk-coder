//! Provider abstraction for the web-search tool.
//!
//! The `SearchProvider` trait is the seam that keeps `WebSearchTool` free of
//! Google-specific code so we can swap backends (Bing, Brave, SearxNG) later
//! without touching the tool contract. Concrete providers live as sibling
//! modules (`google.rs` lands in Phase 4); this file defines only the trait
//! and the request/response DTOs that cross the seam.

use thiserror::Error;

/// Normalized request issued to a search provider.
///
/// Policy enforcement (allow/deny lists) and caching operate above this
/// layer — by the time a provider sees a `SearchRequest`, the arguments are
/// already validated and (if applicable) rewritten per workspace policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    /// Query string (already trimmed).
    pub query: String,
    /// Number of results requested; guaranteed in `[1, MAX_WEB_SEARCH_LIMIT]`.
    pub num_results: usize,
    /// Optional BCP-47 language tag (Google's `hl` parameter).
    pub language: Option<String>,
    /// Optional site restriction (Google's `siteSearch`).
    pub site: Option<String>,
}

/// Raw, provider-specific result before snippet sanitization.
///
/// The tool layer strips HTML, decodes entities, drops tracking parameters,
/// and derives `source_domain` before surfacing anything to the LLM or the
/// frontend. Keeping the provider honest about returning *raw* snippets makes
/// the sanitation boundary explicit and testable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawResult {
    /// Title as returned by the provider (may contain `<b>` highlight tags).
    pub title: String,
    /// Absolute URL as returned by the provider.
    pub url: String,
    /// Snippet as returned by the provider (HTML / entities not yet stripped).
    pub snippet_html: String,
}

/// Errors a `SearchProvider` implementation may return.
///
/// Each variant maps 1:1 to an `error_code` in the tool's structured error
/// body (see `spec.md §4.4`). Mapping is enforced by
/// `web_search/tests/error_mapping_tests.rs` in Phase 3.
#[derive(Debug, Error)]
pub enum SearchProviderError {
    /// Provider returned HTTP 429 (quota exhaustion).
    #[error("search provider rate-limited the request")]
    RateLimited,

    /// Provider returned HTTP 401 or 403 (credentials rejected).
    #[error("search provider rejected the credentials")]
    Unauthorized,

    /// Provider returned a 5xx or otherwise unexpected upstream status.
    #[error("search provider upstream error: {0}")]
    Upstream(String),

    /// The request could not complete (timeout, DNS, transport error).
    #[error("search provider network error: {0}")]
    Network(String),
}

/// Provider-agnostic search seam implemented by `GoogleProvider` (Phase 4)
/// and by `MockProvider` (this phase, test-only).
#[async_trait::async_trait]
pub trait SearchProvider: Send + Sync {
    /// Issue a search against the provider.
    async fn search(&self, req: SearchRequest) -> Result<Vec<RawResult>, SearchProviderError>;
}

#[cfg(test)]
pub mod mock;
