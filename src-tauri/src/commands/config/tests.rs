use crate::core::config::{
    ApiType, Model, ModelCapabilities, ModelsSource, OperationType, Provider, SaveProviderRequest,
};

pub use super::*;

mod tests {
    use super::*;
    use crate::core::config::ManualModelRequest;

    #[tokio::test]
    async fn test_config_state_creation() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;
        let providers = store.get_providers().expect("Failed to get providers");
        assert_eq!(providers.len(), 3); // Built-in providers
    }

    #[tokio::test]
    async fn test_store_operations() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Test save provider
        let provider = Provider::custom("Test Provider", "TP", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");

        // Test get providers
        let providers = store.get_providers().expect("Failed to get providers");
        assert!(providers.iter().any(|p| p.id == provider.id));

        // Test delete provider
        store
            .delete_provider(&provider.id)
            .expect("Failed to delete provider");
    }

    #[tokio::test]
    async fn test_builtin_providers_not_custom() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;
        let providers = store.get_providers().expect("Failed to get providers");

        // Verify all built-in providers have is_custom = false
        for provider in providers {
            assert!(
                !provider.is_custom,
                "Built-in provider {} should have is_custom = false",
                provider.id
            );
        }
    }

    #[tokio::test]
    async fn test_cannot_delete_builtin_providers() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Try to delete built-in providers - should fail
        for builtin_id in BUILTIN_PROVIDER_IDS {
            let result = store.delete_provider(builtin_id);
            assert!(
                result.is_err(),
                "Should not be able to delete built-in provider {}",
                builtin_id
            );
        }
    }

    // =========================================================================
    // Phase 1 Tests: Configuration Query
    // =========================================================================

    #[tokio::test]
    async fn test_get_code_operation_model_success() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Setup: Create a model for OpenAI
        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        store
            .save_models("openai", &[model.clone()])
            .expect("Failed to save model");

        // Set the code operation default
        store
            .set_operation_default(OperationType::Code, "openai-gpt-4")
            .expect("Failed to set operation default");

        // Test: Get the code operation model
        let result = get_code_operation_model(store.as_ref())
            .expect("Failed to get code operation model");

        assert_eq!(result.name, "gpt-4");
        assert_eq!(result.provider_id, "openai");
        assert_eq!(result.id, "openai-gpt-4");
    }

    #[tokio::test]
    async fn test_get_code_operation_model_not_configured() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Test: No code operation configured
        let result = get_code_operation_model(store.as_ref());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("No model configured for 'code' operation"));
    }

    #[tokio::test]
    async fn test_get_code_llm_config_full() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Setup: Create a model for OpenAI
        let model = Model::new("openai", "gpt-4-turbo", ModelCapabilities::default());
        store
            .save_models("openai", &[model.clone()])
            .expect("Failed to save model");

        // Set the code operation default
        store
            .set_operation_default(OperationType::Code, "openai-gpt-4-turbo")
            .expect("Failed to set operation default");

        // Test: Get full LLM config
        let result =
            get_code_llm_config(store.as_ref()).expect("Failed to get code LLM config");

        assert_eq!(result.name, "gpt-4-turbo");
        assert_eq!(result.provider_id, "openai");
        assert_eq!(result.provider_name, "OpenAI");
        assert_eq!(result.api_type, ApiType::OpenAI);
        assert_eq!(result.base_url, None);
    }

    // =========================================================================
    // Command Tests (async)
    // =========================================================================

    #[tokio::test]
    async fn test_get_providers_command() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let providers = get_providers(crate::test_support::state(&state))
            .await
            .expect("Failed to get providers");
        assert!(!providers.is_empty());
        assert_eq!(providers.len(), 3); // Built-in providers
    }

    #[tokio::test]
    async fn test_save_provider_new_custom() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = SaveProviderRequest {
            id: None,
            name: "Custom Provider".to_string(),
            icon: "CP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: Some("https://custom.example.com".to_string()),
        };
        let result = save_provider(request, crate::test_support::state(&state))
            .await
            .expect("Failed to save provider");
        assert_eq!(result.name, "Custom Provider");
        assert!(result.is_custom);
    }

    #[tokio::test]
    async fn test_save_provider_with_builtin_id_fails() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = SaveProviderRequest {
            id: Some("anthropic".to_string()),
            name: "Malicious".to_string(),
            icon: "M".to_string(),
            api_type: ApiType::Anthropic,
            base_url: None,
        };
        let result = save_provider(request, crate::test_support::state(&state)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("reserved ID"));
    }

    #[tokio::test]
    async fn test_save_provider_update_existing() {
        let state = ConfigState::in_memory().expect("Failed to create state");

        // First create a custom provider
        let create_request = SaveProviderRequest {
            id: None,
            name: "Test Provider".to_string(),
            icon: "TP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: None,
        };
        let created = save_provider(create_request, crate::test_support::state(&state))
            .await
            .expect("Failed to create provider");

        // Update it
        let update_request = SaveProviderRequest {
            id: Some(created.id.clone()),
            name: "Updated Provider".to_string(),
            icon: "UP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: Some("https://updated.com".to_string()),
        };
        let result = save_provider(update_request, crate::test_support::state(&state))
            .await
            .expect("Failed to update provider");
        assert_eq!(result.name, "Updated Provider");
        assert_eq!(result.base_url, Some("https://updated.com".to_string()));
    }

    #[tokio::test]
    async fn test_remove_provider_custom() {
        let state = ConfigState::in_memory().expect("Failed to create state");

        // Create a custom provider
        let request = SaveProviderRequest {
            id: None,
            name: "To Delete".to_string(),
            icon: "TD".to_string(),
            api_type: ApiType::OpenAI,
            base_url: None,
        };
        let created = save_provider(request, crate::test_support::state(&state))
            .await
            .expect("Failed to create provider");

        // Delete it
        let result = remove_provider(created.id.clone(), crate::test_support::state(&state)).await;
        assert!(result.is_ok());

        // Verify it's gone
        let providers = get_providers(crate::test_support::state(&state))
            .await
            .expect("Failed to get providers");
        assert!(!providers.iter().any(|p| p.id == created.id));
    }

    #[tokio::test]
    async fn test_validate_api_key_empty() {
        let result = validate_api_key("test".to_string(), "".to_string(), ApiType::OpenAI, None)
            .await
            .expect("Failed to validate");
        assert!(!result.valid);
        assert!(result.message.contains("enter an API key"));
    }

    #[tokio::test]
    async fn test_validate_api_key_whitespace_only() {
        let result = validate_api_key("test".to_string(), "   ".to_string(), ApiType::OpenAI, None)
            .await
            .expect("Failed to validate");
        assert!(!result.valid);
    }

    #[tokio::test]
    async fn test_test_connection_empty_api_key() {
        let result = test_connection(
            "https://api.example.com".to_string(),
            "".to_string(),
            ApiType::OpenAI,
        )
        .await
        .expect("Failed to test connection");
        assert!(!result.valid);
    }

    #[tokio::test]
    async fn test_store_api_key() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = store_api_key(
            "openai".to_string(),
            "sk-test-key-123".to_string(),
            crate::test_support::state(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retrieve_api_key() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let provider_id = "openai";
        let api_key = "sk-test-key-456";

        // Persist the key
        store_api_key(
            provider_id.to_string(),
            api_key.to_string(),
            crate::test_support::state(&state),
        )
        .await
        .expect("Failed to store API key");

        // Retrieve it
        let result = retrieve_api_key(provider_id.to_string(), crate::test_support::state(&state))
            .await
            .expect("Failed to retrieve API key");
        assert_eq!(result, api_key);
    }

    #[tokio::test]
    async fn test_has_api_key_true() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let provider_id = "openai";

        // Persist a key
        store_api_key(
            provider_id.to_string(),
            "sk-test".to_string(),
            crate::test_support::state(&state),
        )
        .await
        .expect("Failed to store API key");

        // Check if exists
        let result = has_api_key(provider_id.to_string(), crate::test_support::state(&state))
            .await
            .expect("Failed to check API key");
        assert!(result);
    }

    #[tokio::test]
    async fn test_has_api_key_false() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = has_api_key(
            "nonexistent-provider".to_string(),
            crate::test_support::state(&state),
        )
        .await
        .expect("Failed to check API key");
        assert!(!result);
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let provider_id = "openai";

        // Persist a key
        store_api_key(
            provider_id.to_string(),
            "sk-test".to_string(),
            crate::test_support::state(&state),
        )
        .await
        .expect("Failed to store API key");

        // Delete it
        delete_api_key(provider_id.to_string(), crate::test_support::state(&state))
            .await
            .expect("Failed to delete API key");

        // Verify it's gone
        let result = has_api_key(provider_id.to_string(), crate::test_support::state(&state))
            .await
            .expect("Failed to check API key");
        assert!(!result);
    }

    #[tokio::test]
    async fn test_get_operation_defaults() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = get_operation_defaults(crate::test_support::state(&state))
            .await
            .expect("Failed to get operation defaults");
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_set_operation_default_success() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Create a model first
        let model = Model::new(
            "openai",
            "gpt-4",
            ModelCapabilities {
                analysis: true,
                code: true,
                vision: true,
                audio: true,
                tips: true,
                review: true,
            },
        );
        store
            .save_models("openai", &[model])
            .expect("Failed to save model");
        drop(store);

        // Set operation default
        let result = set_operation_default(
            OperationType::Code,
            "openai-gpt-4".to_string(),
            crate::test_support::state(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_operation_default_model_not_found() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = set_operation_default(
            OperationType::Code,
            "nonexistent-model".to_string(),
            crate::test_support::state(&state),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_models() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Create a model
        let model = Model::new(
            "openai",
            "gpt-4",
            ModelCapabilities {
                analysis: true,
                code: true,
                vision: true,
                audio: true,
                tips: true,
                review: true,
            },
        );
        store
            .save_models("openai", &[model])
            .expect("Failed to save model");
        drop(store);

        // Get models
        let result = get_models("openai".to_string(), crate::test_support::state(&state))
            .await
            .expect("Failed to get models");
        assert!(!result.is_empty());
        assert_eq!(result[0].name, "gpt-4");
    }

    #[tokio::test]
    async fn test_add_manual_model() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Create a custom provider
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");
        drop(store);

        // Add manual model
        let request = ManualModelRequest {
            provider_id: provider.id.clone(),
            name: "custom-model".to_string(),
            capabilities: None,
        };
        let result = add_manual_model(request, crate::test_support::state(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_manual_model_empty_name() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = ManualModelRequest {
            provider_id: "test".to_string(),
            name: "".to_string(),
            capabilities: None,
        };
        let result = add_manual_model(request, crate::test_support::state(&state)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn test_remove_manual_model() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Create a custom provider
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");
        drop(store);

        // Add manual model
        let request = ManualModelRequest {
            provider_id: provider.id.clone(),
            name: "custom-model".to_string(),
            capabilities: None,
        };
        add_manual_model(request, crate::test_support::state(&state))
            .await
            .expect("Failed to add manual model");

        // Remove it
        let result = remove_manual_model(
            provider.id.clone(),
            "custom-model".to_string(),
            crate::test_support::state(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_manual_models() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Create a custom provider
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");
        drop(store);

        // Add manual model
        let request = ManualModelRequest {
            provider_id: provider.id.clone(),
            name: "custom-model".to_string(),
            capabilities: None,
        };
        add_manual_model(request, crate::test_support::state(&state))
            .await
            .expect("Failed to add manual model");

        // Get manual models
        let result = get_manual_models(provider.id.clone(), crate::test_support::state(&state))
            .await
            .expect("Failed to get manual models");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "custom-model");
    }

    #[tokio::test]
    async fn test_set_provider_models_source() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        // Create a custom provider
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");
        drop(store);

        // Set to manual
        let result = set_provider_models_source(
            provider.id.clone(),
            ModelsSource::Manual,
            crate::test_support::state(&state),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_models_empty_api_key() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = fetch_models(
            "test".to_string(),
            "".to_string(),
            ApiType::OpenAI,
            None,
            crate::test_support::state(&state),
        )
        .await
        .expect("Failed to fetch models");
        assert!(!result.success);
    }

    // =========================================================================
    // ModelCapabilities Tests
    // =========================================================================

    #[test]
    fn test_model_capabilities_default() {
        let caps = ModelCapabilities::default();
        assert!(!caps.analysis);
        assert!(!caps.code);
        assert!(!caps.vision);
        assert!(!caps.audio);
        assert!(!caps.tips);
        assert!(!caps.review);
    }

    #[test]
    fn test_model_capabilities_all() {
        let caps = ModelCapabilities {
            analysis: true,
            code: true,
            vision: true,
            audio: true,
            tips: true,
            review: true,
        };
        assert!(caps.analysis);
        assert!(caps.code);
        assert!(caps.vision);
        assert!(caps.audio);
        assert!(caps.tips);
        assert!(caps.review);
    }

    #[test]
    fn test_model_capabilities_infer_from_name_gpt4() {
        let caps = ModelCapabilities::infer_from_name("gpt-4");
        assert!(caps.code);
        assert!(caps.analysis);
        assert!(caps.review);
    }

    #[test]
    fn test_model_capabilities_infer_from_name_vision() {
        let caps = ModelCapabilities::infer_from_name("gpt-4-vision");
        assert!(caps.vision);
        assert!(caps.code);
    }

    #[test]
    fn test_model_capabilities_infer_from_name_audio() {
        let caps = ModelCapabilities::infer_from_name("whisper-1");
        assert!(caps.audio);
    }

    // =========================================================================
    // Provider Tests
    // =========================================================================

    #[test]
    fn test_provider_custom() {
        let provider = Provider::custom("Test", "T", ApiType::OpenAI, None);
        assert!(provider.is_custom);
        assert_eq!(provider.api_type, ApiType::OpenAI);
        assert!(provider.base_url.is_none());
    }

    #[test]
    fn test_provider_custom_with_base_url() {
        let provider = Provider::custom(
            "Test",
            "T",
            ApiType::OpenAI,
            Some("https://custom.com".to_string()),
        );
        assert_eq!(provider.base_url, Some("https://custom.com".to_string()));
    }

    // =========================================================================
    // OperationType Tests
    // =========================================================================

    #[test]
    fn test_operation_type_display() {
        assert_eq!(OperationType::Analysis.to_string(), "analysis");
        assert_eq!(OperationType::Code.to_string(), "code");
        assert_eq!(OperationType::Vision.to_string(), "vision");
        assert_eq!(OperationType::Audio.to_string(), "audio");
        assert_eq!(OperationType::Tips.to_string(), "tips");
        assert_eq!(OperationType::Review.to_string(), "review");
    }

    // =========================================================================
    // Input Validation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_provider_name_with_special_characters() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = SaveProviderRequest {
            id: None,
            name: "Test <Provider> & More!".to_string(),
            icon: "TP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: None,
        };
        let result = save_provider(request, crate::test_support::state(&state))
            .await
            .expect("Failed to save provider");
        assert_eq!(result.name, "Test <Provider> & More!");
    }

    #[tokio::test]
    async fn test_very_long_provider_name() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let long_name = "A".repeat(500);
        let request = SaveProviderRequest {
            id: None,
            name: long_name.clone(),
            icon: "TP".to_string(),
            api_type: ApiType::OpenAI,
            base_url: None,
        };
        let result = save_provider(request, crate::test_support::state(&state))
            .await
            .expect("Failed to save provider");
        assert_eq!(result.name, long_name);
    }

    #[tokio::test]
    async fn test_invalid_base_url_format() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let request = SaveProviderRequest {
            id: None,
            name: "Test".to_string(),
            icon: "T".to_string(),
            api_type: ApiType::OpenAI,
            base_url: Some("not-a-url".to_string()),
        };
        // Command accepts any string; validation happens at API call time
        let result = save_provider(request, crate::test_support::state(&state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_key_with_unicode() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let result = store_api_key(
            "openai".to_string(),
            "sk-test-🔑-key".to_string(),
            crate::test_support::state(&state),
        )
        .await;
        // Unicode in API keys should be accepted
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_model_id_with_dash() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        let provider = Provider::custom("Test Provider", "TP", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");

        let model = Model::new(
            &provider.id,
            "model-with-dashes",
            ModelCapabilities {
                analysis: true,
                code: true,
                vision: true,
                audio: true,
                tips: true,
                review: true,
            },
        );
        store
            .save_models("test-provider", &[model])
            .expect("Failed to save model");
        drop(store);

        let result = get_models(
            provider.id.clone(),
            crate::test_support::state(&state),
        )
        .await
        .expect("Failed to get models");
        assert_eq!(result[0].name, "model-with-dashes");
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_nonexistent_provider() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;
        let result = store.get_provider("nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_builtin_provider_fails() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;
        let result = store.delete_provider("anthropic");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_operation_default_unsupported_capability() {
        let state = ConfigState::in_memory().expect("Failed to create state");
        let store = &state.repository;

        let provider = Provider::custom("Test Provider", "TP", ApiType::OpenAI, None);
        store
            .save_provider(&provider)
            .expect("Failed to save provider");

        // Create a model without vision capability
        let mut caps = ModelCapabilities::default();
        caps.code = true;
        let model = Model::new(&provider.id, "model", caps);
        store
            .save_models(&provider.id, &[model])
            .expect("Failed to save model");
        drop(store);

        // Try to set as vision default
        let result = set_operation_default(
            OperationType::Vision,
            format!("{}-model", provider.id),
            crate::test_support::state(&state),
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not support"));
    }

    #[tokio::test]
    async fn test_models_source_variants() {
        // Test that all ModelsSource variants can be created
        let _manual = ModelsSource::Manual;
        let _api = ModelsSource::Auto;
        let _hybrid = ModelsSource::Hybrid;
    }

    #[tokio::test]
    async fn test_api_type_variants() {
        // Test that all ApiType variants work
        let _anthropic = ApiType::Anthropic;
        let _openai = ApiType::OpenAI;
        let _google = ApiType::Google;
    }
}
