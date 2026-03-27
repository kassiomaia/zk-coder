//! Application layer for LLM services.
//!
//! This module provides the concrete implementation of the LlmService trait
//! defined in the core layer. It coordinates between the domain entities and
pub mod config;
/// the infrastructure layer (OpenAI, Anthropic clients).
pub mod service;

#[cfg(test)]
mod service_tests {
    include!("service_tests.rs");
}

pub use config::{get_code_llm_config, get_code_operation_model, CodeLlmConfigResponse};
pub use service::LlmServiceImpl;
