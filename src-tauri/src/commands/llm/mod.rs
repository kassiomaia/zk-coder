//! LLM Tauri IPC commands.
//!
//! Thin adapter layer for LLM chat completions and streaming.

pub mod orchestrator;

pub mod commands;
mod helpers;
mod state;
#[cfg(test)]
mod tests;
mod types;

pub use commands::{cancel_chat_request, send_chat_message, send_chat_message_streaming};
pub use orchestrator::{OrchestratedConversationRunner, ORCHESTRATOR_LLM_MAX_RETRIES};
pub use state::{ConversationRunner, LlmState};
pub use types::{SendChatRequest, SendChatResponse};
