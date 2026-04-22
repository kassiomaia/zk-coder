//! SQLite-backed [`AgentTelemetryRepository`] implementation.
//!
//! Shares a single `Arc<Mutex<Connection>>` with the other agent
//! domain repositories. Delegates to the free functions in the
//! `runtime_data`, `tool_calls`, `sessions`, and `tasks` submodules.
//!
//! Introduced in the Phase 6 SOLID refactor. The trait itself still
//! lives in `infrastructure::agent::session_telemetry` — moving it
//! would ripple through 15+ call sites with no architectural benefit
//! for this phase.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::core::agent::session::{AgentSession, SessionId};
use crate::core::agent::task::ExecutionStep;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;

use super::{
    execution_debug, runtime_data, sessions, tasks, tool_calls, SessionRepositoryError,
    TerminalLineEntry, ToolCallEntry, TouchedFileEntry,
};

/// SQLite-backed implementation of [`AgentTelemetryRepository`].
pub struct SqliteAgentTelemetryRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteAgentTelemetryRepository {
    /// Builds the repository from a shared connection. The schema must
    /// already have been applied via `migrations::init_all`.
    #[must_use]
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, SessionRepositoryError> {
        self.conn
            .lock()
            .map_err(|e| SessionRepositoryError::LockError(e.to_string()))
    }
}

impl AgentTelemetryRepository for SqliteAgentTelemetryRepository {
    fn create_execution_step(
        &self,
        step: &ExecutionStep,
        session_id: &str,
        task_id: Option<&str>,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self.lock_conn()?;
        tasks::create_execution_step(&conn, step, session_id, task_id, subtask_id)
    }

    fn get_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<AgentSession>, SessionRepositoryError> {
        let conn = self.lock_conn()?;
        sessions::get_session(&conn, session_id)
    }

    fn get_session_execution_steps(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ExecutionStep>, SessionRepositoryError> {
        let conn = self.lock_conn()?;
        tasks::get_session_execution_steps(&conn, session_id)
    }

    fn add_file_touched(
        &self,
        session_id: &SessionId,
        file_path: &str,
        action: &str,
        diff: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self.lock_conn()?;
        runtime_data::add_file_touched(&conn, session_id, file_path, action, diff)
    }

    fn get_session_files(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TouchedFileEntry>, SessionRepositoryError> {
        let conn = self.lock_conn()?;
        runtime_data::get_session_files(&conn, session_id)
    }

    fn add_terminal_line(
        &self,
        session_id: &SessionId,
        line_type: &str,
        content: &str,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self.lock_conn()?;
        runtime_data::add_terminal_line(&conn, session_id, line_type, content)
    }

    fn get_session_terminal_output(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TerminalLineEntry>, SessionRepositoryError> {
        let conn = self.lock_conn()?;
        runtime_data::get_session_terminal_output(&conn, session_id)
    }

    fn add_status_phrase(
        &self,
        session_id: &SessionId,
        phrase: &str,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self.lock_conn()?;
        runtime_data::add_status_phrase(&conn, session_id, phrase)
    }

    fn get_session_status_phrases(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<String>, SessionRepositoryError> {
        let conn = self.lock_conn()?;
        runtime_data::get_session_status_phrases(&conn, session_id)
    }

    fn clear_session_runtime_data(
        &self,
        session_id: &SessionId,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self.lock_conn()?;
        runtime_data::clear_session_runtime_data(&conn, session_id)?;
        tasks::clear_session_execution_steps(&conn, session_id)?;
        execution_debug::clear_execution_snapshot(&conn, &session_id.0)?;
        Ok(())
    }

    fn insert_tool_call(
        &self,
        session_id: &SessionId,
        tool_call_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self.lock_conn()?;
        tool_calls::insert_tool_call(
            &conn,
            session_id,
            tool_call_id,
            tool_name,
            arguments,
            subtask_id,
        )
    }

    fn update_tool_call_result(
        &self,
        tool_call_id: &str,
        result: &serde_json::Value,
        is_error: bool,
        duration_ms: Option<i64>,
    ) -> Result<(), SessionRepositoryError> {
        let conn = self.lock_conn()?;
        tool_calls::update_tool_call_result(&conn, tool_call_id, result, is_error, duration_ms)
    }

    fn get_tool_calls_for_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ToolCallEntry>, SessionRepositoryError> {
        let conn = self.lock_conn()?;
        tool_calls::get_tool_calls_for_session(&conn, session_id)
    }
}
