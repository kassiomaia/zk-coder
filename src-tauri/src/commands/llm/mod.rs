//! LLM Tauri IPC commands.
//!
//! Thin adapter layer for LLM chat completions and streaming.

/// Re-exports for orchestrator commands.
pub mod orchestrator;

/// Re-exports for LLM command handlers.
pub mod commands;
mod helpers;
/// Slash-command to prompt-template expansion.
pub mod slash_resolver;
mod state;
#[cfg(test)]
mod tests;
mod types;

// Re-exports from commands module
pub use commands::{cancel_chat_request, send_chat_message, send_chat_message_streaming};
// Re-exports from orchestrator module
pub use orchestrator::{OrchestratedConversationRunner, ORCHESTRATOR_LLM_MAX_RETRIES};
// Re-exports from state module
pub use state::{ConversationRunner, LlmState};
// Re-exports from types module
pub use types::{SendChatRequest, SendChatResponse};
