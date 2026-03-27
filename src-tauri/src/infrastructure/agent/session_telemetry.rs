//! Runtime telemetry and read-model access for agent sessions.
//!
//! This is not core domain persistence. It covers execution steps, terminal
//! output, touched files, status phrases, tool calls, and session read access
//! needed by the runtime and command layer.

use crate::core::agent::session::{AgentSession, SessionId};
use crate::core::agent::task::ExecutionStep;
use crate::infrastructure::sqlite::repositories::agent::{
    SessionRepositoryError, SqliteSessionRepository, TerminalLineEntry, ToolCallEntry,
    TouchedFileEntry,
};

/// Runtime telemetry access required by the agent layer.
pub trait AgentTelemetryRepository: Send + Sync {
    fn create_execution_step(
        &self,
        step: &ExecutionStep,
        session_id: &str,
        task_id: Option<&str>,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError>;

    fn get_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<AgentSession>, SessionRepositoryError>;

    fn get_session_execution_steps(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ExecutionStep>, SessionRepositoryError>;

    fn add_file_touched(
        &self,
        session_id: &SessionId,
        file_path: &str,
        action: &str,
        diff: Option<&str>,
    ) -> Result<(), SessionRepositoryError>;

    fn get_session_files(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TouchedFileEntry>, SessionRepositoryError>;

    fn add_terminal_line(
        &self,
        session_id: &SessionId,
        line_type: &str,
        content: &str,
    ) -> Result<(), SessionRepositoryError>;

    fn get_session_terminal_output(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TerminalLineEntry>, SessionRepositoryError>;

    fn add_status_phrase(
        &self,
        session_id: &SessionId,
        phrase: &str,
    ) -> Result<(), SessionRepositoryError>;

    fn get_session_status_phrases(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<String>, SessionRepositoryError>;

    fn clear_session_runtime_data(
        &self,
        session_id: &SessionId,
    ) -> Result<(), SessionRepositoryError>;

    fn insert_tool_call(
        &self,
        session_id: &SessionId,
        tool_call_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError>;

    fn update_tool_call_result(
        &self,
        tool_call_id: &str,
        result: &serde_json::Value,
        is_error: bool,
        duration_ms: Option<i64>,
    ) -> Result<(), SessionRepositoryError>;

    fn get_tool_calls_for_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ToolCallEntry>, SessionRepositoryError>;
}

impl AgentTelemetryRepository for SqliteSessionRepository {
    fn create_execution_step(
        &self,
        step: &ExecutionStep,
        session_id: &str,
        task_id: Option<&str>,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        SqliteSessionRepository::create_execution_step(self, step, session_id, task_id, subtask_id)
    }

    fn get_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<AgentSession>, SessionRepositoryError> {
        SqliteSessionRepository::get_session(self, session_id)
    }

    fn get_session_execution_steps(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ExecutionStep>, SessionRepositoryError> {
        SqliteSessionRepository::get_session_execution_steps(self, session_id)
    }

    fn add_file_touched(
        &self,
        session_id: &SessionId,
        file_path: &str,
        action: &str,
        diff: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        SqliteSessionRepository::add_file_touched(self, session_id, file_path, action, diff)
    }

    fn get_session_files(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TouchedFileEntry>, SessionRepositoryError> {
        SqliteSessionRepository::get_session_files(self, session_id)
    }

    fn add_terminal_line(
        &self,
        session_id: &SessionId,
        line_type: &str,
        content: &str,
    ) -> Result<(), SessionRepositoryError> {
        SqliteSessionRepository::add_terminal_line(self, session_id, line_type, content)
    }

    fn get_session_terminal_output(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TerminalLineEntry>, SessionRepositoryError> {
        SqliteSessionRepository::get_session_terminal_output(self, session_id)
    }

    fn add_status_phrase(
        &self,
        session_id: &SessionId,
        phrase: &str,
    ) -> Result<(), SessionRepositoryError> {
        SqliteSessionRepository::add_status_phrase(self, session_id, phrase)
    }

    fn get_session_status_phrases(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<String>, SessionRepositoryError> {
        SqliteSessionRepository::get_session_status_phrases(self, session_id)
    }

    fn clear_session_runtime_data(
        &self,
        session_id: &SessionId,
    ) -> Result<(), SessionRepositoryError> {
        SqliteSessionRepository::clear_session_runtime_data(self, session_id)
    }

    fn insert_tool_call(
        &self,
        session_id: &SessionId,
        tool_call_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError> {
        SqliteSessionRepository::insert_tool_call(
            self,
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
        SqliteSessionRepository::update_tool_call_result(
            self,
            tool_call_id,
            result,
            is_error,
            duration_ms,
        )
    }

    fn get_tool_calls_for_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ToolCallEntry>, SessionRepositoryError> {
        SqliteSessionRepository::get_tool_calls_for_session(self, session_id)
    }
}
