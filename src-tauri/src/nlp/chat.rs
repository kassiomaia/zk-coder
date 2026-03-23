//! Chat data types for conversation and thread management.
//!
//! This module provides data structures for conversations, threads, and messages
//! that match the frontend TypeScript types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Priority level for a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThreadPriority {
    /// Low priority
    Low,
    /// Medium priority (default)
    #[default]
    Medium,
    /// High priority
    High,
}

/// Feedback type for a message (like/dislike).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageFeedback {
    /// User liked the response
    Like,
    /// User disliked the response
    Dislike,
}

/// Status of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    /// Message is being sent
    Sending,
    /// Message has been sent
    #[default]
    Sent,
    /// Message has been delivered
    Delivered,
    /// Message failed to send
    Error,
}

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message
    User,
    /// Assistant/AI message
    Assistant,
    /// System message
    System,
}

impl MessageRole {
    /// Converts the role to a string for database storage.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
        }
    }

    /// Creates a role from a string.
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "assistant" => Self::Assistant,
            "system" => Self::System,
            _ => Self::User,
        }
    }
}

/// Additional metadata for a message.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessageMetadata {
    /// Detected intent from NLP
    pub intent: Option<String>,
    /// Confidence score of intent detection (0-1)
    pub confidence: Option<f64>,
    /// Token count for the message
    pub tokens: Option<usize>,
    /// AI model used for generation
    pub model: Option<String>,
    /// User feedback on the message (like/dislike)
    pub feedback: Option<MessageFeedback>,
}

/// Represents a single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    /// Unique identifier for the message
    pub id: String,
    /// ID of the conversation this message belongs to
    pub conversation_id: String,
    /// ID of the thread this message belongs to (optional)
    pub thread_id: Option<String>,
    /// Role of the message sender
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// When the message was sent
    pub timestamp: DateTime<Utc>,
    /// Current status of the message
    pub status: MessageStatus,
    /// Optional metadata about the message
    pub metadata: Option<MessageMetadata>,
}

impl ChatMessage {
    /// Creates a new user message.
    #[must_use]
    pub fn user(conversation_id: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            thread_id: None,
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: Utc::now(),
            status: MessageStatus::Sent,
            metadata: None,
        }
    }

    /// Creates a new assistant message.
    #[must_use]
    pub fn assistant(conversation_id: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            thread_id: None,
            role: MessageRole::Assistant,
            content: content.to_string(),
            timestamp: Utc::now(),
            status: MessageStatus::Sent,
            metadata: None,
        }
    }

    /// Sets the thread ID for this message.
    #[must_use]
    pub fn with_thread(mut self, thread_id: &str) -> Self {
        self.thread_id = Some(thread_id.to_string());
        self
    }
}

/// Metadata for a thread.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMetadata {
    /// Whether the thread has been resolved
    pub resolved: bool,
    /// Tags associated with the thread
    pub tags: Vec<String>,
    /// Priority level of the thread
    pub priority: ThreadPriority,
    /// Detected intent for the thread
    pub intent: Option<String>,
    /// Summary of the thread
    pub summary: Option<String>,
    /// ID of the associated agent session (if any)
    pub agent_session_id: Option<String>,
}

/// Represents a thread within a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    /// Unique identifier for the thread
    pub id: String,
    /// ID of the parent conversation
    pub conversation_id: String,
    /// ID of the parent thread (for nested threads)
    pub parent_id: Option<String>,
    /// Title/topic of the thread
    pub title: String,
    /// When the thread was created
    pub created_at: DateTime<Utc>,
    /// When the thread was last updated
    pub updated_at: DateTime<Utc>,
    /// Number of messages in the thread
    pub message_count: usize,
    /// Additional metadata about the thread
    pub metadata: ThreadMetadata,
}

impl Thread {
    /// Creates a new thread.
    #[must_use]
    pub fn new(conversation_id: &str, title: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            parent_id: None,
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            message_count: 0,
            metadata: ThreadMetadata::default(),
        }
    }
}

/// Settings for a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSettings {
    /// AI model to use
    #[serde(default = "default_model")]
    pub model: String,
    /// Temperature for response generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Maximum tokens for responses
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Custom system prompt
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Workspace directory for agent operations
    #[serde(default)]
    pub workspace_root: Option<String>,
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

impl Default for ConversationSettings {
    fn default() -> Self {
        Self {
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            system_prompt: None,
            workspace_root: None,
        }
    }
}

/// Represents a full conversation with its threads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    /// Unique identifier for the conversation
    pub id: String,
    /// Title of the conversation
    pub title: String,
    /// When the conversation was created
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated
    pub updated_at: DateTime<Utc>,
    /// Settings for this conversation
    pub settings: ConversationSettings,
}

impl Conversation {
    /// Creates a new conversation.
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            settings: ConversationSettings::default(),
        }
    }

    /// Creates a new conversation with a custom ID.
    #[must_use]
    pub fn with_id(id: &str, title: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            settings: ConversationSettings::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_conversation() {
        let conv = Conversation::new("Test Conversation");
        assert_eq!(conv.title, "Test Conversation");
        assert!(!conv.id.is_empty());
    }

    #[test]
    fn test_create_thread() {
        let thread = Thread::new("conv-123", "Test Thread");
        assert_eq!(thread.conversation_id, "conv-123");
        assert_eq!(thread.title, "Test Thread");
        assert!(!thread.metadata.resolved);
    }

    #[test]
    fn test_create_message() {
        let msg = ChatMessage::user("conv-123", "Hello world");
        assert_eq!(msg.conversation_id, "conv-123");
        assert_eq!(msg.content, "Hello world");
        assert_eq!(msg.role, MessageRole::User);
    }

    #[test]
    fn test_message_role_serialization() {
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
        assert_eq!(MessageRole::from_str("user"), MessageRole::User);
        assert_eq!(MessageRole::from_str("assistant"), MessageRole::Assistant);
    }
}
