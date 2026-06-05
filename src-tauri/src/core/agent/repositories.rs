//! Defines the repository interfaces for agent-related data.
//!
//! This module provides traits that abstract the storage and retrieval of
//! agent sessions, approvals, tasks, and execution data. These interfaces
//! decouple the core agent logic from specific database implementations.

use super::clarification::PendingClarificationRecord;
use super::errors::AgentDomainError;
use super::execution::{ExecutionCheckpoint, ExecutionSnapshot, RuntimeTrace};
use super::session::{AgentSession, PendingAction, SessionId, SessionState};
use super::task::{AgentTask, SubTask, SubTaskContract, SubTaskMode, TaskStatus};
use super::types::{Agent, AgentId};
use chrono::{DateTime, Utc};

/// A partial update applied to an existing subtask (Workstream α.1).
///
/// Each field is `Some(new_value)` to overwrite the corresponding column,
/// or `None` to leave it unchanged. The struct exists so the
/// tool-planner merge path can attach a contract, rewrite a description,
/// rename the title, or promote the mode on an already-persisted subtask
/// without bypassing the repository abstraction.
///
/// Semantics are "set-if-Some" — none of the optional fields can be
/// cleared back to SQL NULL through this struct. Clearing is not a
/// declared use case for the merge flow (the auto-planner never emits a
/// contract, and the model only refines it upward); adding an explicit
/// "clear" variant would inflate the surface area without benefit.
#[derive(Debug, Default, Clone)]
pub struct SubTaskRefinement {
    /// Overwrite the subtask title with a trimmed, non-empty value when
    /// set. Callers are responsible for the trim + non-empty validation
    /// before constructing the refinement.
    pub title: Option<String>,
    /// Overwrite the subtask description with a new value when set.
    pub description: Option<String>,
    /// Overwrite the subtask mode when set. The runtime enforces mode
    /// semantics elsewhere; the repository only persists the value.
    pub mode: Option<SubTaskMode>,
    /// Overwrite the subtask contract with a new value when set. The
    /// planner validator rejects empty contracts before they reach the
    /// repository, so the repository does not re-validate here.
    pub contract: Option<SubTaskContract>,
}

impl SubTaskRefinement {
    /// Returns `true` when every field is `None`, meaning the refinement
    /// would write no columns. The repository short-circuits on empty
    /// refinements so callers can pass them unconditionally.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.mode.is_none()
            && self.contract.is_none()
    }
}

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

    /// Flips the session's `needs_review` flag (Workstream F.2).
    ///
    /// Called when a subtask is explicitly marked `Failed` through
    /// `agent_mark_subtask_status`. Default implementation is a no-op so
    /// in-memory test doubles can opt in incrementally — the SQLite
    /// implementation overrides it.
    fn set_needs_review(&self, _id: &SessionId, _needs_review: bool) -> RepositoryResult<()> {
        Ok(())
    }

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

    /// Applies a partial update to an existing subtask's metadata
    /// (Workstream α.1).
    ///
    /// Called by the tool-planner merge path when the model's declared
    /// plan attaches a contract, rewrites a description, renames a
    /// title, or promotes the mode on a subtask that the auto-planner
    /// persisted at task-creation time with contract-less defaults.
    ///
    /// Each [`SubTaskRefinement`] field is applied only when `Some`;
    /// `None` leaves the corresponding column untouched. An empty
    /// refinement (every field `None`) is a no-op and must return
    /// `Ok(())`.
    ///
    /// The default implementation returns `Ok(())` so test doubles and
    /// in-memory repositories compile without opting in. The SQLite
    /// implementation overrides it with a dynamic UPDATE that only
    /// writes the columns whose corresponding `Option` is `Some`.
    fn refine_subtask(
        &self,
        _subtask_id: &str,
        _refinement: SubTaskRefinement,
    ) -> RepositoryResult<()> {
        Ok(())
    }

    /// Updates (or clears) the `framing` record for a specific task
    /// (Workstream H.2.b).
    ///
    /// Passing `Some(framing)` overwrites the column with the serialized
    /// JSON; `None` clears the column back to SQL NULL so the system
    /// prompt composer falls back to the genre default. Default
    /// implementation is a no-op so alternative implementations (e.g. the
    /// in-memory test double) can opt in incrementally — the SQLite
    /// implementation overrides it.
    fn update_task_framing(
        &self,
        _task_id: &str,
        _framing: Option<crate::core::agent::task::TaskFraming>,
    ) -> RepositoryResult<()> {
        Ok(())
    }
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

/// A repository for the coordinator's resume checkpoints
/// (agent-clarification-resume plan).
///
/// At most one checkpoint per session. `save_checkpoint` upserts on
/// the `session_id` primary key; `load_checkpoint` returns `None` when
/// there is nothing to resume from. `schema_version` on the checkpoint
/// is the forward-compatibility handle: a loader that finds a version
/// it does not understand returns a typed error rather than panicking.
pub trait ExecutionCheckpointRepository: Send + Sync {
    /// Inserts or replaces the checkpoint for a session.
    fn save_checkpoint(&self, checkpoint: &ExecutionCheckpoint) -> RepositoryResult<()>;

    /// Returns the persisted checkpoint for a session, if any.
    fn load_checkpoint(&self, session_id: &str) -> RepositoryResult<Option<ExecutionCheckpoint>>;

    /// Returns `true` if a checkpoint row exists for the session. Used
    /// by the post-restart reply path to distinguish "resume possible"
    /// from "session is in an orphan-repair state".
    fn checkpoint_present(&self, session_id: &str) -> RepositoryResult<bool>;

    /// Removes the checkpoint for a session.
    fn clear_checkpoint(&self, session_id: &str) -> RepositoryResult<()>;
}

/// A repository for persisting pending agent clarifications.
///
/// The at-most-one-per-session invariant is enforced at the SQL layer
/// via the primary key on `session_id`; `upsert_pending` replaces an
/// existing row when a new question supersedes the previous one.
pub trait ClarificationRepository: Send + Sync {
    /// Inserts or replaces the pending clarification for a session.
    fn upsert_pending(&self, record: &PendingClarificationRecord) -> RepositoryResult<()>;

    /// Returns the pending clarification for `session_id`, if any.
    fn get_pending(&self, session_id: &str)
        -> RepositoryResult<Option<PendingClarificationRecord>>;

    /// Returns every pending clarification across all sessions. Used by
    /// the rehydrate path at startup.
    fn list_pending(&self) -> RepositoryResult<Vec<PendingClarificationRecord>>;

    /// Removes the pending clarification for `session_id`, if any.
    fn clear_pending(&self, session_id: &str) -> RepositoryResult<()>;
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
