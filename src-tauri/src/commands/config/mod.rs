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
/// Re-exports for prompt-template management commands.
pub mod prompt_templates;
/// Re-exports for provider management commands.
pub mod providers;
mod state;
#[cfg(all(test, feature = "integration-test"))]
mod tests;
/// Re-exports for PDF theme selection commands.
pub mod pdf_theme;
/// Re-exports for global UI theme preference commands.
pub mod ui_theme;
/// Re-exports for per-tool credentials commands (web search today).
pub mod tool_credentials;
/// Re-exports for web-search cache management commands.
pub mod web_search_cache;
/// Re-exports for per-workspace web-search policy commands.
pub mod workspace_policy;

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
// Re-exports from prompt_templates module
pub use prompt_templates::{
    delete_prompt_template, list_prompt_templates, reset_prompt_template, save_prompt_template,
};
// Re-exports from providers module
pub use providers::{get_providers, remove_provider, save_provider};
// Re-exports from state module
pub use state::ConfigState;
// Re-exports from tool_credentials module
pub use tool_credentials::{
    delete_tool_credentials, get_tool_credentials, has_tool_credentials, set_tool_credentials,
    test_web_search_credentials, WebSearchCredentialsPayload,
};
// Re-exports from pdf_theme module
pub use pdf_theme::{get_pdf_theme, list_pdf_themes, set_pdf_theme};
// Re-exports from ui_theme module
pub use ui_theme::{get_ui_theme, set_ui_theme};
// Re-exports from web_search_cache module
pub use web_search_cache::{clear_web_search_cache, web_search_cache_stats, CacheStats};
// Re-exports from workspace_policy module
pub use workspace_policy::{
    delete_workspace_search_policy, get_workspace_search_policy, set_workspace_search_policy,
    WorkspacePolicyDto,
};
