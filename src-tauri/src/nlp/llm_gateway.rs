//! NLP-facing gateway for LLM completion calls.
//!
//! This module decouples NLP components from provider-specific clients.

use futures::future::BoxFuture;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::llm::{create_client, ChatRequest, Message};
use crate::nlp::config::{Model, OperationType, Provider};
use crate::storage::ConfigStore;

/// Minimal LLM completion interface required by NLP modules.
pub trait NlpLlmGateway: Send + Sync {
    /// Completes a prompt and returns model output text.
    fn complete<'a>(
        &'a self,
        system: &'a str,
        user: &'a str,
    ) -> BoxFuture<'a, Result<String, NlpLlmError>>;
}

/// Errors returned by the NLP LLM gateway.
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
    Llm(#[from] crate::llm::LlmError),
    /// Configuration storage access error.
    #[error("Configuration store error: {0}")]
    Store(String),
}

/// Config-backed NLP LLM gateway.
pub struct ConfigBackedNlpLlmGateway {
    /// Configuration storage used to resolve model/provider/key.
    config_store: Arc<Mutex<ConfigStore>>,
}

/// Resolved configuration required to execute a completion request.
#[derive(Debug, Clone)]
struct CompletionTarget {
    model: Model,
    provider: Provider,
    api_key: String,
}

impl ConfigBackedNlpLlmGateway {
    /// Creates a new gateway from a config database path.
    ///
    /// # Errors
    ///
    /// Returns an error if the config store cannot be initialized.
    pub fn new(db_path: &str) -> Result<Self, NlpLlmError> {
        let config_store = ConfigStore::new(db_path).map_err(|e| NlpLlmError::Store(e.to_string()))?;
        Ok(Self {
            config_store: Arc::new(Mutex::new(config_store)),
        })
    }

    /// Creates a gateway backed by an in-memory config store (tests only).
    ///
    /// # Errors
    ///
    /// Returns an error if the in-memory store cannot be initialized.
    pub fn in_memory() -> Result<Self, NlpLlmError> {
        let config_store =
            ConfigStore::in_memory().map_err(|e| NlpLlmError::Store(e.to_string()))?;
        Ok(Self {
            config_store: Arc::new(Mutex::new(config_store)),
        })
    }

    /// Creates a gateway from an existing shared config store.
    #[must_use]
    pub fn from_store(config_store: Arc<Mutex<ConfigStore>>) -> Self {
        Self { config_store }
    }

    async fn resolve_completion_target(&self) -> Result<CompletionTarget, NlpLlmError> {
        let store = self.config_store.lock().await;
        let model = resolve_code_operation_model(&store)?;

        let provider = store
            .get_provider(&model.provider_id)
            .map_err(|e| NlpLlmError::Store(e.to_string()))?
            .ok_or_else(|| {
                NlpLlmError::ProviderConfig(format!("Provider '{}' not found", model.provider_id))
            })?;

        let api_key = store
            .retrieve_api_key(&provider.id)
            .map_err(|e| NlpLlmError::ApiKey(e.to_string()))?;

        Ok(CompletionTarget {
            model,
            provider,
            api_key,
        })
    }
}

impl NlpLlmGateway for ConfigBackedNlpLlmGateway {
    fn complete<'a>(
        &'a self,
        system: &'a str,
        user: &'a str,
    ) -> BoxFuture<'a, Result<String, NlpLlmError>> {
        Box::pin(async move {
            let target = self.resolve_completion_target().await?;
            let client = create_client(
                target.provider.api_type,
                target.api_key,
                target.provider.base_url,
            );

            let request = ChatRequest::new(target.model.name, vec![Message::user(user)])
                .with_system(system);

            let response = client.chat(request).await?;
            Ok(response.content)
        })
    }
}

fn resolve_code_operation_model(store: &ConfigStore) -> Result<Model, NlpLlmError> {
    let defaults = store
        .get_operation_defaults()
        .map_err(|e| NlpLlmError::Store(e.to_string()))?;

    let default = defaults
        .into_iter()
        .find(|d| d.operation == OperationType::Code)
        .ok_or_else(|| {
            NlpLlmError::ModelConfig(
                "No model configured for 'code' operation. Configure a model first.".to_string(),
            )
        })?;

    let model_id = default.model_id.ok_or_else(|| {
        NlpLlmError::ModelConfig("Code operation has no configured model".to_string())
    })?;

    store
        .get_model(&model_id)
        .map_err(|e| NlpLlmError::Store(e.to_string()))?
        .ok_or_else(|| NlpLlmError::ModelConfig(format!("Model '{}' not found", model_id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::config::{ModelCapabilities, OperationType};

    async fn create_configured_gateway() -> ConfigBackedNlpLlmGateway {
        let gateway = ConfigBackedNlpLlmGateway::in_memory().expect("Should create gateway");
        let store = gateway.config_store.lock().await;

        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        store
            .save_models("openai", &[model])
            .expect("Should save models");
        store
            .set_operation_default(OperationType::Code, "openai-gpt-4")
            .expect("Should set operation default");
        store
            .store_api_key("openai", "test-key")
            .expect("Should store API key");
        drop(store);

        gateway
    }

    #[tokio::test]
    async fn resolves_completion_target_from_config() {
        let gateway = create_configured_gateway().await;
        let target = gateway
            .resolve_completion_target()
            .await
            .expect("Should resolve completion target");

        assert_eq!(target.model.id, "openai-gpt-4");
        assert_eq!(target.provider.id, "openai");
        assert_eq!(target.api_key, "test-key");
    }

    #[tokio::test]
    async fn missing_code_operation_returns_error() {
        let gateway = ConfigBackedNlpLlmGateway::in_memory().expect("Should create gateway");

        let error = gateway
            .resolve_completion_target()
            .await
            .expect_err("Should fail without operation default");

        assert!(matches!(error, NlpLlmError::ModelConfig(_)));
    }
}
