//! Defines the core domain entities for the chat module.
//!
//! These data structures represent the fundamental concepts of the chat system,
//! such as conversations, threads, and messages. They are pure data types and
//! have no dependencies on storage or external services.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents the priority level for a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThreadPriority {
    /// Low priority.
    Low,
    /// Medium priority (the default).
    #[default]
    Medium,
    /// High priority.
    High,
}

/// Represents user feedback on a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageFeedback {
    /// The user liked the response.
    Like,
    /// The user disliked the response.
    Dislike,
}

/// Represents the status of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    /// The message is in the process of being sent.
    Sending,
    /// The message has been sent (the default).
    #[default]
    Sent,
    /// The message has been delivered to the recipient.
    Delivered,
    /// The message failed to send.
    Error,
}

/// Represents the role of the sender of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// A message sent by the user.
    User,
    /// A message sent by the assistant or AI.
    Assistant,
    /// A system-level message.
    System,
}

impl MessageRole {
    /// Converts the message role to a string slice for database storage.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
        }
    }

    /// Parses a `MessageRole` from a string slice, falling back to `User` for
    /// any unknown input. Kept for DB-read tolerance where historic rows may
    /// contain unknown role values that must not crash the application. For
    /// strict parsing with explicit error reporting, use `"..".parse::<MessageRole>()`.
    #[must_use]
    pub fn from_str_or_default(s: &str) -> Self {
        match s {
            "assistant" => Self::Assistant,
            "system" => Self::System,
            _ => Self::User,
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            other => Err(format!("unknown MessageRole: {other}")),
        }
    }
}

/// Additional metadata associated with a message.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessageMetadata {
    /// The intent detected by the NLP system.
    pub intent: Option<String>,
    /// The confidence score of the intent detection, from 0 to 1.
    pub confidence: Option<f64>,
    /// The number of tokens in the message.
    pub tokens: Option<usize>,
    /// The AI model that was used for generation.
    pub model: Option<String>,
    /// User feedback on the message, if any.
    pub feedback: Option<MessageFeedback>,
}

/// Represents a single message in a chat.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    /// The unique identifier for the message.
    pub id: String,
    /// The ID of the conversation this message belongs to.
    pub conversation_id: String,
    /// The ID of the thread this message belongs to, if any.
    pub thread_id: Option<String>,
    /// The role of the message sender.
    pub role: MessageRole,
    /// The content of the message.
    pub content: String,
    /// The timestamp of when the message was sent.
    pub timestamp: DateTime<Utc>,
    /// The current status of the message.
    pub status: MessageStatus,
    /// Optional metadata about the message.
    pub metadata: Option<MessageMetadata>,
}

impl ChatMessage {
    /// Creates a new message with the `User` role.
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

    /// Creates a new message with the `Assistant` role.
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

    /// Creates a new message with the `System` role.
    #[must_use]
    pub fn system(conversation_id: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            thread_id: None,
            role: MessageRole::System,
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

    /// Sets the status for this message.
    #[must_use]
    pub const fn with_status(mut self, status: MessageStatus) -> Self {
        self.status = status;
        self
    }
}

/// Additional metadata associated with a thread.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMetadata {
    /// A flag indicating whether the thread has been resolved.
    pub resolved: bool,
    /// A flag indicating whether the thread has been archived.
    #[serde(default)]
    pub archived: bool,
    /// A list of tags associated with the thread.
    pub tags: Vec<String>,
    /// The priority level of the thread.
    pub priority: ThreadPriority,
    /// The detected intent for the thread.
    pub intent: Option<String>,
    /// A summary of the thread's content.
    pub summary: Option<String>,
    /// The ID of the agent session associated with this thread, if any.
    pub agent_session_id: Option<String>,
    /// A flag indicating whether YOLO mode is enabled for this thread.
    #[serde(default)]
    pub yolo_mode: bool,
}

/// Represents a thread of conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    /// The unique identifier for the thread.
    pub id: String,
    /// The ID of the parent conversation.
    pub conversation_id: String,
    /// The ID of the parent thread, for nested threads.
    pub parent_id: Option<String>,
    /// The title or topic of the thread.
    pub title: String,
    /// The timestamp of when the thread was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of when the thread was last updated.
    pub updated_at: DateTime<Utc>,
    /// The number of messages in the thread.
    pub message_count: usize,
    /// Additional metadata about the thread.
    pub metadata: ThreadMetadata,
}

impl Thread {
    /// Creates a new `Thread` with the given conversation ID and title.
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

    /// Checks if the thread is marked as resolved.
    #[must_use]
    pub const fn is_resolved(&self) -> bool {
        self.metadata.resolved
    }
}

/// Represents the settings for a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSettings {
    /// The AI model to be used for the conversation.
    #[serde(default = "default_model")]
    pub model: String,
    /// The temperature setting for response generation.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// The maximum number of tokens for responses.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// A custom system prompt for the conversation.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// The workspace directory for agent operations.
    #[serde(default)]
    pub workspace_root: Option<String>,
}

/// Returns the default model for a conversation.
fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

/// Returns the default temperature for a conversation.
const fn default_temperature() -> f32 {
    0.7
}

/// Returns the default maximum tokens for a conversation.
const fn default_max_tokens() -> u32 {
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

/// Represents a full conversation, which can contain multiple threads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    /// The unique identifier for the conversation.
    pub id: String,
    /// The title of the conversation.
    pub title: String,
    /// The timestamp of when the conversation was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of when the conversation was last updated.
    pub updated_at: DateTime<Utc>,
    /// A flag indicating whether the conversation has been archived.
    #[serde(default)]
    pub archived: bool,
    /// The settings for this conversation.
    pub settings: ConversationSettings,
}

impl Conversation {
    /// Creates a new `Conversation` with the given title.
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived: false,
            settings: ConversationSettings::default(),
        }
    }

    /// Creates a new `Conversation` with a custom ID and title.
    #[must_use]
    pub fn with_id(id: &str, title: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived: false,
            settings: ConversationSettings::default(),
        }
    }
}

/// Represents an error that can occur during chat domain operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatDomainError {
    /// The thread is resolved and cannot accept new messages.
    ThreadResolved {
        /// The title of the resolved thread.
        thread_title: String,
    },
    /// The specified conversation was not found.
    ConversationNotFound {
        /// The ID of the conversation that was not found.
        id: String,
    },
    /// The specified thread was not found.
    ThreadNotFound {
        /// The ID of the thread that was not found.
        id: String,
    },
    /// The specified message was not found.
    MessageNotFound {
        /// The ID of the message that was not found.
        id: String,
    },
    /// The input provided for an operation was invalid.
    InvalidInput {
        /// A description of the reason for the invalid input.
        reason: String,
    },
}

impl std::fmt::Display for ChatDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ThreadResolved { thread_title } => write!(
                f,
                "Thread '{thread_title}' is resolved and cannot accept new messages. Reopen the thread to continue the conversation."
            ),
            Self::ConversationNotFound { id } => write!(f, "Conversation not found: {id}"),
            Self::ThreadNotFound { id } => write!(f, "Thread not found: {id}"),
            Self::MessageNotFound { id } => write!(f, "Message not found: {id}"),
            Self::InvalidInput { reason } => write!(f, "Invalid input: {reason}"),
        }
    }
}

impl std::error::Error for ChatDomainError {}

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
        assert_eq!(MessageRole::from_str_or_default("user"), MessageRole::User);
        assert_eq!(
            MessageRole::from_str_or_default("assistant"),
            MessageRole::Assistant
        );
        assert_eq!(MessageRole::from_str_or_default("unknown"), MessageRole::User);
    }

    #[test]
    fn test_message_role_from_str_trait() {
        assert_eq!("user".parse::<MessageRole>(), Ok(MessageRole::User));
        assert_eq!("assistant".parse::<MessageRole>(), Ok(MessageRole::Assistant));
        assert_eq!("system".parse::<MessageRole>(), Ok(MessageRole::System));
        assert!("unknown".parse::<MessageRole>().is_err());
        assert!("".parse::<MessageRole>().is_err());
    }

    #[test]
    fn test_thread_is_resolved() {
        let mut thread = Thread::new("conv-123", "Test");
        assert!(!thread.is_resolved());
        thread.metadata.resolved = true;
        assert!(thread.is_resolved());
    }

    #[test]
    fn test_domain_error_display() {
        let err = ChatDomainError::ThreadResolved {
            thread_title: "Test Thread".to_string(),
        };
        assert!(err.to_string().contains("resolved"));
    }
}
