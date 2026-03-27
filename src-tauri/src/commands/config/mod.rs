//! Configuration Tauri IPC commands.
//!
//! Thin adapter layer for provider and model configuration management.

pub mod api_keys;
pub mod defaults;
mod helpers;
pub mod models;
pub mod providers;
mod state;
#[cfg(all(test, feature = "integration-test"))]
mod tests;

pub use api_keys::{
    delete_api_key, fetch_models, has_api_key, retrieve_api_key, store_api_key, test_connection,
    validate_api_key,
};
pub use defaults::{get_operation_defaults, set_operation_default};
pub use helpers::{
    get_code_llm_config, get_code_operation_model, CodeLlmConfigResponse, BUILTIN_PROVIDER_IDS,
};
pub use models::{
    add_manual_model, get_manual_models, get_models, remove_manual_model,
    set_provider_models_source,
};
pub use providers::{get_providers, remove_provider, save_provider};
pub use state::ConfigState;
