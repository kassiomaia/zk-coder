//! `SQLite` storage for agent sessions and tasks.
//!
//! Persists agent sessions, tasks, subtasks, execution steps,
//! runtime data, tool calls, and execution traces across app restarts
//! so users can resume incomplete tasks.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::core::agent::errors::AgentDomainError;
use crate::core::agent::repositories::{RepositoryResult, SessionRepository};
use crate::core::agent::session as agent_session;

pub mod clarifications;
pub mod execution_checkpoints;
pub mod execution_debug;
pub mod memory;
pub mod runtime_data;
pub mod sessions;
pub mod tasks;
pub mod tool_calls;

mod clarification_repo;
mod execution_repo;
mod memory_repo;
mod task_repo;
mod telemetry;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod tests;

pub use clarification_repo::SqliteClarificationRepository;
pub use execution_repo::SqliteExecutionRepository;
pub use memory_repo::SqliteMemoryRepository;
pub use runtime_data::{TerminalLineEntry, TouchedFileEntry};
pub use task_repo::SqliteTaskRepository;
pub use telemetry::SqliteAgentTelemetryRepository;
pub use tool_calls::ToolCallEntry;

/// Errors from the `SQLite` agent repositories.
#[derive(Debug, thiserror::Error)]
pub enum SessionRepositoryError {
    /// `SQLite` database error.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Session not found for the given ID.
    #[error("Session not found: {0}")]
    NotFound(String),
    /// Failed to acquire database lock.
    #[error("Lock error: {0}")]
    LockError(String),
    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<SessionRepositoryError> for AgentDomainError {
    fn from(err: SessionRepositoryError) -> Self {
        match err {
            SessionRepositoryError::NotFound(id) => Self::SessionNotFound { id },
            SessionRepositoryError::Sqlite(e) => Self::InvalidInput {
                reason: format!("Database error: {e}"),
            },
            SessionRepositoryError::LockError(e) => Self::InvalidInput {
                reason: format!("Lock error: {e}"),
            },
            SessionRepositoryError::Json(e) => Self::InvalidInput {
                reason: format!("JSON error: {e}"),
            },
        }
    }
}

/// SQLite-backed agent session repository.
pub struct SqliteSessionRepository {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl SqliteSessionRepository {
    /// Create a new session store at the given database path.
    pub fn new(db_path: &str) -> Result<Self, SessionRepositoryError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn)?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Creates an agent repository from a shared connection.
    ///
    /// The schema must already have been applied via `migrations::init_all`.
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Create a new session store from an owned connection.
    pub fn from_connection(conn: Connection) -> Result<Self, SessionRepositoryError> {
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // ========================================================================
    // Session methods (delegated to sessions module)
    // ========================================================================

    /// Save a new session.
    pub fn create_session(
        &self,
        session: &crate::core::agent::session::AgentSession,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::create_session(&conn, session)
    }

    /// Load a session by ID.
    pub fn get_session(
        &self,
        id: &crate::core::agent::session::SessionId,
    ) -> Result<Option<crate::core::agent::session::AgentSession>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::get_session(&conn, id)
    }

    /// Update session state.
    pub fn update_state(
        &self,
        id: &crate::core::agent::session::SessionId,
        state: &crate::core::agent::session::SessionState,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::update_state(&conn, id, state)
    }

    /// Get all sessions for a conversation.
    pub fn get_sessions_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<crate::core::agent::session::AgentSession>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::get_sessions_for_conversation(&conn, conversation_id)
    }

    /// List all sessions.
    pub fn list_sessions(
        &self,
    ) -> Result<Vec<crate::core::agent::session::AgentSession>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::list_sessions(&conn)
    }

    /// Delete a session.
    pub fn delete_session(
        &self,
        id: &crate::core::agent::session::SessionId,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::delete_session(&conn, id)
    }

    /// Update workspace root for a session.
    pub fn update_workspace(
        &self,
        id: &crate::core::agent::session::SessionId,
        workspace_root: &str,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::update_workspace(&conn, id, workspace_root)
    }

    /// Update model_id for a session.
    pub fn update_model_id(
        &self,
        id: &crate::core::agent::session::SessionId,
        model_id: &str,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::update_model_id(&conn, id, model_id)
    }

    /// Flip the `needs_review` flag for a session (Workstream F.2).
    pub fn set_needs_review(
        &self,
        id: &crate::core::agent::session::SessionId,
        needs_review: bool,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::update_needs_review(&conn, id, needs_review)
    }

    /// Record a startup crash-recovery event.
    pub fn record_recovery_event(
        &self,
        session_id: &crate::core::agent::session::SessionId,
        reason: &str,
        recovered_at: DateTime<Utc>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        sessions::record_recovery_event(&conn, session_id, reason, recovered_at)
    }

    // ========================================================================
    // Task methods (delegated to tasks module)
    // ========================================================================

    /// Create a new agent task.
    pub fn create_task(
        &self,
        task: &crate::core::agent::task::AgentTask,
        title: Option<&str>,
        description: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::create_task(&conn, task, title, description)
    }

    /// Update task status.
    pub fn update_task_status(
        &self,
        task_id: &str,
        status: &crate::core::agent::task::TaskStatus,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::update_task_status(&conn, task_id, status)
    }

    /// Update the `framing` record for a task (Workstream H.2.b).
    pub fn update_task_framing(
        &self,
        task_id: &str,
        framing: Option<&crate::core::agent::task::TaskFraming>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::update_task_framing(&conn, task_id, framing)
    }

    /// Get all tasks for a session.
    pub fn get_session_tasks(
        &self,
        session_id: &crate::core::agent::session::SessionId,
    ) -> Result<Vec<crate::core::agent::task::AgentTask>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::get_session_tasks(&conn, session_id)
    }

    /// Create a new subtask.
    pub fn create_subtask(
        &self,
        subtask: &crate::core::agent::task::SubTask,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::create_subtask(&conn, subtask)
    }

    /// Update subtask status.
    pub fn update_subtask_status(
        &self,
        subtask_id: &str,
        status: &crate::core::agent::task::TaskStatus,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::update_subtask_status(&conn, subtask_id, status)
    }

    /// Apply a partial refinement to an existing subtask (Workstream α.1).
    pub fn refine_subtask(
        &self,
        subtask_id: &str,
        refinement: &crate::core::agent::repositories::SubTaskRefinement,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::refine_subtask(&conn, subtask_id, refinement)
    }

    /// Get all subtasks for a task.
    pub fn get_subtasks(
        &self,
        task_id: &str,
    ) -> Result<Vec<crate::core::agent::task::SubTask>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::get_subtasks(&conn, task_id)
    }

    /// Delete all subtasks for a task.
    pub fn delete_subtasks(&self, task_id: &str) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::delete_subtasks(&conn, task_id)
    }

    /// Create a new execution step record.
    pub fn create_execution_step(
        &self,
        step: &crate::core::agent::task::ExecutionStep,
        session_id: &str,
        task_id: Option<&str>,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::create_execution_step(&conn, step, session_id, task_id, subtask_id)
    }

    /// Get recent execution steps for a session.
    pub fn get_recent_execution_steps(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::core::agent::task::ExecutionStep>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::get_recent_execution_steps(&conn, session_id, limit)
    }

    /// Get all execution steps for a session in chronological order.
    pub fn get_session_execution_steps(
        &self,
        session_id: &crate::core::agent::session::SessionId,
    ) -> Result<Vec<crate::core::agent::task::ExecutionStep>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::get_session_execution_steps(&conn, session_id)
    }

    // ========================================================================
    // Runtime data methods (delegated to runtime_data module)
    // ========================================================================

    /// Add a file operation record.
    pub fn add_file_touched(
        &self,
        session_id: &crate::core::agent::session::SessionId,
        file_path: &str,
        action: &str,
        diff: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        runtime_data::add_file_touched(&conn, session_id, file_path, action, diff)
    }

    /// Get all files touched for a session.
    pub fn get_session_files(
        &self,
        session_id: &crate::core::agent::session::SessionId,
    ) -> Result<Vec<TouchedFileEntry>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        runtime_data::get_session_files(&conn, session_id)
    }

    /// Add a terminal output line.
    pub fn add_terminal_line(
        &self,
        session_id: &crate::core::agent::session::SessionId,
        line_type: &str,
        content: &str,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        runtime_data::add_terminal_line(&conn, session_id, line_type, content)
    }

    /// Get all terminal output for a session.
    pub fn get_session_terminal_output(
        &self,
        session_id: &crate::core::agent::session::SessionId,
    ) -> Result<Vec<TerminalLineEntry>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        runtime_data::get_session_terminal_output(&conn, session_id)
    }

    /// Add a status phrase.
    pub fn add_status_phrase(
        &self,
        session_id: &crate::core::agent::session::SessionId,
        phrase: &str,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        runtime_data::add_status_phrase(&conn, session_id, phrase)
    }

    /// Get all status phrases for a session.
    pub fn get_session_status_phrases(
        &self,
        session_id: &crate::core::agent::session::SessionId,
    ) -> Result<Vec<String>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        runtime_data::get_session_status_phrases(&conn, session_id)
    }

    /// Clear all runtime data for a session.
    pub fn clear_session_runtime_data(
        &self,
        session_id: &crate::core::agent::session::SessionId,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        runtime_data::clear_session_runtime_data(&conn, session_id)?;
        tasks::clear_session_execution_steps(&conn, session_id)?;
        execution_debug::clear_execution_snapshot(&conn, &session_id.0)?;
        Ok(())
    }

    // ========================================================================
    // Tool call methods (delegated to tool_calls module)
    // ========================================================================

    /// Insert a tool call record.
    pub fn insert_tool_call(
        &self,
        session_id: &crate::core::agent::session::SessionId,
        tool_call_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tool_calls::insert_tool_call(
            &conn,
            session_id,
            tool_call_id,
            tool_name,
            arguments,
            subtask_id,
        )
    }

    /// Update tool call result.
    pub fn update_tool_call_result(
        &self,
        tool_call_id: &str,
        result: &serde_json::Value,
        is_error: bool,
        duration_ms: Option<i64>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tool_calls::update_tool_call_result(&conn, tool_call_id, result, is_error, duration_ms)
    }

    /// Get all tool calls for a session.
    pub fn get_tool_calls_for_session(
        &self,
        session_id: &crate::core::agent::session::SessionId,
    ) -> Result<Vec<ToolCallEntry>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tool_calls::get_tool_calls_for_session(&conn, session_id)
    }

    /// Save current execution snapshot.
    pub fn save_execution_snapshot(
        &self,
        snapshot: &crate::core::agent::execution::ExecutionSnapshot,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        execution_debug::save_execution_snapshot(&conn, snapshot)
    }

    /// Load current execution snapshot.
    pub fn get_execution_snapshot(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::core::agent::execution::ExecutionSnapshot>, SessionRepositoryError>
    {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        execution_debug::get_execution_snapshot(&conn, session_id)
    }

    /// Clear current execution snapshot.
    pub fn clear_execution_snapshot(&self, session_id: &str) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        execution_debug::clear_execution_snapshot(&conn, session_id)
    }

    /// Append a runtime trace.
    pub fn append_runtime_trace(
        &self,
        trace: &crate::core::agent::execution::RuntimeTrace,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        execution_debug::append_runtime_trace(&conn, trace)
    }

    /// Load runtime traces for a session.
    pub fn get_runtime_traces(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<crate::core::agent::execution::RuntimeTrace>, SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        execution_debug::get_runtime_traces(&conn, session_id, limit)
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl SessionRepository for SqliteSessionRepository {
    fn create_session(&self, session: &agent_session::AgentSession) -> RepositoryResult<()> {
        Self::create_session(self, session).map_err(Into::into)
    }

    fn get_session(
        &self,
        id: &agent_session::SessionId,
    ) -> RepositoryResult<agent_session::AgentSession> {
        Self::get_session(self, id)
            .map_err(AgentDomainError::from)?
            .ok_or_else(|| AgentDomainError::SessionNotFound { id: id.0.clone() })
    }

    fn get_sessions_for_conversation(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<agent_session::AgentSession>> {
        Self::get_sessions_for_conversation(self, conversation_id)
            .map_err(AgentDomainError::from)
    }

    fn list_sessions(&self) -> RepositoryResult<Vec<agent_session::AgentSession>> {
        Self::list_sessions(self).map_err(AgentDomainError::from)
    }

    fn update_state(
        &self,
        id: &agent_session::SessionId,
        state: &agent_session::SessionState,
    ) -> RepositoryResult<()> {
        Self::update_state(self, id, state).map_err(AgentDomainError::from)
    }

    fn update_workspace(
        &self,
        id: &agent_session::SessionId,
        workspace: &str,
    ) -> RepositoryResult<()> {
        Self::update_workspace(self, id, workspace).map_err(AgentDomainError::from)
    }

    fn update_model_id(
        &self,
        id: &agent_session::SessionId,
        model_id: &str,
    ) -> RepositoryResult<()> {
        Self::update_model_id(self, id, model_id).map_err(AgentDomainError::from)
    }

    fn set_needs_review(
        &self,
        id: &agent_session::SessionId,
        needs_review: bool,
    ) -> RepositoryResult<()> {
        Self::set_needs_review(self, id, needs_review).map_err(AgentDomainError::from)
    }

    fn delete_session(&self, id: &agent_session::SessionId) -> RepositoryResult<()> {
        Self::delete_session(self, id).map_err(AgentDomainError::from)
    }

    fn record_recovery_event(
        &self,
        session_id: &agent_session::SessionId,
        reason: &str,
        recovered_at: DateTime<Utc>,
    ) -> RepositoryResult<()> {
        Self::record_recovery_event(self, session_id, reason, recovered_at)
            .map_err(AgentDomainError::from)
    }
}

