//! Error types for LLM operations.
//!
//! This module provides comprehensive error handling for all LLM-related operations.

use thiserror::Error;

/// Errors from LLM operations.
#[derive(Debug, Error)]
pub enum LlmError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message from the API
        message: String,
    },

    /// Failed to parse response
    #[error("Failed to parse response: {0}")]
    Parse(String),

    /// API key not found or invalid
    #[error("API key error: {0}")]
    ApiKey(String),

    /// Provider not found
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Streaming error
    #[error("Streaming error: {0}")]
    Streaming(String),

    /// Context limit exceeded
    #[error("Context limit exceeded: {0}")]
    ContextLimit(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after {retry_after:?} seconds")]
    RateLimit {
        /// Seconds to wait before retrying
        retry_after: Option<u64>,
    },

    /// Timeout error
    #[error("Request timed out after {seconds} seconds")]
    Timeout {
        /// Seconds before timeout
        seconds: u64,
    },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl LlmError {
    /// Creates an API error from a status code and message.
    #[must_use]
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
        }
    }

    /// Creates a parse error.
    #[must_use]
    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse(message.into())
    }

    /// Creates an API key error.
    #[must_use]
    pub fn api_key(message: impl Into<String>) -> Self {
        Self::ApiKey(message.into())
    }

    /// Creates a configuration error.
    #[must_use]
    pub fn config(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    /// Creates a streaming error.
    #[must_use]
    pub fn streaming(message: impl Into<String>) -> Self {
        Self::Streaming(message.into())
    }

    /// Creates a rate limit error.
    #[must_use]
    pub fn rate_limit(retry_after: Option<u64>) -> Self {
        Self::RateLimit { retry_after }
    }

    /// Creates a timeout error.
    #[must_use]
    pub fn timeout(seconds: u64) -> Self {
        Self::Timeout { seconds }
    }

    /// Returns true if the error is retryable.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(e) => e.is_timeout() || e.is_connect(),
            Self::RateLimit { .. } => true,
            Self::Timeout { .. } => true,
            Self::Api { status, .. } => *status >= 500,
            _ => false,
        }
    }

    /// Returns true if the error is due to rate limiting.
    #[must_use]
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Self::RateLimit { .. })
    }

    /// Returns true if the error is due to authentication.
    #[must_use]
    pub fn is_auth_error(&self) -> bool {
        match self {
            Self::ApiKey(_) => true,
            Self::Api { status, .. } => *status == 401 || *status == 403,
            _ => false,
        }
    }
}

/// Result type for LLM operations.
pub type LlmResult<T> = Result<T, LlmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = LlmError::api(404, "Not found");
        assert!(err.to_string().contains("404"));
        assert!(err.to_string().contains("Not found"));
    }

    #[test]
    fn test_retryable_errors() {
        let err = LlmError::rate_limit(Some(60));
        assert!(err.is_retryable());
        assert!(err.is_rate_limit());

        let err = LlmError::api(500, "Internal server error");
        assert!(err.is_retryable());

        let err = LlmError::api(400, "Bad request");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_auth_errors() {
        let err = LlmError::api_key("Invalid key");
        assert!(err.is_auth_error());

        let err = LlmError::api(401, "Unauthorized");
        assert!(err.is_auth_error());

        let err = LlmError::api(403, "Forbidden");
        assert!(err.is_auth_error());

        let err = LlmError::api(404, "Not found");
        assert!(!err.is_auth_error());
    }
}
