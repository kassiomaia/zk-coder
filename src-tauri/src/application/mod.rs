//! Application layer for the Coder application.
//!
//! This module provides concrete implementations of service traits defined
//! in the core layer. It coordinates between domain entities and infrastructure
//! implementations.

pub mod agent;
pub mod chat;
pub mod llm;
pub mod nlp;

pub use agent::AgentServiceImpl;
pub use chat::ChatServiceImpl;
pub use llm::LlmServiceImpl;
