//! Infrastructure adapters for LLM providers.
//!
//! This module hosts async provider clients, streaming helpers, and the provider
//! factory used by the application layer. Provider-agnostic entities remain in
//! `crate::core::llm`.

pub mod anthropic;
pub mod error;
pub mod openai;
pub mod streaming;

use futures::future::BoxFuture;
use futures::Stream;
use std::pin::Pin;

use crate::core::config::ApiType;

pub use crate::core::llm::entities::{
    ChatMessage as Message, ChatRequest, ChatResponse, ContentBlock, ImageSource, MessageContent,
    MessageRole, StreamChunk, StreamDelta, ToolCall, ToolDefinition, ToolResult, UsageStats,
};
pub use crate::core::llm::services::LlmEvent;
pub use error::{LlmError, LlmResult};
pub use streaming::{stream_to_events, stream_to_events_with_config, BatchingConfig, ChunkBatcher};

/// Async client contract for provider adapters.
pub trait LlmClient: Send + Sync {
    /// Gets the provider type for this client.
    fn provider_type(&self) -> ApiType;

    /// Sends a chat completion request and returns the response.
    fn chat<'a>(&'a self, request: ChatRequest) -> BoxFuture<'a, LlmResult<ChatResponse>>;

    /// Sends a streaming chat completion request.
    fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = LlmResult<StreamChunk>> + Send>>;

    /// Validates the API key by making a test request.
    fn validate_api_key(&self) -> BoxFuture<'_, LlmResult<bool>>;

    /// Fetches available models from the provider.
    fn fetch_models(&self) -> BoxFuture<'_, LlmResult<Vec<String>>>;
}

/// Creates an LLM client for the given provider type.
#[must_use]
pub fn create_client(
    api_type: ApiType,
    api_key: String,
    base_url: Option<String>,
) -> Box<dyn LlmClient> {
    match api_type {
        ApiType::OpenAI => Box::new(openai::OpenAiClient::new(api_key, base_url)),
        ApiType::Google => Box::new(openai::OpenAiClient::new_google(api_key)),
        ApiType::Anthropic => Box::new(anthropic::AnthropicClient::new(api_key, base_url)),
    }
}

#[cfg(test)]
#[path = "orchestrator_tests.rs"]
mod orchestrator_tests;
