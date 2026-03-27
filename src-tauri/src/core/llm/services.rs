//! LLM service interface.
//!
//! Defines business operations for LLM functionality.

use super::entities::{ChatRequest, ChatResponse, LlmResult};
use super::repositories::LlmProvider;

/// Event emitted during LLM streaming.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmEvent {
    /// Streaming started
    Started { request_id: String, model: String },
    /// Content chunk received
    Chunk { request_id: String, delta: String },
    /// Streaming completed
    Complete {
        request_id: String,
        content: String,
        usage: Option<super::entities::UsageStats>,
        finish_reason: Option<String>,
    },
    /// Error occurred
    Error {
        request_id: String,
        error: String,
        retryable: bool,
    },
}

/// Options for streaming requests.
#[derive(Debug, Clone, Default)]
pub struct StreamingOptions {
    /// Unique request ID
    pub request_id: Option<String>,
    /// Batch interval in milliseconds
    pub batch_interval_ms: Option<u64>,
    /// Minimum content length to trigger immediate flush
    pub min_flush_length: Option<usize>,
}

/// LLM service interface.
///
/// Defines business logic operations for LLM functionality.
pub trait LlmService: Send + Sync {
    /// Sends a simple chat request (non-streaming).
    fn chat(&self, request: ChatRequest) -> LlmResult<ChatResponse>;

    /// Validates an API key for a provider.
    fn validate_api_key(&self, provider: LlmProvider, api_key: &str) -> LlmResult<bool>;

    /// Fetches available models for a provider.
    fn fetch_models(&self, provider: LlmProvider, api_key: &str) -> LlmResult<Vec<String>>;

    /// Gets the default model for a provider.
    fn get_default_model(&self, provider: LlmProvider) -> Option<String>;
}
