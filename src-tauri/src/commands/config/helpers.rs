pub use crate::application::llm::config::{
    get_code_llm_config, get_code_operation_model, CodeLlmConfigResponse,
};

/// Built-in provider IDs that cannot be used for custom providers.
pub const BUILTIN_PROVIDER_IDS: &[&str] = &["anthropic", "openai", "google"];
