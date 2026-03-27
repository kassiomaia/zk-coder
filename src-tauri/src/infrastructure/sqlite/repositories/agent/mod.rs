//! SQLite storage for agent sessions and tasks.
//!
//! Persists agent sessions across app restarts so users can
//! resume incomplete tasks.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::core::agent::entities as core_entities;
use crate::core::agent::repositories::{RepositoryResult, SessionRepository, TaskRepository};

pub mod runtime_data;
pub mod sessions;
pub mod tasks;
pub mod tool_calls;

pub use runtime_data::{TerminalLineEntry, TouchedFileEntry};
pub use tool_calls::ToolCallEntry;

/// Errors from the SQLite agent repositories.
#[derive(Debug, thiserror::Error)]
pub enum SessionRepositoryError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Lock error: {0}")]
    LockError(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<SessionRepositoryError> for core_entities::AgentDomainError {
    fn from(err: SessionRepositoryError) -> Self {
        match err {
            SessionRepositoryError::NotFound(id) => {
                core_entities::AgentDomainError::SessionNotFound { id }
            }
            SessionRepositoryError::Sqlite(e) => core_entities::AgentDomainError::InvalidInput {
                reason: format!("Database error: {e}"),
            },
            SessionRepositoryError::LockError(e) => core_entities::AgentDomainError::InvalidInput {
                reason: format!("Lock error: {e}"),
            },
            SessionRepositoryError::Json(e) => core_entities::AgentDomainError::InvalidInput {
                reason: format!("JSON error: {e}"),
            },
        }
    }
}

/// SQLite-backed agent session repository.
pub struct SqliteSessionRepository {
    pub(crate) conn: Mutex<Connection>,
}

fn session_state_to_agents(
    state: &core_entities::SessionState,
) -> crate::core::agent::session::SessionState {
    use crate::core::agent::session::SessionState as AgentSessionState;

    match state {
        core_entities::SessionState::WaitForRequest { last_action_at } => {
            AgentSessionState::WaitForRequest {
                last_action_at: *last_action_at,
            }
        }
        core_entities::SessionState::Processing { .. } => AgentSessionState::Running {
            current_step: 0,
            context_usage: 0.0,
        },
        core_entities::SessionState::AwaitingApproval { action_id, .. } => {
            AgentSessionState::UserInputRequired {
                pending_action_ids: vec![action_id.clone()],
            }
        }
        core_entities::SessionState::Finished { completed_at, .. } => AgentSessionState::Finished {
            completed_at: *completed_at,
        },
        core_entities::SessionState::Error { error, .. } => AgentSessionState::Error {
            recoverable:
                crate::application::agent::service::AgentServiceImpl::is_recoverable_error_message(
                    error,
                ),
            message: crate::application::agent::service::AgentServiceImpl::strip_recoverable_prefix(
                error,
            ),
        },
        core_entities::SessionState::Cancelled { reason, .. } => AgentSessionState::Cancelled {
            reason: reason.clone(),
        },
    }
}

fn session_state_to_core(
    state: &crate::core::agent::session::SessionState,
) -> core_entities::SessionState {
    use crate::core::agent::session::SessionState as AgentSessionState;

    match state {
        AgentSessionState::Launching => core_entities::SessionState::WaitForRequest {
            last_action_at: chrono::Utc::now(),
        },
        AgentSessionState::Running { .. } => core_entities::SessionState::Processing {
            started_at: chrono::Utc::now(),
        },
        AgentSessionState::WaitForRequest { last_action_at } => {
            core_entities::SessionState::WaitForRequest {
                last_action_at: *last_action_at,
            }
        }
        AgentSessionState::UserInputRequired { pending_action_ids } => {
            core_entities::SessionState::AwaitingApproval {
                action_id: pending_action_ids.first().cloned().unwrap_or_default(),
                requested_at: chrono::Utc::now(),
            }
        }
        AgentSessionState::Finished { completed_at } => core_entities::SessionState::Finished {
            completed_at: *completed_at,
            result: String::new(),
        },
        AgentSessionState::Cancelled { reason } => core_entities::SessionState::Cancelled {
            cancelled_at: chrono::Utc::now(),
            reason: reason.clone(),
        },
        AgentSessionState::Error { message, .. } => core_entities::SessionState::Error {
            error_at: chrono::Utc::now(),
            error: message.clone(),
        },
    }
}

fn session_to_core(
    session: crate::core::agent::session::AgentSession,
) -> core_entities::AgentSession {
    core_entities::AgentSession {
        id: core_entities::SessionId(session.id.0),
        agent_id: core_entities::AgentId(session.agent_id.0),
        agent_name: session.agent_name,
        conversation_id: session.conversation_id,
        thread_id: session.thread_id,
        workspace_root: session.workspace_root,
        state: session_state_to_core(&session.state),
        created_at: session.created_at,
    }
}

fn session_from_core(
    session: &core_entities::AgentSession,
) -> crate::core::agent::session::AgentSession {
    crate::core::agent::session::AgentSession {
        id: crate::core::agent::session::SessionId(session.id.0.clone()),
        agent_id: crate::core::agent::types::AgentId(session.agent_id.0.clone()),
        agent_name: session.agent_name.clone(),
        conversation_id: session.conversation_id.clone(),
        thread_id: session.thread_id.clone(),
        state: session_state_to_agents(&session.state),
        workspace_root: session.workspace_root.clone(),
        parent_session_id: None,
        created_at: session.created_at,
        updated_at: session.created_at,
    }
}

fn task_status_to_agents(
    status: core_entities::TaskStatus,
) -> crate::core::agent::task::TaskStatus {
    match status {
        core_entities::TaskStatus::Pending => crate::core::agent::task::TaskStatus::Pending,
        core_entities::TaskStatus::InProgress => crate::core::agent::task::TaskStatus::InProgress,
        core_entities::TaskStatus::Completed => crate::core::agent::task::TaskStatus::Completed,
        core_entities::TaskStatus::Failed => crate::core::agent::task::TaskStatus::Failed,
        core_entities::TaskStatus::Cancelled => crate::core::agent::task::TaskStatus::Cancelled,
    }
}

fn task_mode_to_agents(mode: core_entities::TaskMode) -> crate::core::agent::task::SubTaskMode {
    match mode {
        core_entities::TaskMode::Read => crate::core::agent::task::SubTaskMode::Read,
        core_entities::TaskMode::Write => crate::core::agent::task::SubTaskMode::Write,
        core_entities::TaskMode::Execute => crate::core::agent::task::SubTaskMode::Execute,
        core_entities::TaskMode::FullAccess => crate::core::agent::task::SubTaskMode::FullAccess,
    }
}

fn task_mode_to_core(mode: crate::core::agent::task::SubTaskMode) -> core_entities::TaskMode {
    match mode {
        crate::core::agent::task::SubTaskMode::Read => core_entities::TaskMode::Read,
        crate::core::agent::task::SubTaskMode::Write => core_entities::TaskMode::Write,
        crate::core::agent::task::SubTaskMode::Execute => core_entities::TaskMode::Execute,
        crate::core::agent::task::SubTaskMode::FullAccess => core_entities::TaskMode::FullAccess,
    }
}

fn task_status_to_core(status: crate::core::agent::task::TaskStatus) -> core_entities::TaskStatus {
    match status {
        crate::core::agent::task::TaskStatus::Pending => core_entities::TaskStatus::Pending,
        crate::core::agent::task::TaskStatus::InProgress => core_entities::TaskStatus::InProgress,
        crate::core::agent::task::TaskStatus::WaitingApproval => core_entities::TaskStatus::Pending,
        crate::core::agent::task::TaskStatus::Completed => core_entities::TaskStatus::Completed,
        crate::core::agent::task::TaskStatus::Failed => core_entities::TaskStatus::Failed,
        crate::core::agent::task::TaskStatus::Cancelled => core_entities::TaskStatus::Cancelled,
    }
}

fn core_task_to_agents(task: &core_entities::AgentTask) -> crate::core::agent::task::AgentTask {
    crate::core::agent::task::AgentTask {
        id: task.id.clone(),
        session_id: crate::core::agent::session::SessionId(task.session_id.clone()),
        request: crate::core::agent::task::AgentRequest {
            content: task.title.clone(),
            attachments: vec![],
            context: task.description.as_ref().map(|description| {
                crate::core::agent::task::RequestContext {
                    file_path: None,
                    selection: Some(description.clone()),
                }
            }),
        },
        status: task_status_to_agents(task.status),
        steps: vec![],
        subtasks: None,
        created_at: chrono::Utc::now(),
        completed_at: None,
    }
}

fn agents_task_to_core(task: &crate::core::agent::task::AgentTask) -> core_entities::AgentTask {
    core_entities::AgentTask {
        id: task.id.clone(),
        session_id: task.session_id.0.clone(),
        parent_task_id: None,
        title: task.request.content.clone(),
        description: task
            .request
            .context
            .as_ref()
            .and_then(|ctx| ctx.selection.clone()),
        status: task_status_to_core(task.status.clone()),
        order: 0,
        mode: None,
    }
}

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
        runtime_data::clear_session_runtime_data(&conn, session_id)
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
}

impl SessionRepository for SqliteSessionRepository {
    fn create_session(&self, session: &core_entities::AgentSession) -> RepositoryResult<()> {
        let internal = session_from_core(session);
        SqliteSessionRepository::create_session(self, &internal).map_err(Into::into)
    }

    fn get_session(
        &self,
        id: &core_entities::SessionId,
    ) -> RepositoryResult<core_entities::AgentSession> {
        let internal_id = crate::core::agent::session::SessionId(id.0.clone());
        let session = SqliteSessionRepository::get_session(self, &internal_id)
            .map_err(core_entities::AgentDomainError::from)?
            .ok_or_else(|| core_entities::AgentDomainError::SessionNotFound { id: id.0.clone() })?;

        Ok(session_to_core(session))
    }

    fn get_sessions_for_conversation(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<core_entities::AgentSession>> {
        let sessions =
            SqliteSessionRepository::get_sessions_for_conversation(self, conversation_id)
                .map_err(core_entities::AgentDomainError::from)?;

        Ok(sessions.into_iter().map(session_to_core).collect())
    }

    fn list_sessions(&self) -> RepositoryResult<Vec<core_entities::AgentSession>> {
        let sessions = SqliteSessionRepository::list_sessions(self)
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
        SqliteSessionRepository::update_state(self, &internal_id, &internal_state)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn update_workspace(
        &self,
        id: &core_entities::SessionId,
        workspace: &str,
    ) -> RepositoryResult<()> {
        let internal_id = crate::core::agent::session::SessionId(id.0.clone());
        SqliteSessionRepository::update_workspace(self, &internal_id, workspace)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn delete_session(&self, id: &core_entities::SessionId) -> RepositoryResult<()> {
        let internal_id = crate::core::agent::session::SessionId(id.0.clone());
        SqliteSessionRepository::delete_session(self, &internal_id)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn record_recovery_event(
        &self,
        session_id: &core_entities::SessionId,
        reason: &str,
        recovered_at: DateTime<Utc>,
    ) -> RepositoryResult<()> {
        let internal_id = crate::core::agent::session::SessionId(session_id.0.clone());
        SqliteSessionRepository::record_recovery_event(self, &internal_id, reason, recovered_at)
            .map_err(core_entities::AgentDomainError::from)
    }
}

impl TaskRepository for SqliteSessionRepository {
    fn create_task(&self, task: &core_entities::AgentTask) -> RepositoryResult<()> {
        let internal = core_task_to_agents(task);
        SqliteSessionRepository::create_task(self, &internal)
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
                .map(task_mode_to_agents)
                .unwrap_or(crate::core::agent::task::SubTaskMode::FullAccess),
            status: task_status_to_agents(task.status),
            order: task.order,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
        };

        SqliteSessionRepository::create_subtask(self, &internal)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn get_session_tasks(
        &self,
        session_id: &str,
    ) -> RepositoryResult<Vec<core_entities::AgentTask>> {
        let internal_id = crate::core::agent::session::SessionId(session_id.to_string());
        let tasks = SqliteSessionRepository::get_session_tasks(self, &internal_id)
            .map_err(core_entities::AgentDomainError::from)?;

        Ok(tasks.iter().map(agents_task_to_core).collect())
    }

    fn get_subtasks(
        &self,
        parent_task_id: &str,
    ) -> RepositoryResult<Vec<core_entities::AgentTask>> {
        let subtasks = SqliteSessionRepository::get_subtasks(self, parent_task_id)
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
            })
            .collect())
    }

    fn update_task_status(
        &self,
        task_id: &str,
        status: core_entities::TaskStatus,
    ) -> RepositoryResult<()> {
        let internal_status = task_status_to_agents(status);
        SqliteSessionRepository::update_task_status(self, task_id, &internal_status)
            .map_err(core_entities::AgentDomainError::from)
    }

    fn update_subtask_status(
        &self,
        subtask_id: &str,
        status: core_entities::TaskStatus,
    ) -> RepositoryResult<()> {
        let internal_status = task_status_to_agents(status);
        SqliteSessionRepository::update_subtask_status(self, subtask_id, &internal_status)
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
}
