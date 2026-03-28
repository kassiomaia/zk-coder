//! Defines the service interface for LLM-related business operations.
//!
//! This module provides the `LlmService` trait, which outlines the core
//! functionalities for interacting with LLMs, such as sending chat requests
//! and managing provider configurations.

use super::entities::{ChatRequest, ChatResponse, LlmResult};
use super::repositories::LlmProvider;

/// An event emitted during an LLM streaming operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmEvent {
    /// Indicates that the streaming has started.
    Started {
        /// The unique ID of the request.
        request_id: String,
        /// The model being used for the request.
        model: String,
    },
    /// A chunk of content has been received.
    Chunk {
        /// The unique ID of the request.
        request_id: String,
        /// The incremental change in the content.
        delta: String,
        /// Optional reasoning content from models that provide it.
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_delta: Option<String>,
    },
    /// Indicates that the streaming has completed.
    Complete {
        /// The unique ID of the request.
        request_id: String,
        /// The final, complete content of the response.
        content: String,
        /// Token usage statistics for the response.
        usage: Option<super::entities::UsageStats>,
        /// The reason why the model finished generating the response.
        finish_reason: Option<String>,
    },
    /// An error occurred during the streaming operation.
    Error {
        /// The unique ID of the request.
        request_id: String,
        /// The error message.
        error: String,
        /// A flag indicating whether the error is retryable.
        retryable: bool,
    },
}

/// Options for configuring LLM streaming requests.
#[derive(Debug, Clone, Default)]
pub struct StreamingOptions {
    /// A unique identifier for the request.
    pub request_id: Option<String>,
    /// The interval in milliseconds at which to batch chunks.
    pub batch_interval_ms: Option<u64>,
    /// The minimum content length to trigger an immediate flush of the batch.
    pub min_flush_length: Option<usize>,
}

/// The main service interface for LLM-related business logic.
///
/// This trait defines the high-level operations that can be performed with LLMs,
/// abstracting away the underlying implementation details.
pub trait LlmService: Send + Sync {
    /// Sends a simple, non-streaming chat request to the LLM.
    fn chat(&self, request: ChatRequest) -> LlmResult<ChatResponse>;

    /// Validates an API key for a given LLM provider.
    fn validate_api_key(&self, provider: LlmProvider, api_key: &str) -> LlmResult<bool>;

    /// Fetches the list of available models for a given LLM provider.
    fn fetch_models(&self, provider: LlmProvider, api_key: &str) -> LlmResult<Vec<String>>;

    /// Retrieves the default model for a given LLM provider.
    fn get_default_model(&self, provider: LlmProvider) -> Option<String>;
}
