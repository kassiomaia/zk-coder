//! Agent repository interfaces.
//!
//! Defines storage operations for agent sessions and approvals.

use super::entities::{
    Agent, AgentDomainError, AgentSession, AgentTask, PendingAction, SessionId, TaskStatus,
};
use chrono::{DateTime, Utc};

/// Result type for repository operations.
pub type RepositoryResult<T> = Result<T, AgentDomainError>;

/// Repository for agent session operations.
pub trait SessionRepository: Send + Sync {
    /// Creates a new agent session.
    fn create_session(&self, session: &AgentSession) -> RepositoryResult<()>;

    /// Gets a session by ID.
    fn get_session(&self, id: &SessionId) -> RepositoryResult<AgentSession>;

    /// Gets all sessions for a conversation.
    fn get_sessions_for_conversation(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<AgentSession>>;

    /// Lists all sessions.
    fn list_sessions(&self) -> RepositoryResult<Vec<AgentSession>>;

    /// Updates the session state.
    fn update_state(
        &self,
        id: &SessionId,
        state: &super::entities::SessionState,
    ) -> RepositoryResult<()>;

    /// Updates the workspace for a session.
    fn update_workspace(&self, id: &SessionId, workspace: &str) -> RepositoryResult<()>;

    /// Deletes a session.
    fn delete_session(&self, id: &SessionId) -> RepositoryResult<()>;

    /// Records a crash-recovery event for diagnostics.
    fn record_recovery_event(
        &self,
        session_id: &SessionId,
        reason: &str,
        recovered_at: DateTime<Utc>,
    ) -> RepositoryResult<()>;
}

/// Repository for agent registry operations.
pub trait AgentRepository: Send + Sync {
    /// Gets an agent by ID.
    fn get_agent(&self, id: &super::entities::AgentId) -> RepositoryResult<Agent>;

    /// Lists all available agents.
    fn list_agents(&self) -> RepositoryResult<Vec<Agent>>;
}

/// Repository for approval queue operations.
pub trait ApprovalRepository: Send + Sync {
    /// Adds a pending action.
    fn add_action(&self, action: &PendingAction) -> RepositoryResult<()>;

    /// Gets all pending actions for a session.
    fn get_pending_actions(&self, session_id: &str) -> RepositoryResult<Vec<PendingAction>>;

    /// Gets a specific action by ID.
    fn get_action(&self, action_id: &str) -> RepositoryResult<PendingAction>;

    /// Approves an action.
    fn approve_action(&self, action_id: &str) -> RepositoryResult<()>;

    /// Denies an action.
    fn deny_action(&self, action_id: &str) -> RepositoryResult<()>;

    /// Clears all actions for a session.
    fn clear_session(&self, session_id: &str) -> RepositoryResult<()>;
}

/// Repository for task tracking.
pub trait TaskRepository: Send + Sync {
    /// Creates a new task.
    fn create_task(&self, task: &AgentTask) -> RepositoryResult<()>;

    /// Creates a subtask for a parent task.
    fn create_subtask(&self, parent_task_id: &str, task: &AgentTask) -> RepositoryResult<()>;

    /// Gets tasks for a session.
    fn get_session_tasks(&self, session_id: &str) -> RepositoryResult<Vec<AgentTask>>;

    /// Gets tasks for a parent task.
    fn get_subtasks(&self, parent_task_id: &str) -> RepositoryResult<Vec<AgentTask>>;

    /// Updates task status.
    fn update_task_status(&self, task_id: &str, status: TaskStatus) -> RepositoryResult<()>;

    /// Updates subtask status.
    fn update_subtask_status(&self, subtask_id: &str, status: TaskStatus) -> RepositoryResult<()>;
}
