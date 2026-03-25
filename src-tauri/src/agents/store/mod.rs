//! SQLite storage for agent sessions and tasks.
//!
//! Persists agent sessions across app restarts so users can
//! resume incomplete tasks.

use std::sync::Mutex;

use rusqlite::Connection;

pub mod schema;
pub mod sessions;
pub mod tasks;
pub mod runtime_data;
pub mod tool_calls;

pub use runtime_data::{TouchedFileEntry, TerminalLineEntry};
pub use tool_calls::ToolCallEntry;

/// Errors from the session store.
#[derive(Debug, thiserror::Error)]
pub enum SessionStoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Lock error: {0}")]
    LockError(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// SQLite-based session store.
pub struct SessionStore {
    pub(crate) conn: Mutex<Connection>,
}

impl SessionStore {
    /// Create a new session store at the given database path.
    pub fn new(db_path: &str) -> Result<Self, SessionStoreError> {
        let conn = Connection::open(db_path)?;
        schema::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create a new session store at the given database path.
    pub fn from_connection(conn: Connection) -> Result<Self, SessionStoreError> {
        schema::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ========================================================================
    // Session methods (delegated to sessions module)
    // ========================================================================

    /// Save a new session.
    pub fn create_session(&self, session: &crate::agents::session::AgentSession) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        sessions::create_session(&conn, session)
    }

    /// Load a session by ID.
    pub fn get_session(&self, id: &crate::agents::session::SessionId) -> Result<Option<crate::agents::session::AgentSession>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        sessions::get_session(&conn, id)
    }

    /// Update session state.
    pub fn update_state(
        &self,
        id: &crate::agents::session::SessionId,
        state: &crate::agents::session::SessionState,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        sessions::update_state(&conn, id, state)
    }

    /// Get all sessions for a conversation.
    pub fn get_sessions_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<crate::agents::session::AgentSession>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        sessions::get_sessions_for_conversation(&conn, conversation_id)
    }

    /// Delete a session.
    pub fn delete_session(&self, id: &crate::agents::session::SessionId) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        sessions::delete_session(&conn, id)
    }

    /// Update workspace root for a session.
    pub fn update_workspace(&self, id: &crate::agents::session::SessionId, workspace_root: &str) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        sessions::update_workspace(&conn, id, workspace_root)
    }

    // ========================================================================
    // Task methods (delegated to tasks module)
    // ========================================================================

    /// Create a new agent task.
    pub fn create_task(
        &self,
        task: &crate::agents::task::AgentTask,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::create_task(&conn, task)
    }

    /// Update task status.
    pub fn update_task_status(
        &self,
        task_id: &str,
        status: &crate::agents::task::TaskStatus,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::update_task_status(&conn, task_id, status)
    }

    /// Get all tasks for a session.
    pub fn get_session_tasks(
        &self,
        session_id: &crate::agents::session::SessionId,
    ) -> Result<Vec<crate::agents::task::AgentTask>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::get_session_tasks(&conn, session_id)
    }

    /// Create a new subtask.
    pub fn create_subtask(
        &self,
        subtask: &crate::agents::task::SubTask,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::create_subtask(&conn, subtask)
    }

    /// Update subtask status.
    pub fn update_subtask_status(
        &self,
        subtask_id: &str,
        status: &crate::agents::task::TaskStatus,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::update_subtask_status(&conn, subtask_id, status)
    }

    /// Get all subtasks for a task.
    pub fn get_subtasks(&self, task_id: &str) -> Result<Vec<crate::agents::task::SubTask>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::get_subtasks(&conn, task_id)
    }

    /// Delete all subtasks for a task.
    pub fn delete_subtasks(&self, task_id: &str) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::delete_subtasks(&conn, task_id)
    }

    /// Create a new execution step record.
    pub fn create_execution_step(
        &self,
        step: &crate::agents::task::ExecutionStep,
        session_id: &str,
        task_id: Option<&str>,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::create_execution_step(&conn, step, session_id, task_id, subtask_id)
    }

    /// Get recent execution steps for a session.
    pub fn get_recent_execution_steps(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::agents::task::ExecutionStep>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::get_recent_execution_steps(&conn, session_id, limit)
    }

    /// Get all execution steps for a session in chronological order.
    pub fn get_session_execution_steps(
        &self,
        session_id: &crate::agents::session::SessionId,
    ) -> Result<Vec<crate::agents::task::ExecutionStep>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tasks::get_session_execution_steps(&conn, session_id)
    }

    // ========================================================================
    // Runtime data methods (delegated to runtime_data module)
    // ========================================================================

    /// Add a file operation record.
    pub fn add_file_touched(
        &self,
        session_id: &crate::agents::session::SessionId,
        file_path: &str,
        action: &str,
        diff: Option<&str>,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        runtime_data::add_file_touched(&conn, session_id, file_path, action, diff)
    }

    /// Get all files touched for a session.
    pub fn get_session_files(
        &self,
        session_id: &crate::agents::session::SessionId,
    ) -> Result<Vec<TouchedFileEntry>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        runtime_data::get_session_files(&conn, session_id)
    }

    /// Add a terminal output line.
    pub fn add_terminal_line(
        &self,
        session_id: &crate::agents::session::SessionId,
        line_type: &str,
        content: &str,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        runtime_data::add_terminal_line(&conn, session_id, line_type, content)
    }

    /// Get all terminal output for a session.
    pub fn get_session_terminal_output(
        &self,
        session_id: &crate::agents::session::SessionId,
    ) -> Result<Vec<TerminalLineEntry>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        runtime_data::get_session_terminal_output(&conn, session_id)
    }

    /// Add a status phrase.
    pub fn add_status_phrase(
        &self,
        session_id: &crate::agents::session::SessionId,
        phrase: &str,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        runtime_data::add_status_phrase(&conn, session_id, phrase)
    }

    /// Get all status phrases for a session.
    pub fn get_session_status_phrases(
        &self,
        session_id: &crate::agents::session::SessionId,
    ) -> Result<Vec<String>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        runtime_data::get_session_status_phrases(&conn, session_id)
    }

    /// Clear all runtime data for a session.
    pub fn clear_session_runtime_data(
        &self,
        session_id: &crate::agents::session::SessionId,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        runtime_data::clear_session_runtime_data(&conn, session_id)
    }

    // ========================================================================
    // Tool call methods (delegated to tool_calls module)
    // ========================================================================

    /// Insert a tool call record.
    pub fn insert_tool_call(
        &self,
        session_id: &crate::agents::session::SessionId,
        tool_call_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tool_calls::insert_tool_call(&conn, session_id, tool_call_id, tool_name, arguments, subtask_id)
    }

    /// Update tool call result.
    pub fn update_tool_call_result(
        &self,
        tool_call_id: &str,
        result: &serde_json::Value,
        is_error: bool,
        duration_ms: Option<i64>,
    ) -> Result<(), SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tool_calls::update_tool_call_result(&conn, tool_call_id, result, is_error, duration_ms)
    }

    /// Get all tool calls for a session.
    pub fn get_tool_calls_for_session(
        &self,
        session_id: &crate::agents::session::SessionId,
    ) -> Result<Vec<ToolCallEntry>, SessionStoreError> {
        let conn = self.conn.lock().map_err(|e| SessionStoreError::LockError(e.to_string()))?;
        tool_calls::get_tool_calls_for_session(&conn, session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> crate::agents::session::AgentSession {
        crate::agents::session::AgentSession::new(
            crate::agents::types::AgentId::new("code-agent"),
            "Code Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
        )
    }

    #[test]
    fn test_session_store_integration() {
        let store = SessionStore::new(":memory:").unwrap();
        let session = make_session();

        store.create_session(&session).unwrap();
        let loaded = store.get_session(&session.id).unwrap();

        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.agent_id.0, "code-agent");
        assert_eq!(loaded.conversation_id, "conv-1");
    }

    #[test]
    fn test_task_integration() {
        let store = SessionStore::new(":memory:").unwrap();
        let session = make_session();
        store.create_session(&session).unwrap();

        let task = crate::agents::task::AgentTask::new(session.id.clone(), crate::agents::task::AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        });
        store.create_task(&task).unwrap();

        let tasks = store.get_session_tasks(&session.id).unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn test_subtask_integration() {
        let store = SessionStore::new(":memory:").unwrap();
        let session = make_session();
        store.create_session(&session).unwrap();

        let task = crate::agents::task::AgentTask::new(session.id.clone(), crate::agents::task::AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        });
        store.create_task(&task).unwrap();

        let subtask = crate::agents::task::SubTask::new(
            task.id.clone(),
            "Test Subtask".to_string(),
            crate::agents::task::SubTaskMode::Read,
            0,
        );
        store.create_subtask(&subtask).unwrap();

        let subtasks = store.get_subtasks(&task.id).unwrap();
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0].title, "Test Subtask");
    }

    #[test]
    fn test_runtime_data_integration() {
        let store = SessionStore::new(":memory:").unwrap();
        let session = make_session();
        store.create_session(&session).unwrap();

        store.add_file_touched(&session.id, "/test/file.txt", "write", Some("diff")).unwrap();
        store.add_terminal_line(&session.id, "stdout", "output").unwrap();
        store.add_status_phrase(&session.id, "Working...").unwrap();

        let files = store.get_session_files(&session.id).unwrap();
        assert_eq!(files.len(), 1);

        let terminal = store.get_session_terminal_output(&session.id).unwrap();
        assert_eq!(terminal.len(), 1);

        let phrases = store.get_session_status_phrases(&session.id).unwrap();
        assert_eq!(phrases.len(), 1);
    }

    #[test]
    fn test_tool_call_integration() {
        let store = SessionStore::new(":memory:").unwrap();
        let session = make_session();
        store.create_session(&session).unwrap();

        store.insert_tool_call(
            &session.id,
            "tool-1",
            "fs_read_file",
            &serde_json::json!({"path": "/test"}),
            None,
        ).unwrap();

        let tool_calls = store.get_tool_calls_for_session(&session.id).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].tool_name, "fs_read_file");
    }
}
