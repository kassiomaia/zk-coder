//! LLM domain module.
//!
//! Contains LLM entities, provider interfaces, and service interfaces.

pub mod entities;
pub mod repositories;
pub mod services;

// Re-export core types
pub use entities::{
    ChatMessage, ChatRequest, ChatResponse, ContentBlock, ImageSource, LlmError, LlmResult,
    MessageContent, MessageRole, StreamChunk, ToolCall, ToolDefinition, UsageStats,
};
pub use repositories::{LlmClient, LlmProvider};
pub use services::LlmService;
