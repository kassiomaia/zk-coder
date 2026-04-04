//! Defines the core domain entities for Natural Language Processing (NLP).
//!
//! This module contains the data structures that represent persisted NLP context,
//! which belong to the core domain so that repository adapters can be implemented
//! entirely in the infrastructure layer.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::core::chat::entities::MessageRole;

/// Represents a context window for a conversation or thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    /// The unique identifier for the context window.
    pub id: String,
    /// The ID of the conversation this context belongs to.
    pub conversation_id: String,
    /// The ID of the thread this context belongs to, if applicable.
    pub thread_id: Option<String>,
    /// The list of messages included in the context window.
    pub messages: Vec<ContextMessage>,
    /// Additional metadata about the context.
    pub metadata: ContextMetadata,
    /// The timestamp of when the context window was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of when the context window was last updated.
    pub updated_at: DateTime<Utc>,
}

impl ContextWindow {
    /// Creates a new `ContextWindow` for a given conversation.
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

    /// Creates a new `ContextWindow` for a specific thread within a conversation.
    #[must_use]
    pub fn new_for_thread(conversation_id: &str, thread_id: &str) -> Self {
        let mut window = Self::new(conversation_id);
        window.thread_id = Some(thread_id.to_string());
        window
    }

    /// Estimates the number of tokens in the context window.
    #[must_use]
    pub fn estimate_tokens(&self) -> usize {
        self.messages
            .iter()
            .map(|m| m.content.len() / 4)
            .sum::<usize>()
            + self
                .metadata
                .summary
                .as_ref()
                .map_or(0, |s| s.len() / 4)
    }

    /// Returns a summary of the context window.
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

/// A single message within a `ContextWindow`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    /// The unique identifier for the message.
    pub id: String,
    /// The role of the message sender.
    pub role: MessageRole,
    /// The content of the message.
    pub content: String,
    /// The timestamp of when the message was sent.
    pub timestamp: DateTime<Utc>,
    /// The number of tokens in the message, if calculated.
    pub tokens: Option<usize>,
}

impl ContextMessage {
    /// Creates a new `ContextMessage` with the `User` role.
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

    /// Creates a new `ContextMessage` with the `Assistant` role.
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

    /// Creates a new `ContextMessage` with the `System` role.
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

/// Metadata that tracks the current state of the context.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextMetadata {
    /// Information about the file that is currently in context.
    pub current_file: Option<FileInfo>,
    /// A list of code symbols that are currently active in the context.
    pub active_symbols: Vec<SymbolReference>,
    /// A summary of the context.
    pub summary: Option<String>,
}

/// Information about a file that is part of the context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// The path of the file.
    pub path: String,
    /// The programming language of the file.
    pub language: String,
}

impl FileInfo {
    /// Creates a new `FileInfo`.
    #[must_use]
    pub fn new(path: &str, language: &str) -> Self {
        Self {
            path: path.to_string(),
            language: language.to_string(),
        }
    }
}

/// A reference to a symbol within a code file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    /// The name of the symbol.
    pub name: String,
    /// The kind of the symbol (e.g., function, class).
    pub kind: SymbolKind,
    /// The path of the file where the symbol is defined.
    pub file_path: String,
    /// The line number where the symbol is defined.
    pub line: usize,
}

impl SymbolReference {
    /// Creates a new `SymbolReference`.
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

/// The kind of a code symbol.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// A function.
    Function,
    /// A class or struct.
    Class,
    /// A variable.
    Variable,
    /// An interface.
    Interface,
    /// A constant.
    Constant,
    /// A type alias.
    TypeAlias,
}

/// A simplified representation of the context used for prompt assembly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationContext {
    /// A list of recent messages in the conversation.
    pub recent_messages: Vec<ContextMessage>,
    /// Metadata associated with the context.
    pub metadata: ContextMetadata,
    /// A summary of the conversation.
    pub summary: Option<String>,
}

impl ConversationContext {
    /// Creates a new, empty `ConversationContext`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            recent_messages: Vec::new(),
            metadata: ContextMetadata::default(),
            summary: None,
        }
    }

    /// Returns a summary of the conversation context.
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
