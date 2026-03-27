//! NLP domain entities.
//!
//! Persisted NLP context belongs to the core domain so repository adapters can
//! live entirely in infrastructure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::core::chat::entities::MessageRole;

/// A conversation context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    pub id: String,
    pub conversation_id: String,
    pub thread_id: Option<String>,
    pub messages: Vec<ContextMessage>,
    pub metadata: ContextMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ContextWindow {
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

    #[must_use]
    pub fn new_for_thread(conversation_id: &str, thread_id: &str) -> Self {
        let mut window = Self::new(conversation_id);
        window.thread_id = Some(thread_id.to_string());
        window
    }

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
                .map(|s| s.len() / 4)
                .unwrap_or(0)
    }

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

/// A single message in the context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tokens: Option<usize>,
}

impl ContextMessage {
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

/// Metadata tracking current context state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextMetadata {
    pub current_file: Option<FileInfo>,
    pub active_symbols: Vec<SymbolReference>,
    pub summary: Option<String>,
}

/// Information about a file in context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub language: String,
}

impl FileInfo {
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
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line: usize,
}

impl SymbolReference {
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
    Function,
    Class,
    Variable,
    Interface,
    Constant,
    TypeAlias,
}

/// Simplified context used by prompt assembly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationContext {
    pub recent_messages: Vec<ContextMessage>,
    pub metadata: ContextMetadata,
    pub summary: Option<String>,
}

impl ConversationContext {
    #[must_use]
    pub fn new() -> Self {
        Self {
            recent_messages: Vec::new(),
            metadata: ContextMetadata::default(),
            summary: None,
        }
    }

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
