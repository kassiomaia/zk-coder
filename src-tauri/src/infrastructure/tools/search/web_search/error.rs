//! Internal failure type and stable error-body shape for the web-search tool.
//!
//! The LLM and the frontend both branch on `error_code`, so that value lives
//! in this file as the single source of truth (`spec.md §4.4`). Every failure
//! path inside `WebSearchTool` is represented as a [`WebSearchFailure`]
//! variant, and the conversion into a wire-format [`ErrorBody`] is pure so it
//! can be tested without any Tauri or network plumbing.

use serde::Serialize;

use crate::infrastructure::tools::constants::ENV_WEB_SEARCH_ENABLED;

use super::provider::SearchProviderError;

/// Machine-readable codes frozen by the spec. Adding a new code is a
/// backwards-compatible change; renaming or removing one is breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Credentials missing / malformed in the DB.
    NotConfigured,
    /// Feature flag is off at the tool layer.
    DisabledByFlag,
    /// Provider returned HTTP 429.
    RateLimited,
    /// Provider rejected credentials (HTTP 401/403).
    Unauthorized,
    /// Provider returned 5xx, timed out, or otherwise failed the network call.
    UpstreamError,
    /// Arguments did not match the input schema / validation rules.
    InvalidArguments,
    /// Workspace policy rejected the request (distinct from `InvalidArguments`).
    FilteredByPolicy,
}

impl ErrorCode {
    /// Returns the stable wire string for this code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotConfigured => "not_configured",
            Self::DisabledByFlag => "disabled_by_flag",
            Self::RateLimited => "rate_limited",
            Self::Unauthorized => "unauthorized",
            Self::UpstreamError => "upstream_error",
            Self::InvalidArguments => "invalid_arguments",
            Self::FilteredByPolicy => "filtered_by_policy",
        }
    }

    /// Whether a retry of the same request could plausibly succeed.
    /// Matches the `retryable` column in `spec.md §4.4`.
    #[must_use]
    pub const fn retryable(self) -> bool {
        matches!(self, Self::RateLimited | Self::UpstreamError)
    }
}

/// Stable JSON body emitted on every tool failure.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErrorBody {
    /// Human-readable, already-localized-where-possible message.
    pub error: String,
    /// Machine-readable stable code.
    pub error_code: ErrorCode,
    /// Retry advisory flag; derived from `error_code`.
    pub retryable: bool,
}

impl ErrorBody {
    /// Constructs an error body with the retry flag derived from `code`.
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            retryable: code.retryable(),
            error_code: code,
        }
    }
}

/// Internal failure enum used throughout `WebSearchTool::execute`. Every
/// variant has a 1:1 mapping to [`ErrorCode`] enforced in `tests/error_mapping_tests.rs`.
#[derive(Debug)]
pub enum WebSearchFailure {
    /// Feature flag off. Unreachable when routed through `ToolRegistry`, but
    /// kept as a defensive branch for tools called directly (e.g. in tests).
    #[allow(dead_code)]
    DisabledByFlag,
    /// Credentials row absent in `tool_credentials` or malformed JSON. The
    /// tool layer treats the two the same way — the user fix is identical.
    NotConfigured,
    /// Argument validation rejected the input.
    InvalidArguments(String),
    /// Policy rejected either the explicit `site` argument or the whole
    /// request shape before it reached the provider.
    FilteredByPolicy(String),
    /// Provider returned HTTP 429.
    RateLimited,
    /// Provider rejected credentials (HTTP 401/403).
    Unauthorized,
    /// Provider returned 5xx / network-level error. The contained message is
    /// for logs only; the user-facing text is generic.
    #[allow(dead_code)]
    UpstreamError(String),
}

impl WebSearchFailure {
    /// Maps the failure to its stable `ErrorCode`. Keep this function pure.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::DisabledByFlag => ErrorCode::DisabledByFlag,
            Self::NotConfigured => ErrorCode::NotConfigured,
            Self::InvalidArguments(_) => ErrorCode::InvalidArguments,
            Self::FilteredByPolicy(_) => ErrorCode::FilteredByPolicy,
            Self::RateLimited => ErrorCode::RateLimited,
            Self::Unauthorized => ErrorCode::Unauthorized,
            Self::UpstreamError(_) => ErrorCode::UpstreamError,
        }
    }

    /// Produces the human-readable message surfaced in the JSON body.
    #[must_use]
    pub fn human_message(&self) -> String {
        match self {
            Self::DisabledByFlag => {
                format!("Web search is disabled. Set {ENV_WEB_SEARCH_ENABLED}=1 to enable.")
            }
            Self::NotConfigured => {
                "Web search is not configured. Add a Google Custom Search API key and Engine ID \
                 to enable this tool."
                    .to_string()
            }
            Self::InvalidArguments(msg) | Self::FilteredByPolicy(msg) => msg.clone(),
            Self::RateLimited => {
                "Web search quota exceeded. Try again after the quota resets.".to_string()
            }
            Self::Unauthorized => {
                "Web search credentials were rejected by Google. Verify the API key and Engine ID."
                    .to_string()
            }
            Self::UpstreamError(_) => "Web search provider is temporarily unavailable.".to_string(),
        }
    }

    /// Consumes the failure and produces the wire-format body.
    #[must_use]
    pub fn into_body(self) -> ErrorBody {
        let code = self.code();
        ErrorBody::new(code, self.human_message())
    }
}

impl From<SearchProviderError> for WebSearchFailure {
    fn from(err: SearchProviderError) -> Self {
        match err {
            SearchProviderError::RateLimited => Self::RateLimited,
            SearchProviderError::Unauthorized => Self::Unauthorized,
            SearchProviderError::Upstream(msg) | SearchProviderError::Network(msg) => {
                Self::UpstreamError(msg)
            }
        }
    }
}
