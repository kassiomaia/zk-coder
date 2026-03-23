//! Tauri command handlers for the Coder application.
//!
//! This module contains all the Tauri commands that can be invoked from the frontend.

use serde::{Deserialize, Serialize};

pub mod agent;
pub mod chat;
pub mod config;
pub mod file;
pub mod llm;
pub mod nlp;

// Re-export state types for convenience
pub use agent::AgentState;
pub use chat::ChatState;
pub use config::ConfigState;
pub use llm::LlmState;
pub use nlp::NlpState;

/// Simple health check command to verify the backend is running.
#[tauri::command]
pub fn health_check() -> String {
    "Coder backend is running!".to_string()
}

/// Example command that returns application info.
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Application information structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}
