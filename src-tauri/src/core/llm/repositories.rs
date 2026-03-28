//! Defines the repository interfaces for LLM providers.
//!
//! This module provides the traits that abstract the interactions with
//! different LLM providers, defining a common contract for operations
//! such as chat completions and model fetching.

use super::entities::{ChatRequest, ChatResponse, LlmResult};

/// Represents the type of an LLM provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmProvider {
    /// The OpenAI provider.
    OpenAI,
    /// The Anthropic provider.
    Anthropic,
    /// The Google AI (Gemini) provider.
    Google,
    /// A custom, user-defined provider.
    Custom {
        /// The name of the custom provider.
        name: String,
    },
}

/// A trait for LLM provider clients, defining a common interface for all providers.
pub trait LlmClient: Send + Sync {
    /// Returns the `LlmProvider` type for this client.
    fn provider_type(&self) -> LlmProvider;

    /// Sends a chat completion request to the provider.
    ///
    /// # Arguments
    ///
    /// * `request` - A `ChatRequest` containing the details of the request.
    ///
    /// # Returns
    ///
    /// An `LlmResult` containing a `ChatResponse` on success, or an `LlmError` on failure.
    fn chat(&self, request: ChatRequest) -> LlmResult<ChatResponse>;

    /// Validates the API key for the provider.
    ///
    /// # Returns
    ///
    /// An `LlmResult` containing `true` if the key is valid, or an `LlmError` on failure.
    fn validate_api_key(&self) -> LlmResult<bool>;

    /// Fetches the list of available models from the provider.
    ///
    /// # Returns
    ///
    /// An `LlmResult` containing a vector of model names on success, or an `LlmError` on failure.
    fn fetch_models(&self) -> LlmResult<Vec<String>>;
}
