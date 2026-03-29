//! Configuration Tauri IPC commands.
//!
//! Thin adapter layer for provider and model configuration management.

/// Re-exports for API key management commands.
pub mod api_keys;
/// Re-exports for default configuration commands.
pub mod defaults;
mod helpers;
/// Re-exports for model management commands.
pub mod models;
/// Re-exports for provider management commands.
pub mod providers;
mod state;
#[cfg(all(test, feature = "integration-test"))]
mod tests;

// Re-exports from api_keys module
pub use api_keys::{
    delete_api_key, fetch_models, has_api_key, retrieve_api_key, store_api_key, test_connection,
    validate_api_key,
};
// Re-exports from defaults module
pub use defaults::{get_operation_defaults, set_operation_default};
// Re-exports from helpers module
pub use helpers::{
    get_code_llm_config, get_code_operation_model, CodeLlmConfigResponse, BUILTIN_PROVIDER_IDS,
};
// Re-exports from models module
pub use models::{
    add_manual_model, get_manual_models, get_models, remove_manual_model,
    set_provider_models_source,
};
// Re-exports from providers module
pub use providers::{get_providers, remove_provider, save_provider};
// Re-exports from state module
pub use state::ConfigState;
