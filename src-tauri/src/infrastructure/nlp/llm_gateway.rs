//! NLP-facing gateway for LLM completion calls.
//!
//! This module decouples NLP components from provider-specific clients.

use futures::future::BoxFuture;
use log::error;
use std::sync::Arc;

use crate::application::nlp::gateway::{NlpLlmError, NlpLlmGateway};
use crate::core::config::{ConfigRepository, Model, OperationType, Provider};
use crate::core::llm::entities::{ChatMessage, ChatRequest};
use crate::infrastructure::llm::create_client_for_provider;
use crate::infrastructure::sqlite::repositories::config::SqliteConfigRepository;

/// Config-backed NLP LLM gateway.
pub struct ConfigBackedNlpLlmGateway {
    /// Configuration storage used to resolve model/provider/key.
    config_repository: Arc<dyn ConfigRepository>,
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
        let config_repository = Arc::new(
            SqliteConfigRepository::new(db_path)
                .map_err(|e| NlpLlmError::Repository(e.to_string()))?,
        ) as Arc<dyn ConfigRepository>;
        Ok(Self { config_repository })
    }

    /// Creates a gateway backed by an in-memory config store (tests only).
    ///
    /// # Errors
    ///
    /// Returns an error if the in-memory store cannot be initialized.
    pub fn in_memory() -> Result<Self, NlpLlmError> {
        let config_repository = Arc::new(
            SqliteConfigRepository::in_memory()
                .map_err(|e| NlpLlmError::Repository(e.to_string()))?,
        ) as Arc<dyn ConfigRepository>;
        Ok(Self { config_repository })
    }

    /// Creates a gateway from an existing shared configuration repository.
    #[must_use]
    pub fn from_repository(config_repository: Arc<dyn ConfigRepository>) -> Self {
        Self { config_repository }
    }

    async fn resolve_completion_target(&self) -> Result<CompletionTarget, NlpLlmError> {
        let model = resolve_nlp_operation_model(self.config_repository.as_ref()).map_err(|e| {
            error!("[NLP LLM Gateway] Failed to resolve NLP model: {e}");
            e
        })?;

        let provider = self
            .config_repository
            .get_provider(&model.provider_id)
            .map_err(|e| {
                error!(
                    "[NLP LLM Gateway] Failed to get provider '{}': {}",
                    model.provider_id, e
                );
                NlpLlmError::Repository(e.to_string())
            })?
            .ok_or_else(|| {
                let err = NlpLlmError::ProviderConfig(format!(
                    "Provider '{}' not found",
                    model.provider_id
                ));
                error!("[NLP LLM Gateway] {err}");
                err
            })?;

        let api_key = self
            .config_repository
            .retrieve_api_key(&provider.id)
            .map_err(|e| {
                error!(
                    "[NLP LLM Gateway] Failed to retrieve API key for provider '{}': {}",
                    provider.id, e
                );
                NlpLlmError::ApiKey(e.to_string())
            })?;

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
            let model_name = target.model.name.clone();
            let provider_type = target.provider.api_type;
            let client = create_client_for_provider(&target.provider, target.api_key);

            let request = ChatRequest::new(model_name.clone(), vec![ChatMessage::user(user)])
                .with_system(system);

            let response = client.chat(request).await.map_err(|error| {
                error!(
                    "[NLP LLM Gateway] LLM call failed: provider={provider_type:?}, model={model_name}, error={error}"
                );
                NlpLlmError::Llm(error.to_string())
            })?;
            Ok(response.content)
        })
    }
}

fn resolve_nlp_operation_model(repository: &dyn ConfigRepository) -> Result<Model, NlpLlmError> {
    let defaults = repository.get_operation_defaults().map_err(|e| {
        error!("[NLP LLM Gateway] Failed to get operation defaults: {e}");
        NlpLlmError::Repository(e.to_string())
    })?;

    let analysis_default = defaults
        .into_iter()
        .find(|d| d.operation == OperationType::Analysis);

    if let Some(default) = analysis_default {
        let model_id = default.model_id.ok_or_else(|| {
            let err =
                NlpLlmError::ModelConfig("Analysis operation has no configured model".to_string());
            error!("[NLP LLM Gateway] {err}");
            err
        })?;
        return repository
            .get_model(&model_id)
            .map_err(|e| {
                error!("[NLP LLM Gateway] Failed to get model '{model_id}': {e}");
                NlpLlmError::Repository(e.to_string())
            })?
            .ok_or_else(|| {
                let err = NlpLlmError::ModelConfig(format!("Model '{model_id}' not found"));
                error!("[NLP LLM Gateway] {err}");
                err
            });
    }

    let defaults = repository.get_operation_defaults().map_err(|e| {
        error!("[NLP LLM Gateway] Failed to get operation defaults: {e}");
        NlpLlmError::Repository(e.to_string())
    })?;
    let code_default = defaults
        .into_iter()
        .find(|d| d.operation == OperationType::Code)
        .ok_or_else(|| {
            let err = NlpLlmError::ModelConfig(
                "No model configured for NLP operations. Configure an 'analysis' or 'code' default model."
                    .to_string(),
            );
            error!("[NLP LLM Gateway] {err}");
            err
        })?;

    let model_id = code_default.model_id.ok_or_else(|| {
        let err = NlpLlmError::ModelConfig("Code operation has no configured model".to_string());
        error!("[NLP LLM Gateway] {err}");
        err
    })?;

    repository
        .get_model(&model_id)
        .map_err(|e| {
            error!("[NLP LLM Gateway] Failed to get model '{model_id}': {e}");
            NlpLlmError::Repository(e.to_string())
        })?
        .ok_or_else(|| {
            let err = NlpLlmError::ModelConfig(format!("Model '{model_id}' not found"));
            error!("[NLP LLM Gateway] {err}");
            err
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{ModelCapabilities, OperationType};

    async fn create_configured_gateway() -> ConfigBackedNlpLlmGateway {
        let gateway = ConfigBackedNlpLlmGateway::in_memory().expect("Should create gateway");
        let repository = gateway.config_repository.clone();

        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        repository
            .save_models("openai", &[model])
            .expect("Should save models");
        repository
            .set_operation_default(OperationType::Code, "openai-gpt-4")
            .expect("Should set operation default");
        repository
            .store_api_key("openai", "test-key")
            .expect("Should store API key");

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

    #[tokio::test]
    async fn prefers_analysis_operation_when_available() {
        let gateway = ConfigBackedNlpLlmGateway::in_memory().expect("Should create gateway");
        let repository = gateway.config_repository.clone();

        let analysis_model = Model::new("openai", "gpt-4.1", ModelCapabilities::default());
        let code_model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        repository
            .save_models("openai", &[analysis_model, code_model])
            .expect("Should save models");
        repository
            .set_operation_default(OperationType::Analysis, "openai-gpt-4-1")
            .expect("Should set analysis default");
        repository
            .set_operation_default(OperationType::Code, "openai-gpt-4")
            .expect("Should set code default");
        repository
            .store_api_key("openai", "test-key")
            .expect("Should store API key");

        let target = gateway
            .resolve_completion_target()
            .await
            .expect("Should resolve target");
        assert_eq!(target.model.id, "openai-gpt-4-1");
    }

    #[tokio::test]
    async fn falls_back_to_code_when_analysis_is_not_configured() {
        let gateway = create_configured_gateway().await;
        let target = gateway
            .resolve_completion_target()
            .await
            .expect("Should resolve target");
        assert_eq!(target.model.id, "openai-gpt-4");
    }

    #[tokio::test]
    async fn missing_api_key_returns_error() {
        let gateway = ConfigBackedNlpLlmGateway::in_memory().expect("Should create gateway");
        let repository = gateway.config_repository.clone();

        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        repository
            .save_models("openai", &[model])
            .expect("Should save models");
        repository
            .set_operation_default(OperationType::Code, "openai-gpt-4")
            .expect("Should set code default");

        let error = gateway
            .resolve_completion_target()
            .await
            .expect_err("Should fail without API key");
        assert!(matches!(error, NlpLlmError::ApiKey(_)));
    }
}
