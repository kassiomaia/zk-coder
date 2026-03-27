//! Application-facing gateway contract for NLP LLM completions.

use futures::future::BoxFuture;
use thiserror::Error;

/// Minimal LLM completion interface required by NLP workflows.
pub trait NlpLlmGateway: Send + Sync {
    /// Completes a prompt and returns model output text.
    fn complete<'a>(
        &'a self,
        system: &'a str,
        user: &'a str,
    ) -> BoxFuture<'a, Result<String, NlpLlmError>>;
}

/// Errors returned by the NLP LLM gateway contract.
#[derive(Debug, Error)]
pub enum NlpLlmError {
    /// Model configuration could not be resolved.
    #[error("Model configuration error: {0}")]
    ModelConfig(String),
    /// Provider configuration is missing or invalid.
    #[error("Provider configuration error: {0}")]
    ProviderConfig(String),
    /// API key is missing or invalid for the resolved provider.
    #[error("API key error: {0}")]
    ApiKey(String),
    /// Generic LLM layer failure.
    #[error("LLM error: {0}")]
    Llm(String),
    /// Configuration or repository access error.
    #[error("Configuration repository error: {0}")]
    Repository(String),
}
