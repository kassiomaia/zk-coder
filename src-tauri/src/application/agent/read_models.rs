//! Application-facing read models for the agent backend.
//!
//! These structs aggregate runtime data needed by the Tauri command layer so
//! IPC handlers can depend on a stable application surface instead of reaching
//! directly into runtime and repository internals.

use serde::Serialize;

use crate::core::agent::execution::{ExecutionSnapshot, RuntimeTrace};
use crate::core::agent::session::{AgentSession, PendingAction};
use crate::core::agent::task::AgentTask;
use crate::core::agent::types::Agent;

/// Rich runtime view of a session for frontend inspection.
#[derive(Debug, Clone)]
pub struct AgentSessionRuntimeDetails {
    /// The agent session data.
    pub session: AgentSession,
    /// Files that were touched during the session.
    pub files_touched: Vec<FileTouchedView>,
    /// Terminal output lines captured during execution.
    pub terminal_output: Vec<TerminalLineView>,
    /// Execution steps recorded during the session.
    pub execution_steps: Vec<serde_json::Value>,
    /// Status phrases reported during execution.
    pub status_phrases: Vec<String>,
    /// Tasks associated with this session.
    pub tasks: Vec<AgentTask>,
    /// Tool calls made during the session.
    pub tool_calls: Vec<ToolCallView>,
}

/// Deep execution debug data used by the development trace window.
#[derive(Debug, Clone)]
pub struct AgentExecutionDebugDetails {
    /// Snapshot of the execution state at a point in time.
    pub snapshot: Option<ExecutionSnapshot>,
    /// Runtime traces collected during execution.
    pub traces: Vec<RuntimeTrace>,
}

/// Prepared execution context for an agent request.
#[derive(Debug, Clone)]
pub struct AgentExecutionContext {
    /// The agent session.
    pub session: AgentSession,
    /// The system prompt for the agent.
    pub system_prompt: String,
    /// The trust mode for execution.
    pub trust_mode: crate::core::agent::types::TrustMode,
}

/// Runtime read model used by approval commands.
#[derive(Debug, Clone)]
pub struct PendingActionsView {
    /// List of pending actions awaiting approval.
    pub actions: Vec<PendingAction>,
}

/// Runtime read model used by registry commands.
#[derive(Debug, Clone)]
pub struct AgentCatalog {
    /// List of available agents.
    pub agents: Vec<Agent>,
}

/// Public view of a touched file in a session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTouchedView {
    /// Path to the file.
    pub path: String,
    /// Whether the file was modified (vs just read).
    pub modified: bool,
    /// The action performed on the file.
    pub action: String,
    /// The diff of changes made (if available).
    pub diff: Option<String>,
}

/// Public view of a terminal output line in a session.
#[derive(Debug, Clone, Serialize)]
pub struct TerminalLineView {
    /// Unique identifier for this line.
    pub id: String,
    /// The type of line (e.g., "stdout", "stderr").
    #[serde(rename = "type")]
    pub line_type: String,
    /// The content of the line.
    pub content: String,
    /// Timestamp when this line was generated.
    pub timestamp: String,
}

/// Public view of a recorded tool call in a session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallView {
    /// Unique identifier for this tool call.
    pub id: String,
    /// The session this tool call belongs to.
    pub session_id: String,
    /// The name of the tool that was called.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub arguments: serde_json::Value,
    /// Result returned by the tool (if completed).
    pub result: Option<serde_json::Value>,
    /// Whether the tool call resulted in an error.
    pub is_error: bool,
    /// Duration of the tool call in milliseconds.
    pub duration_ms: Option<i64>,
    /// The subtask ID this tool call belongs to (if any).
    pub subtask_id: Option<String>,
    /// Timestamp when this tool call was recorded.
    pub timestamp: String,
}
