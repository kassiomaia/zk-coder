//! Runtime telemetry and read-model access for agent sessions.
//!
//! This is not core domain persistence. It covers execution steps, terminal
//! output, touched files, status phrases, tool calls, and session read access
//! needed by the runtime and command layer.
//!
//! The concrete SQLite implementation lives in
//! [`crate::infrastructure::sqlite::repositories::agent::SqliteAgentTelemetryRepository`].

use crate::core::agent::session::{AgentSession, SessionId};
use crate::core::agent::task::ExecutionStep;
use crate::infrastructure::sqlite::repositories::agent::{
    SessionRepositoryError, TerminalLineEntry, ToolCallEntry, TouchedFileEntry,
};

/// Runtime telemetry access required by the agent layer.
pub trait AgentTelemetryRepository: Send + Sync {
    /// Create an execution step record.
    fn create_execution_step(
        &self,
        step: &ExecutionStep,
        session_id: &str,
        task_id: Option<&str>,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError>;

    /// Get a session by ID.
    fn get_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<AgentSession>, SessionRepositoryError>;

    /// Get execution steps for a session.
    fn get_session_execution_steps(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ExecutionStep>, SessionRepositoryError>;

    /// Record a file touched event.
    fn add_file_touched(
        &self,
        session_id: &SessionId,
        file_path: &str,
        action: &str,
        diff: Option<&str>,
    ) -> Result<(), SessionRepositoryError>;

    /// Get files touched in a session.
    fn get_session_files(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TouchedFileEntry>, SessionRepositoryError>;

    /// Add a terminal output line.
    fn add_terminal_line(
        &self,
        session_id: &SessionId,
        line_type: &str,
        content: &str,
    ) -> Result<(), SessionRepositoryError>;

    /// Get terminal output for a session.
    fn get_session_terminal_output(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<TerminalLineEntry>, SessionRepositoryError>;

    /// Add a status phrase.
    fn add_status_phrase(
        &self,
        session_id: &SessionId,
        phrase: &str,
    ) -> Result<(), SessionRepositoryError>;

    /// Get status phrases for a session.
    fn get_session_status_phrases(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<String>, SessionRepositoryError>;

    /// Clear runtime data for a session.
    fn clear_session_runtime_data(
        &self,
        session_id: &SessionId,
    ) -> Result<(), SessionRepositoryError>;

    /// Insert a tool call record.
    fn insert_tool_call(
        &self,
        session_id: &SessionId,
        tool_call_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionRepositoryError>;

    /// Update tool call result.
    fn update_tool_call_result(
        &self,
        tool_call_id: &str,
        result: &serde_json::Value,
        is_error: bool,
        duration_ms: Option<i64>,
    ) -> Result<(), SessionRepositoryError>;

    /// Get tool calls for a session.
    fn get_tool_calls_for_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ToolCallEntry>, SessionRepositoryError>;
}
