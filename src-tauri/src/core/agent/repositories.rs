//! Defines the repository interfaces for agent-related data.
//!
//! This module provides traits that abstract the storage and retrieval of
//! agent sessions, approvals, tasks, and execution data. These interfaces
//! decouple the core agent logic from specific database implementations.

use super::errors::AgentDomainError;
use super::execution::{ExecutionSnapshot, RuntimeTrace};
use super::session::{AgentSession, PendingAction, SessionId, SessionState};
use super::task::{AgentTask, SubTask, TaskStatus};
use super::types::{Agent, AgentId};
use chrono::{DateTime, Utc};

/// A specialized `Result` type for repository operations.
pub type RepositoryResult<T> = Result<T, AgentDomainError>;

/// A repository for managing `AgentSession` data.
pub trait SessionRepository: Send + Sync {
    /// Creates a new agent session in the repository.
    fn create_session(&self, session: &AgentSession) -> RepositoryResult<()>;

    /// Retrieves an agent session by its unique ID.
    fn get_session(&self, id: &SessionId) -> RepositoryResult<AgentSession>;

    /// Retrieves all agent sessions associated with a specific conversation.
    fn get_sessions_for_conversation(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<AgentSession>>;

    /// Lists all agent sessions in the repository.
    fn list_sessions(&self) -> RepositoryResult<Vec<AgentSession>>;

    /// Updates the state of a specific agent session.
    fn update_state(&self, id: &SessionId, state: &SessionState) -> RepositoryResult<()>;

    /// Updates the workspace directory for a specific session.
    fn update_workspace(&self, id: &SessionId, workspace: &str) -> RepositoryResult<()>;

    /// Updates the model ID for a specific session.
    fn update_model_id(&self, id: &SessionId, model_id: &str) -> RepositoryResult<()>;

    /// Deletes an agent session from the repository.
    fn delete_session(&self, id: &SessionId) -> RepositoryResult<()>;

    /// Records a crash recovery event for diagnostic purposes.
    fn record_recovery_event(
        &self,
        session_id: &SessionId,
        reason: &str,
        recovered_at: DateTime<Utc>,
    ) -> RepositoryResult<()>;
}

/// A repository for managing agent definitions.
pub trait AgentRepository: Send + Sync {
    /// Retrieves an agent definition by its unique ID.
    fn get_agent(&self, id: &AgentId) -> RepositoryResult<Agent>;

    /// Lists all available agent definitions.
    fn list_agents(&self) -> RepositoryResult<Vec<Agent>>;
}

/// A repository for managing the approval queue for pending actions.
pub trait ApprovalRepository: Send + Sync {
    /// Adds a new pending action to the approval queue.
    fn add_action(&self, action: &PendingAction) -> RepositoryResult<()>;

    /// Retrieves all pending actions for a specific session.
    fn get_pending_actions(&self, session_id: &str) -> RepositoryResult<Vec<PendingAction>>;

    /// Retrieves a specific pending action by its ID.
    fn get_action(&self, action_id: &str) -> RepositoryResult<PendingAction>;

    /// Marks a pending action as approved.
    fn approve_action(&self, action_id: &str) -> RepositoryResult<()>;

    /// Marks a pending action as denied.
    fn deny_action(&self, action_id: &str) -> RepositoryResult<()>;

    /// Clears all pending actions for a specific session.
    fn clear_session(&self, session_id: &str) -> RepositoryResult<()>;
}

/// A repository for tracking agent tasks and subtasks.
pub trait TaskRepository: Send + Sync {
    /// Creates a new task in the repository.
    fn create_task(&self, task: &AgentTask) -> RepositoryResult<()>;

    /// Creates a new subtask associated with a parent task.
    fn create_subtask(&self, subtask: &SubTask) -> RepositoryResult<()>;

    /// Retrieves all tasks for a specific session.
    fn get_session_tasks(&self, session_id: &str) -> RepositoryResult<Vec<AgentTask>>;

    /// Retrieves all subtasks for a specific parent task.
    fn get_subtasks(&self, parent_task_id: &str) -> RepositoryResult<Vec<SubTask>>;

    /// Updates the status of a specific task.
    fn update_task_status(&self, task_id: &str, status: TaskStatus) -> RepositoryResult<()>;

    /// Updates the status of a specific subtask.
    fn update_subtask_status(&self, subtask_id: &str, status: TaskStatus) -> RepositoryResult<()>;
}

/// A repository for persisting execution snapshots.
pub trait ExecutionRepository: Send + Sync {
    /// Saves or updates the current execution snapshot for a session.
    fn save_execution_snapshot(&self, snapshot: &ExecutionSnapshot) -> RepositoryResult<()>;

    /// Retrieves the current execution snapshot for a session, if it exists.
    fn get_execution_snapshot(
        &self,
        session_id: &str,
    ) -> RepositoryResult<Option<ExecutionSnapshot>>;

    /// Clears the execution snapshot for a session.
    fn clear_execution_snapshot(&self, session_id: &str) -> RepositoryResult<()>;
}

/// A repository for storing and retrieving append-only runtime traces.
pub trait RuntimeTraceRepository: Send + Sync {
    /// Appends a new runtime trace to the repository.
    fn append_runtime_trace(&self, trace: &RuntimeTrace) -> RepositoryResult<()>;

    /// Retrieves recent runtime traces for a session in chronological order.
    fn get_runtime_traces(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> RepositoryResult<Vec<RuntimeTrace>>;
}
