//! Chat domain module.
//!
//! Contains chat entities, repository interfaces, and service interfaces.

pub mod entities;
pub mod repositories;
pub mod services;

// Re-export core types
pub use entities::{
    ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageMetadata,
    MessageRole, MessageStatus, Thread, ThreadMetadata, ThreadPriority,
};
pub use repositories::{ChatRepository, MessageRepository, ThreadRepository};
pub use services::ChatService;
