//! Agent service interface.
//!
//! Defines business operations for agent functionality.

use super::entities::{Agent, AgentExecutionResult, AgentSession, AgentTask, PendingAction, SessionId, TaskStatus};
use super::repositories::RepositoryResult;

/// Request to create a new agent session.
#[derive(Debug, Clone)]
pub struct CreateSessionRequest {
    /// Agent type to use
    pub agent_id: String,
    /// Conversation ID
    pub conversation_id: String,
    /// Thread ID
    pub thread_id: String,
    /// Optional workspace root
    pub workspace_root: Option<String>,
}

/// Request to send to an agent session.
#[derive(Debug, Clone)]
pub struct SendAgentRequestRequest {
    /// Session ID
    pub session_id: String,
    /// Request content
    pub content: String,
}

/// Result of session details query.
#[derive(Debug, Clone)]
pub struct SessionDetails {
    /// The session
    pub session: AgentSession,
    /// Files touched during session
    pub files_touched: Vec<FileChange>,
    /// Terminal output
    pub terminal_output: Vec<TerminalLine>,
    /// Execution steps
    pub execution_steps: Vec<ExecutionStep>,
}

/// File change record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileChange {
    /// File path
    pub path: String,
    /// Action performed
    pub action: FileAction,
    /// Diff preview
    pub diff: Option<String>,
}

/// Action performed on a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileAction {
    Read,
    Write,
    Delete,
}

/// Terminal output line.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminalLine {
    /// Line identifier
    pub id: String,
    /// Line type
    pub line_type: String,
    /// Content
    pub content: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Execution step in agent workflow.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionStep {
    /// Step identifier
    pub id: String,
    /// Step type
    pub step_type: String,
    /// Description
    pub description: String,
    /// Status
    pub status: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Agent service interface.
///
/// Defines business logic operations for agent functionality.
pub trait AgentService: Send + Sync {
    // ========================================================================
    // Session Operations
    // ========================================================================

    /// Creates a new agent session.
    fn create_session(&self, request: CreateSessionRequest) -> RepositoryResult<AgentSession>;

    /// Gets a session by ID.
    fn get_session(&self, session_id: &str) -> RepositoryResult<AgentSession>;

    /// Gets sessions for a conversation.
    fn get_conversation_sessions(&self, conversation_id: &str) -> RepositoryResult<Vec<AgentSession>>;

    /// Cancels an active session.
    fn cancel_session(&self, session_id: &str) -> RepositoryResult<()>;

    /// Deletes a session.
    fn delete_session(&self, session_id: &str) -> RepositoryResult<()>;

    /// Updates session workspace.
    fn update_session_workspace(&self, session_id: &str, workspace_root: &str) -> RepositoryResult<()>;

    // ========================================================================
    // Execution Operations
    // ========================================================================

    /// Sends a request to an agent session.
    fn send_request(&self, request: SendAgentRequestRequest) -> RepositoryResult<SessionId>;

    // ========================================================================
    // Approval Operations
    // ========================================================================

    /// Gets pending actions for a session.
    fn get_pending_actions(&self, session_id: &str) -> RepositoryResult<Vec<PendingAction>>;

    /// Approves a pending action.
    fn approve_action(&self, action_id: &str) -> RepositoryResult<()>;

    /// Denies a pending action.
    fn deny_action(&self, action_id: &str) -> RepositoryResult<()>;

    // ========================================================================
    // Registry Operations
    // ========================================================================

    /// Lists all available agents.
    fn list_agents(&self) -> RepositoryResult<Vec<Agent>>;

    /// Gets a specific agent by ID.
    fn get_agent(&self, agent_id: &str) -> RepositoryResult<Agent>;

    // ========================================================================
    // Task Operations
    // ========================================================================

    /// Creates a subtask.
    fn create_subtask(
        &self,
        task_id: String,
        title: String,
        description: Option<String>,
        order: i32,
    ) -> RepositoryResult<String>;

    /// Updates subtask status.
    fn update_subtask_status(&self, subtask_id: String, status: TaskStatus) -> RepositoryResult<()>;

    /// Gets subtasks for a task.
    fn get_subtasks(&self, task_id: &str) -> RepositoryResult<Vec<AgentTask>>;

    // ========================================================================
    // History Operations
    // ========================================================================

    /// Gets session history for a conversation.
    fn get_conversation_history(&self, conversation_id: &str) -> RepositoryResult<Vec<AgentSession>>;

    /// Gets detailed session information.
    fn get_session_details(&self, session_id: &str) -> RepositoryResult<SessionDetails>;

    // ========================================================================
    // Rollback Operations
    // ========================================================================

    /// Gets file snapshots for a session.
    fn get_file_snapshots(&self, session_id: &str) -> RepositoryResult<Vec<FileChange>>;

    /// Rolls back a single file.
    fn rollback_file(&self, session_id: &str, path: &str) -> RepositoryResult<bool>;

    /// Rolls back an entire session.
    fn rollback_session(&self, session_id: &str) -> RepositoryResult<Vec<String>>;
}
