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

    /// Output truncated (hit max_tokens limit)
    #[error("Output truncated: {0}")]
    Truncated(String),

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
    pub const fn rate_limit(retry_after: Option<u64>) -> Self {
        Self::RateLimit { retry_after }
    }

    /// Creates a timeout error.
    #[must_use]
    pub const fn timeout(seconds: u64) -> Self {
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
            Self::Truncated(_) => true,
            _ => false,
        }
    }

    /// Returns true if the error is due to rate limiting.
    #[must_use]
    pub const fn is_rate_limit(&self) -> bool {
        matches!(self, Self::RateLimit { .. })
    }

    /// Returns true if the error is due to authentication.
    #[must_use]
    pub const fn is_auth_error(&self) -> bool {
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

    // =========================================================================
    // Error Creation Tests
    // =========================================================================

    #[test]
    fn test_error_creation() {
        let err = LlmError::api(404, "Not found");
        assert!(err.to_string().contains("404"));
        assert!(err.to_string().contains("Not found"));
    }

    #[test]
    fn test_all_error_variants_creation() {
        // API error
        let api_err = LlmError::api(500, "Server error");
        assert!(matches!(api_err, LlmError::Api { status: 500, .. }));

        // Parse error
        let parse_err = LlmError::parse("Invalid JSON");
        assert!(matches!(parse_err, LlmError::Parse(_)));

        // API key error
        let key_err = LlmError::api_key("Missing key");
        assert!(matches!(key_err, LlmError::ApiKey(_)));

        // Config error
        let config_err = LlmError::config("Invalid model");
        assert!(matches!(config_err, LlmError::Configuration(_)));

        // Truncated error
        let trunc_err = LlmError::Truncated("output cut off".to_string());
        assert!(matches!(trunc_err, LlmError::Truncated(_)));

        // Streaming error
        let stream_err = LlmError::streaming("Connection lost");
        assert!(matches!(stream_err, LlmError::Streaming(_)));

        // Rate limit error
        let rate_err = LlmError::rate_limit(Some(60));
        assert!(matches!(
            rate_err,
            LlmError::RateLimit {
                retry_after: Some(60)
            }
        ));

        // Timeout error
        let timeout_err = LlmError::timeout(30);
        assert!(matches!(timeout_err, LlmError::Timeout { seconds: 30 }));
    }

    // =========================================================================
    // Retry Logic Tests
    // =========================================================================

    #[test]
    fn test_retryable_errors() {
        // Rate limit is always retryable
        let err = LlmError::rate_limit(Some(60));
        assert!(err.is_retryable());
        assert!(err.is_rate_limit());

        let err = LlmError::rate_limit(None);
        assert!(err.is_retryable());

        // 5xx errors are retryable
        for status in [500, 502, 503, 504] {
            let err = LlmError::api(status, "Server error");
            assert!(err.is_retryable(), "Status {status} should be retryable");
        }

        // Timeout is retryable
        let err = LlmError::timeout(30);
        assert!(err.is_retryable());

        // 4xx errors (except 429) are not retryable
        for status in [400, 401, 403, 404, 422] {
            let err = LlmError::api(status, "Client error");
            assert!(
                !err.is_retryable(),
                "Status {status} should not be retryable"
            );
        }

        // Truncated errors are retryable
        let err = LlmError::Truncated("output cut off".to_string());
        assert!(err.is_retryable());

        // Parse errors are not retryable
        let err = LlmError::parse("Invalid JSON");
        assert!(!err.is_retryable());

        // Config errors are not retryable
        let err = LlmError::config("Invalid model");
        assert!(!err.is_retryable());

        // API key errors are not retryable
        let err = LlmError::api_key("Missing key");
        assert!(!err.is_retryable());

        // Streaming errors are not retryable
        let err = LlmError::streaming("Connection lost");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_rate_limit_detection() {
        let err = LlmError::rate_limit(Some(60));
        assert!(err.is_rate_limit());

        let err = LlmError::rate_limit(None);
        assert!(err.is_rate_limit());

        let err = LlmError::api(429, "Too many requests");
        assert!(!err.is_rate_limit()); // API errors don't count unless created via rate_limit()

        let err = LlmError::api(500, "Server error");
        assert!(!err.is_rate_limit());
    }

    // =========================================================================
    // Authentication Error Tests
    // =========================================================================

    #[test]
    fn test_auth_errors() {
        // API key errors are auth errors
        let err = LlmError::api_key("Invalid key");
        assert!(err.is_auth_error());

        // 401 is an auth error
        let err = LlmError::api(401, "Unauthorized");
        assert!(err.is_auth_error());

        // 403 is an auth error
        let err = LlmError::api(403, "Forbidden");
        assert!(err.is_auth_error());

        // Other status codes are not auth errors
        let err = LlmError::api(404, "Not found");
        assert!(!err.is_auth_error());

        let err = LlmError::api(500, "Server error");
        assert!(!err.is_auth_error());

        let err = LlmError::parse("Invalid JSON");
        assert!(!err.is_auth_error());
    }

    // =========================================================================
    // Error Display Tests
    // =========================================================================

    #[test]
    fn test_error_display_messages() {
        assert_eq!(
            LlmError::api(404, "Not found").to_string(),
            "API error (404): Not found"
        );

        assert_eq!(
            LlmError::parse("Invalid JSON").to_string(),
            "Failed to parse response: Invalid JSON"
        );

        assert_eq!(
            LlmError::api_key("Missing key").to_string(),
            "API key error: Missing key"
        );

        assert_eq!(
            LlmError::ProviderNotFound("openai".to_string()).to_string(),
            "Provider not found: openai"
        );

        assert_eq!(
            LlmError::ModelNotFound("gpt-5".to_string()).to_string(),
            "Model not found: gpt-5"
        );

        assert_eq!(
            LlmError::Configuration("Invalid model".to_string()).to_string(),
            "Configuration error: Invalid model"
        );

        assert_eq!(
            LlmError::Streaming("Connection lost".to_string()).to_string(),
            "Streaming error: Connection lost"
        );

        assert_eq!(
            LlmError::ContextLimit("Token limit exceeded".to_string()).to_string(),
            "Context limit exceeded: Token limit exceeded"
        );

        assert_eq!(
            LlmError::Internal("Unknown error".to_string()).to_string(),
            "Internal error: Unknown error"
        );

        assert_eq!(
            LlmError::Truncated("output cut off".to_string()).to_string(),
            "Output truncated: output cut off"
        );
    }

    #[test]
    fn test_rate_limit_display_with_retry_after() {
        let err = LlmError::rate_limit(Some(60));
        assert_eq!(
            err.to_string(),
            "Rate limit exceeded. Retry after Some(60) seconds"
        );

        let err = LlmError::rate_limit(None);
        assert_eq!(
            err.to_string(),
            "Rate limit exceeded. Retry after None seconds"
        );
    }

    #[test]
    fn test_timeout_display() {
        let err = LlmError::timeout(30);
        assert_eq!(err.to_string(), "Request timed out after 30 seconds");
    }

    // =========================================================================
    // Result Type Tests
    // =========================================================================

    #[test]
    fn test_llm_result_ok() {
        type LlmResult<T> = Result<T, LlmError>;

        let result: LlmResult<String> = Ok("Success".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
    }

    #[test]
    fn test_llm_result_err() {
        type LlmResult<T> = Result<T, LlmError>;

        let result: LlmResult<String> = Err(LlmError::api(500, "Server error"));
        assert!(result.is_err());
        assert!(matches!(result, Err(LlmError::Api { .. })));
    }
}
