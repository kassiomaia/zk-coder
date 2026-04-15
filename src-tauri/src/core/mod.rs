//! Core domain layer for the Coder application.
//!
//! This module contains the fundamental domain entities, value objects,
//! repository interfaces, and service interfaces. The core layer has
//! no dependencies on infrastructure or application logic.

pub mod agent;
pub mod chat;
pub mod config;
pub mod documents;
pub mod llm;
pub mod nlp;

// Re-export commonly used types
pub use chat::entities::*;

// Re-export config types for convenience
pub use config::*;
