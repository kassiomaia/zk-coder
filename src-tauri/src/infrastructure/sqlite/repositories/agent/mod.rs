//! `SQLite` storage for agent sessions and tasks.
//!
//! Persists agent sessions, tasks, subtasks, execution steps,
//! runtime data, tool calls, and execution traces across app restarts
//! so users can resume incomplete tasks.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::core::agent::entities as core_entities;
use crate::core::agent::repositories::{
    ExecutionRepository, RepositoryResult, RuntimeTraceRepository, SessionRepository,
    TaskRepository,
};

pub mod conversions;
pub mod execution_debug;
pub mod runtime_data;
pub mod sessions;
pub mod tasks;
pub mod tool_calls;

pub use runtime_data::{TerminalLineEntry, TouchedFileEntry};
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

impl From<SessionRepositoryError> for core_entities::AgentDomainError {
    fn from(err: SessionRepositoryError) -> Self {
        match err {
            SessionRepositoryError::NotFound(id) => {
                Self::SessionNotFound { id }
            }
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
    pub(crate) conn: Mutex<Connection>,
}

/// Re-export conversion functions from the conversions module.
pub use conversions::{
    agents_task_to_core, core_task_to_agents, session_from_core, session_state_to_agents,
    session_state_to_core, session_to_core, task_mode_to_agents, task_mode_to_core,
    task_status_to_agents, task_status_to_core,
};

impl SqliteSessionRepository {
    /// Create a new session store at the given database path.
    pub fn new(db_path: &str) -> Result<Self, SessionRepositoryError> {
        let conn = Connection::open(db_path)?;
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create a new session store at the given database path.
    pub fn from_connection(conn: Connection) -> Result<Self, SessionRepositoryError> {
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
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
    ) -> Result<(), SessionRepositoryError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))?;
        tasks::create_task(&conn, task)
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
    fn create_session(&self, session: &core_entities::AgentSession) -> RepositoryResult<()> {
        let internal = session_from_core(session);
        Self::create_session(self, &internal).map_err(Into::into)
    }

    fn get_session(
        &self,
        id: &core_entities::SessionId,
    ) -> RepositoryResult<core_entities::AgentSession> {
        let internal_id = crate::core::agent::session::SessionId(id.0.clone());
        let session = Self::get_session(self, &internal_id)
            .map_err(core_entities::AgentDomainError::from)?
            .ok_or_else(|| core_entities::AgentDomainError::SessionNotFound { id: id.0.clone() })?;

        Ok(session_to_core(session))
    }

    fn get_sessions_for_conversation(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<core_entities::AgentSession>> {
        let sessions =
            Self::get_sessions_for_conversation(self, conversation_id)
                .map_err(core_entities::AgentDomainError::from)?;

        Ok(sessions.into_iter().map(session_to_core).collect())
    }

    fn list_sessions(&self) -> RepositoryResult<Vec<core_entities::AgentSession>> {
        let sessions = Self::list_sessions(self)
            .map_err(core_entities::AgentDomainError::from)?;
        Ok(sessions.into_iter().map(session_to_core).collect())
    }

    fn update_state(
        &self,
        id: &core_entities::SessionId,
        state: &core_entities::SessionState,
    ) -> RepositoryResult<()> {
        let internal_id = crate::core::agent::session::SessionId(id.0.clone());
        let internal_state = session_state_to_agents(state);
        Self::update_state(self, &internal_id, &internal_state)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn update_workspace(
        &self,
        id: &core_entities::SessionId,
        workspace: &str,
    ) -> RepositoryResult<()> {
        let internal_id = crate::core::agent::session::SessionId(id.0.clone());
        Self::update_workspace(self, &internal_id, workspace)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn delete_session(&self, id: &core_entities::SessionId) -> RepositoryResult<()> {
        let internal_id = crate::core::agent::session::SessionId(id.0.clone());
        Self::delete_session(self, &internal_id)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn record_recovery_event(
        &self,
        session_id: &core_entities::SessionId,
        reason: &str,
        recovered_at: DateTime<Utc>,
    ) -> RepositoryResult<()> {
        let internal_id = crate::core::agent::session::SessionId(session_id.0.clone());
        Self::record_recovery_event(self, &internal_id, reason, recovered_at)
            .map_err(core_entities::AgentDomainError::from)
    }
}

impl TaskRepository for SqliteSessionRepository {
    fn create_task(&self, task: &core_entities::AgentTask) -> RepositoryResult<()> {
        let internal = core_task_to_agents(task);
        Self::create_task(self, &internal)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn create_subtask(
        &self,
        parent_task_id: &str,
        task: &core_entities::AgentTask,
    ) -> RepositoryResult<()> {
        let internal = crate::core::agent::task::SubTask {
            id: task.id.clone(),
            task_id: parent_task_id.to_string(),
            title: task.title.clone(),
            description: task.description.clone(),
            mode: task
                .mode
                .map_or(crate::core::agent::task::SubTaskMode::FullAccess, task_mode_to_agents),
            status: task_status_to_agents(task.status),
            order: task.order,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
        };

        Self::create_subtask(self, &internal)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn get_session_tasks(
        &self,
        session_id: &str,
    ) -> RepositoryResult<Vec<core_entities::AgentTask>> {
        let internal_id = crate::core::agent::session::SessionId(session_id.to_string());
        let tasks = Self::get_session_tasks(self, &internal_id)
            .map_err(core_entities::AgentDomainError::from)?;

        Ok(tasks.iter().map(agents_task_to_core).collect())
    }

    fn get_subtasks(
        &self,
        parent_task_id: &str,
    ) -> RepositoryResult<Vec<core_entities::AgentTask>> {
        let subtasks = Self::get_subtasks(self, parent_task_id)
            .map_err(core_entities::AgentDomainError::from)?;

        Ok(subtasks
            .iter()
            .map(|subtask| core_entities::AgentTask {
                id: subtask.id.clone(),
                session_id: String::new(),
                parent_task_id: Some(subtask.task_id.clone()),
                title: subtask.title.clone(),
                description: subtask.description.clone(),
                status: task_status_to_core(subtask.status.clone()),
                order: subtask.order,
                mode: Some(task_mode_to_core(subtask.mode.clone())),
                needs_review: false,
            })
            .collect())
    }

    fn update_task_status(
        &self,
        task_id: &str,
        status: core_entities::TaskStatus,
    ) -> RepositoryResult<()> {
        let internal_status = task_status_to_agents(status);
        Self::update_task_status(self, task_id, &internal_status)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn update_subtask_status(
        &self,
        subtask_id: &str,
        status: core_entities::TaskStatus,
    ) -> RepositoryResult<()> {
        let internal_status = task_status_to_agents(status);
        Self::update_subtask_status(self, subtask_id, &internal_status)
            .map_err(core_entities::AgentDomainError::from)
    }
}

impl ExecutionRepository for SqliteSessionRepository {
    fn save_execution_snapshot(
        &self,
        snapshot: &crate::core::agent::execution::ExecutionSnapshot,
    ) -> RepositoryResult<()> {
        Self::save_execution_snapshot(self, snapshot)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn get_execution_snapshot(
        &self,
        session_id: &str,
    ) -> RepositoryResult<Option<crate::core::agent::execution::ExecutionSnapshot>> {
        Self::get_execution_snapshot(self, session_id)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn clear_execution_snapshot(&self, session_id: &str) -> RepositoryResult<()> {
        Self::clear_execution_snapshot(self, session_id)
            .map_err(core_entities::AgentDomainError::from)
    }
}

impl RuntimeTraceRepository for SqliteSessionRepository {
    fn append_runtime_trace(
        &self,
        trace: &crate::core::agent::execution::RuntimeTrace,
    ) -> RepositoryResult<()> {
        Self::append_runtime_trace(self, trace)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn get_runtime_traces(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> RepositoryResult<Vec<crate::core::agent::execution::RuntimeTrace>> {
        Self::get_runtime_traces(self, session_id, limit)
            .map_err(core_entities::AgentDomainError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> crate::core::agent::session::AgentSession {
        crate::core::agent::session::AgentSession::new(
            crate::core::agent::types::AgentId::new("code-agent"),
            "Code Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        )
    }

    #[test]
    fn test_session_store_integration() {
        let repository = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session();

        repository.create_session(&session).unwrap();
        let loaded = repository.get_session(&session.id).unwrap();

        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.agent_id.0, "code-agent");
        assert_eq!(loaded.conversation_id, "conv-1");
    }

    #[test]
    fn test_task_integration() {
        let repository = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session();
        repository.create_session(&session).unwrap();

        let task = crate::core::agent::task::AgentTask::new(
            session.id.clone(),
            crate::core::agent::task::AgentRequest {
                content: "Test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        repository.create_task(&task).unwrap();

        let tasks = repository.get_session_tasks(&session.id).unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn test_subtask_integration() {
        let repository = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session();
        repository.create_session(&session).unwrap();

        let task = crate::core::agent::task::AgentTask::new(
            session.id.clone(),
            crate::core::agent::task::AgentRequest {
                content: "Test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        repository.create_task(&task).unwrap();

        let subtask = crate::core::agent::task::SubTask::new(
            task.id.clone(),
            "Test Subtask".to_string(),
            crate::core::agent::task::SubTaskMode::Read,
            0,
        );
        repository.create_subtask(&subtask).unwrap();

        let subtasks = repository.get_subtasks(&task.id).unwrap();
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0].title, "Test Subtask");
    }

    #[test]
    fn test_runtime_data_integration() {
        let repository = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session();
        repository.create_session(&session).unwrap();

        repository
            .add_file_touched(&session.id, "/test/file.txt", "write", Some("diff"))
            .unwrap();
        repository
            .add_terminal_line(&session.id, "stdout", "output")
            .unwrap();
        repository
            .add_status_phrase(&session.id, "Working...")
            .unwrap();

        let files = repository.get_session_files(&session.id).unwrap();
        assert_eq!(files.len(), 1);

        let terminal = repository.get_session_terminal_output(&session.id).unwrap();
        assert_eq!(terminal.len(), 1);

        let phrases = repository.get_session_status_phrases(&session.id).unwrap();
        assert_eq!(phrases.len(), 1);
    }

    #[test]
    fn test_tool_call_integration() {
        let repository = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session();
        repository.create_session(&session).unwrap();

        repository
            .insert_tool_call(
                &session.id,
                "tool-1",
                "fs_read_file",
                &serde_json::json!({"path": "/test"}),
                None,
            )
            .unwrap();

        let tool_calls = repository.get_tool_calls_for_session(&session.id).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].tool_name, "fs_read_file");
    }

    #[test]
    fn test_clear_session_runtime_data_preserves_runtime_traces() {
        let repository = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session();
        repository.create_session(&session).unwrap();
        let task = crate::core::agent::task::AgentTask::new(
            session.id.clone(),
            crate::core::agent::task::AgentRequest {
                content: "Test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        repository.create_task(&task).unwrap();

        let trace = crate::core::agent::execution::RuntimeTrace {
            id: "trace-1".to_string(),
            session_id: session.id.0.clone(),
            task_id: task.id.clone(),
            subtask_id: None,
            step_number: 1,
            trace_type: crate::core::agent::execution::RuntimeTraceType::ExecutionStarted,
            payload_json: serde_json::json!({"requestContent": "hello"}),
            created_at: chrono::Utc::now(),
        };
        repository.append_runtime_trace(&trace).unwrap();
        repository
            .add_terminal_line(&session.id, "stdout", "temporary output")
            .unwrap();

        repository.clear_session_runtime_data(&session.id).unwrap();

        let traces = repository.get_runtime_traces(&session.id.0, None).unwrap();
        let terminal = repository.get_session_terminal_output(&session.id).unwrap();

        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].id, "trace-1");
        assert!(terminal.is_empty());
    }
}
