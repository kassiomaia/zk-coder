//! LLM repository interfaces.
//!
//! Defines provider interfaces for LLM operations.

use super::entities::{ChatRequest, ChatResponse, LlmResult};

/// LLM provider type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Google,
    Custom { name: String },
}

/// Trait for LLM provider clients.
///
/// Defines the common interface for all LLM providers.
pub trait LlmClient: Send + Sync {
    /// Gets the provider type for this client.
    fn provider_type(&self) -> LlmProvider;

    /// Sends a chat completion request.
    fn chat(&self, request: ChatRequest) -> LlmResult<ChatResponse>;

    /// Validates the API key.
    fn validate_api_key(&self) -> LlmResult<bool>;

    /// Fetches available models.
    fn fetch_models(&self) -> LlmResult<Vec<String>>;
}
