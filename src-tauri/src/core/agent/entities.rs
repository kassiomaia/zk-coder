//! Agent domain entities.
//!
//! Core data structures for agent sessions, tasks, and approvals.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for an agent type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    /// Creates a new agent ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl AsRef<str> for AgentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Unique identifier for a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    /// Creates a new session ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a session ID from a string.
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier
    pub id: AgentId,
    /// Display name
    pub name: String,
    /// Description of capabilities
    pub description: String,
    /// System prompt for the agent
    pub system_prompt: String,
    /// Trust mode for approvals
    pub trust_mode: AgentTrustMode,
}

/// Trust mode determines when agent actions require approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTrustMode {
    /// All actions require approval
    Always,
    /// Only risky actions require approval
    RiskyOnly,
    /// No actions require approval
    Never,
}

/// State of an agent session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SessionState {
    /// Session created, waiting for first request
    WaitForRequest { last_action_at: DateTime<Utc> },
    /// Processing a request
    Processing { started_at: DateTime<Utc> },
    /// Waiting for user approval on an action
    AwaitingApproval {
        action_id: String,
        requested_at: DateTime<Utc>,
    },
    /// Session completed successfully
    Finished {
        completed_at: DateTime<Utc>,
        result: String,
    },
    /// Session encountered an error
    Error {
        error_at: DateTime<Utc>,
        error: String,
    },
    /// Session was cancelled
    Cancelled {
        cancelled_at: DateTime<Utc>,
        reason: String,
    },
}

/// Agent session tracking execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Unique session identifier
    pub id: SessionId,
    /// Agent type being used
    pub agent_id: AgentId,
    /// Agent display name
    pub agent_name: String,
    /// Associated conversation ID
    pub conversation_id: String,
    /// Associated thread ID
    pub thread_id: String,
    /// Workspace root for file operations
    pub workspace_root: Option<PathBuf>,
    /// Current session state
    #[serde(flatten)]
    pub state: SessionState,
    /// When the session was created
    pub created_at: DateTime<Utc>,
}

impl AgentSession {
    /// Creates a new agent session.
    pub fn new(
        agent_id: AgentId,
        agent_name: String,
        conversation_id: String,
        thread_id: String,
        workspace_root: Option<PathBuf>,
    ) -> Self {
        Self {
            id: SessionId::new(),
            agent_id,
            agent_name,
            conversation_id,
            thread_id,
            workspace_root,
            state: SessionState::WaitForRequest {
                last_action_at: Utc::now(),
            },
            created_at: Utc::now(),
        }
    }

    /// Checks if the session is in a terminal state.
    pub fn is_finished(&self) -> bool {
        matches!(
            self.state,
            SessionState::Finished { .. }
                | SessionState::Error { .. }
                | SessionState::Cancelled { .. }
        )
    }

    /// Checks if the session is currently processing.
    pub fn is_processing(&self) -> bool {
        matches!(self.state, SessionState::Processing { .. })
    }
}

/// Request to send to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    /// User's request content
    pub content: String,
    /// Optional file attachments
    pub attachments: Vec<Attachment>,
    /// Additional context
    pub context: Option<serde_json::Value>,
}

/// File attachment for agent requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// File path
    pub path: PathBuf,
    /// File content (optional, for small files)
    pub content: Option<Vec<u8>>,
}

/// Action requiring user approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    /// Unique action identifier
    pub id: String,
    /// Session this action belongs to
    pub session_id: String,
    /// Action type (e.g., "write_file", "run_command")
    pub action_type: String,
    /// Action description for user
    pub description: String,
    /// Action parameters
    pub parameters: serde_json::Value,
    /// When approval was requested
    pub requested_at: DateTime<Utc>,
    /// Current status
    pub status: ApprovalStatus,
}

/// Status of an approval action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    /// Waiting for user decision
    Pending,
    /// User approved the action
    Approved,
    /// User denied the action
    Denied,
}

/// User's decision on an approval action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalAction {
    Approve,
    Deny,
}

/// A task within an agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Unique task identifier
    pub id: String,
    /// Session this task belongs to
    pub session_id: String,
    /// Parent task when this record represents a subtask.
    pub parent_task_id: Option<String>,
    /// Task title
    pub title: String,
    /// Detailed description
    pub description: Option<String>,
    /// Task status
    pub status: TaskStatus,
    /// Display order
    pub order: i32,
    /// Optional execution mode for the task/subtask.
    pub mode: Option<TaskMode>,
}

/// Status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is pending
    Pending,
    /// Task is in progress
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
}

/// Execution mode for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskMode {
    Read,
    Write,
    Execute,
    FullAccess,
}

impl TaskMode {
    pub fn allowed_tools(&self) -> Vec<&'static str> {
        match self {
            TaskMode::Read => vec![
                "fs_read_file",
                "search_grep",
                "fs_list_files",
                "agent_emit_status",
                "agent_advance_to_next_subtask",
            ],
            TaskMode::Write => vec![
                "fs_read_file",
                "fs_write_file",
                "search_grep",
                "fs_list_files",
                "agent_emit_status",
                "agent_advance_to_next_subtask",
            ],
            TaskMode::Execute => vec![
                "fs_read_file",
                "search_google_scrape",
                "fs_list_files",
                "system_execute_command",
                "agent_emit_status",
                "agent_advance_to_next_subtask",
            ],
            TaskMode::FullAccess => vec![
                "fs_read_file",
                "fs_write_file",
                "fs_list_files",
                "search_grep",
                "system_execute",
                "search_google_scrape",
                "system_execute_command",
                "agent_emit_status",
                "agent_plan_subtasks",
                "agent_advance_to_next_subtask",
            ],
        }
    }

    pub fn instruction_text(&self) -> &'static str {
        match self {
            TaskMode::Read => {
                "READ-ONLY: You may only read files and search code. \
                DO NOT create new plans, write files, delete files, or execute commands."
            }
            TaskMode::Write => {
                "WRITE: You may read and write files. \
                DO NOT create new plans or execute commands."
            }
            TaskMode::Execute => {
                "EXECUTE: You may read files and execute commands. \
                DO NOT create new plans or write files."
            }
            TaskMode::FullAccess => {
                "FULL ACCESS: You may read files, write files, and execute commands."
            }
        }
    }
}

/// Result of agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionResult {
    /// Final response content
    pub content: String,
    /// Whether execution was successful
    pub success: bool,
    /// Usage statistics
    pub usage: Option<UsageStats>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Input tokens used
    pub input_tokens: u32,
    /// Output tokens generated
    pub output_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
}

/// Error type for agent domain operations.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentDomainError {
    /// Session not found
    SessionNotFound { id: String },
    /// Agent not found
    AgentNotFound { id: String },
    /// Session is in invalid state for operation
    InvalidSessionState { state: SessionState },
    /// Action not found
    ActionNotFound { id: String },
    /// Invalid input
    InvalidInput { reason: String },
}

impl std::fmt::Display for AgentDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionNotFound { id } => write!(f, "Session not found: {id}"),
            Self::AgentNotFound { id } => write!(f, "Agent not found: {id}"),
            Self::InvalidSessionState { state } => write!(f, "Invalid session state: {state:?}"),
            Self::ActionNotFound { id } => write!(f, "Action not found: {id}"),
            Self::InvalidInput { reason } => write!(f, "Invalid input: {reason}"),
        }
    }
}

impl std::error::Error for AgentDomainError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = AgentSession::new(
            AgentId::new("code-agent"),
            "Code Agent".to_string(),
            "conv-123".to_string(),
            "thread-456".to_string(),
            Some(PathBuf::from("/workspace")),
        );
        assert_eq!(session.agent_id.0, "code-agent");
        assert_eq!(session.agent_name, "Code Agent");
        assert!(matches!(session.state, SessionState::WaitForRequest { .. }));
    }

    #[test]
    fn test_session_is_finished() {
        let mut session = AgentSession::new(
            AgentId::new("test"),
            "Test".to_string(),
            "conv".to_string(),
            "thread".to_string(),
            None,
        );
        assert!(!session.is_finished());

        session.state = SessionState::Finished {
            completed_at: Utc::now(),
            result: "Done".to_string(),
        };
        assert!(session.is_finished());
    }

    #[test]
    fn test_agent_id_from_string() {
        let id = AgentId::from("test-agent".to_string());
        assert_eq!(id.as_ref(), "test-agent");
    }

    #[test]
    fn test_session_id_display() {
        let id = SessionId::from_string("session-123");
        assert_eq!(id.to_string(), "session-123");
        assert_eq!(id.as_ref(), "session-123");
    }
}
