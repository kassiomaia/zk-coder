//! Context data models for conversation management.
//!
//! This module defines the types used to track conversation context,
//! including messages, metadata, and file/symbol references.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A conversation context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Unique identifier
    pub id: String,
    /// Associated conversation ID
    pub conversation_id: String,
    /// Associated thread ID (if any)
    pub thread_id: Option<String>,
    /// Messages in this context
    pub messages: Vec<ContextMessage>,
    /// Metadata about the context
    pub metadata: ContextMetadata,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl ContextWindow {
    /// Creates a new context window for a conversation.
    #[must_use]
    pub fn new(conversation_id: &str) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            thread_id: None,
            messages: Vec::new(),
            metadata: ContextMetadata::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new context window for a thread.
    #[must_use]
    pub fn new_for_thread(conversation_id: &str, thread_id: &str) -> Self {
        let mut window = Self::new(conversation_id);
        window.thread_id = Some(thread_id.to_string());
        window
    }

    /// Estimates the token count for this context.
    #[must_use]
    pub fn estimate_tokens(&self) -> usize {
        // Rough estimate: ~4 characters per token
        self.messages
            .iter()
            .map(|m| m.content.len() / 4)
            .sum::<usize>()
            + self
                .metadata
                .summary
                .as_ref()
                .map(|s| s.len() / 4)
                .unwrap_or(0)
    }

    /// Returns a summary string for debugging.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Conversation: {}, Messages: {}, File: {:?}, Symbols: {:?}",
            self.conversation_id,
            self.messages.len(),
            self.metadata.current_file.as_ref().map(|f| &f.path),
            self.metadata
                .active_symbols
                .iter()
                .map(|s| &s.name)
                .collect::<Vec<_>>()
        )
    }
}

/// A single message in the context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    /// Unique identifier
    pub id: String,
    /// Role (user, assistant, system)
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Token count (if known)
    pub tokens: Option<usize>,
}

impl ContextMessage {
    /// Creates a new user message.
    #[must_use]
    pub fn user(content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: Utc::now(),
            tokens: None,
        }
    }

    /// Creates a new assistant message.
    #[must_use]
    pub fn assistant(content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: content.to_string(),
            timestamp: Utc::now(),
            tokens: None,
        }
    }

    /// Creates a new system message.
    #[must_use]
    pub fn system(content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role: MessageRole::System,
            content: content.to_string(),
            timestamp: Utc::now(),
            tokens: None,
        }
    }
}

/// Message role in a conversation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// User message
    User,
    /// Assistant response
    Assistant,
    /// System instruction
    System,
}

impl MessageRole {
    /// Returns the string representation for serialization.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
        }
    }

    /// Parses a role from a string.
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => MessageRole::User,
        }
    }
}

/// Metadata tracking current context state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextMetadata {
    /// Currently active file
    pub current_file: Option<FileInfo>,
    /// Active symbol references
    pub active_symbols: Vec<SymbolReference>,
    /// Compressed context summary for older messages
    pub summary: Option<String>,
}

/// Information about a file in context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File path
    pub path: String,
    /// Programming language
    pub language: String,
}

impl FileInfo {
    /// Creates new file info.
    #[must_use]
    pub fn new(path: &str, language: &str) -> Self {
        Self {
            path: path.to_string(),
            language: language.to_string(),
        }
    }
}

/// Reference to a code symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// File path containing the symbol
    pub file_path: String,
    /// Line number
    pub line: usize,
}

impl SymbolReference {
    /// Creates a new symbol reference.
    #[must_use]
    pub fn new(name: &str, kind: SymbolKind, file_path: &str, line: usize) -> Self {
        Self {
            name: name.to_string(),
            kind,
            file_path: file_path.to_string(),
            line,
        }
    }
}

/// Kind of code symbol.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// Function
    Function,
    /// Class
    Class,
    /// Variable
    Variable,
    /// Interface
    Interface,
    /// Constant
    Constant,
    /// Type alias
    TypeAlias,
}

/// Simplified context for AI prompts.
#[derive(Debug, Clone, Default)]
pub struct ConversationContext {
    /// Recent messages (last N)
    pub recent_messages: Vec<ContextMessage>,
    /// Context metadata
    pub metadata: ContextMetadata,
    /// Summary of older context
    pub summary: Option<String>,
}

impl ConversationContext {
    /// Creates an empty context.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            recent_messages: Vec::new(),
            metadata: ContextMetadata::default(),
            summary: None,
        }
    }

    /// Returns a summary string for AI prompts.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "File: {:?}, Symbols: {:?}, Recent: {} messages",
            self.metadata.current_file.as_ref().map(|f| &f.path),
            self.metadata
                .active_symbols
                .iter()
                .map(|s| &s.name)
                .collect::<Vec<_>>(),
            self.recent_messages.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_window_new() {
        let window = ContextWindow::new("conv-123");
        assert_eq!(window.conversation_id, "conv-123");
        assert!(window.thread_id.is_none());
        assert!(window.messages.is_empty());
    }

    #[test]
    fn test_context_window_for_thread() {
        let window = ContextWindow::new_for_thread("conv-123", "thread-456");
        assert_eq!(window.conversation_id, "conv-123");
        assert_eq!(window.thread_id, Some("thread-456".to_string()));
    }

    #[test]
    fn test_context_message_user() {
        let msg = ContextMessage::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_context_message_assistant() {
        let msg = ContextMessage::assistant("Hi there!");
        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.content, "Hi there!");
    }

    #[test]
    fn test_message_role_serialization() {
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::from_str("assistant"), MessageRole::Assistant);
    }

    #[test]
    fn test_estimate_tokens() {
        let mut window = ContextWindow::new("conv-123");
        window.messages.push(ContextMessage::user("Hello world"));
        let tokens = window.estimate_tokens();
        assert!(tokens > 0);
    }

    #[test]
    fn test_conversation_context_summary() {
        let ctx = ConversationContext::new();
        let summary = ctx.summary();
        assert!(summary.contains("Recent: 0 messages"));
    }
}
