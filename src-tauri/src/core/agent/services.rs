//! Defines the service interface for agent-related business operations.
//!
//! This module provides the `AgentService` trait, which outlines the core
//! functionalities for managing agent sessions, handling requests, and other
//! agent-related operations.

use super::entities::{
    Agent, AgentExecutionResult, AgentSession, AgentTask, PendingAction, SessionId, TaskStatus,
};
use super::repositories::RepositoryResult;

/// A request to create a new agent session.
#[derive(Debug, Clone)]
pub struct CreateSessionRequest {
    /// The ID of the agent type to be used for the session.
    pub agent_id: String,
    /// The ID of the conversation this session belongs to.
    pub conversation_id: String,
    /// The ID of the thread within the conversation.
    pub thread_id: String,
    /// An optional path to the workspace root for file operations.
    pub workspace_root: Option<String>,
    /// The cached model ID from the conversation settings.
    pub model_id: Option<String>,
}

/// A request to be sent to an active agent session.
#[derive(Debug, Clone)]
pub struct SendAgentRequestRequest {
    /// The ID of the session to send the request to.
    pub session_id: String,
    /// The content of the request.
    pub content: String,
}

/// A detailed view of an agent session, including related data.
#[derive(Debug, Clone)]
pub struct SessionDetails {
    /// The agent session itself.
    pub session: AgentSession,
    /// A list of files that were touched during the session.
    pub files_touched: Vec<FileChange>,
    /// A log of terminal output from the session.
    pub terminal_output: Vec<TerminalLine>,
    /// A list of execution steps that occurred during the session.
    pub execution_steps: Vec<ExecutionStep>,
}

/// Represents a change made to a file during an agent session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileChange {
    /// The path of the file that was changed.
    pub path: String,
    /// The type of action that was performed on the file.
    pub action: FileAction,
    /// An optional diff preview of the change.
    pub diff: Option<String>,
}

/// The type of action performed on a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileAction {
    /// The file was read.
    Read,
    /// The file was written to.
    Write,
    /// The file was deleted.
    Delete,
}

/// A single line of terminal output from an agent session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminalLine {
    /// A unique identifier for the line.
    pub id: String,
    /// The type of the line (e.g., "stdout", "stderr").
    pub line_type: String,
    /// The content of the line.
    pub content: String,
    /// The timestamp of when the line was recorded.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// A single step in the agent's execution workflow.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionStep {
    /// A unique identifier for the step.
    pub id: String,
    /// The type of the step (e.g., "tool_call", "llm_request").
    pub step_type: String,
    /// A description of the step.
    pub description: String,
    /// The status of the step (e.g., "in_progress", "completed").
    pub status: String,
    /// The timestamp of when the step was recorded.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// The main service interface for agent-related business logic.
///
/// This trait defines the high-level operations that can be performed with agents,
/// abstracting away the underlying implementation details.
pub trait AgentService: Send + Sync {
    // ========================================================================
    // Session Operations
    // ========================================================================

    /// Creates a new agent session based on the provided request.
    fn create_session(&self, request: CreateSessionRequest) -> RepositoryResult<AgentSession>;

    /// Retrieves an agent session by its ID.
    fn get_session(&self, session_id: &str) -> RepositoryResult<AgentSession>;

    /// Retrieves all sessions associated with a specific conversation.
    fn get_conversation_sessions(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<AgentSession>>;

    /// Cancels an active agent session.
    fn cancel_session(&self, session_id: &str) -> RepositoryResult<()>;

    /// Deletes an agent session.
    fn delete_session(&self, session_id: &str) -> RepositoryResult<()>;


    /// Updates the workspace for an agent session.
    fn update_session_workspace(
        &self,
        session_id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<()>;

    // ========================================================================
    // Execution Operations
    // ========================================================================

    /// Sends a request to an active agent session for processing.
    fn send_request(&self, request: SendAgentRequestRequest) -> RepositoryResult<SessionId>;

    // ========================================================================
    // Approval Operations
    // ========================================================================

    /// Retrieves a list of pending actions for a session that require user approval.
    fn get_pending_actions(&self, session_id: &str) -> RepositoryResult<Vec<PendingAction>>;

    /// Approves a pending action, allowing the agent to proceed.
    fn approve_action(&self, action_id: &str) -> RepositoryResult<()>;

    /// Denies a pending action, preventing the agent from executing it.
    fn deny_action(&self, action_id: &str) -> RepositoryResult<()>;

    // ========================================================================
    // Registry Operations
    // ========================================================================

    /// Lists all available agents.
    fn list_agents(&self) -> RepositoryResult<Vec<Agent>>;


    /// Retrieves a specific agent by its ID.
    fn get_agent(&self, agent_id: &str) -> RepositoryResult<Agent>;

    // ========================================================================
    // Task Operations
    // ========================================================================

    /// Creates a new subtask for an existing task.
    fn create_subtask(
        &self,
        task_id: String,
        title: String,
        description: Option<String>,
        order: i32,
    ) -> RepositoryResult<String>;

    /// Updates the status of a specific subtask.
    fn update_subtask_status(&self, subtask_id: String, status: TaskStatus)
        -> RepositoryResult<()>;

    /// Retrieves all subtasks for a given parent task.
    fn get_subtasks(&self, task_id: &str) -> RepositoryResult<Vec<AgentTask>>;

    // ========================================================================
    // History Operations
    // ========================================================================

    /// Retrieves the history of all sessions for a specific conversation.
    fn get_conversation_history(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<AgentSession>>;

    /// Retrieves detailed information about a specific session.
    fn get_session_details(&self, session_id: &str) -> RepositoryResult<SessionDetails>;

    // ========================================================================
    // Rollback Operations
    // ========================================================================

    /// Retrieves a list of file snapshots for a session, showing changes made.
    fn get_file_snapshots(&self, session_id: &str) -> RepositoryResult<Vec<FileChange>>;

    /// Rolls back the changes made to a single file during a session.
    fn rollback_file(&self, session_id: &str, path: &str) -> RepositoryResult<bool>;

    /// Rolls back all changes made during an entire session.
    fn rollback_session(&self, session_id: &str) -> RepositoryResult<Vec<String>>;
}
