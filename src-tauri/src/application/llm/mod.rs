//! Provides the application layer for LLM (Large Language Model) services.
//!
//! This module contains the concrete implementation of the `LlmService` trait
//! from the core layer. It is responsible for coordinating between the domain
//! entities and the underlying infrastructure, such as clients for `OpenAI` and
//! Anthropic.

pub mod config;
pub mod service;

#[cfg(test)]
mod service_tests {
    include!("service_tests.rs");
}

pub use config::{get_code_llm_config, get_code_operation_model, CodeLlmConfigResponse};
pub use service::LlmServiceImpl;
