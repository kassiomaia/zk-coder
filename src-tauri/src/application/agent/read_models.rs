//! Application-facing read models for the agent backend.
//!
//! These structs aggregate runtime data needed by the Tauri command layer so
//! IPC handlers can depend on a stable application surface instead of reaching
//! directly into runtime and repository internals.

use serde::Serialize;

use crate::core::agent::session::{AgentSession, PendingAction};
use crate::core::agent::task::AgentTask;
use crate::core::agent::types::Agent;

/// Rich runtime view of a session for frontend inspection.
#[derive(Debug, Clone)]
pub struct AgentSessionRuntimeDetails {
    pub session: AgentSession,
    pub files_touched: Vec<FileTouchedView>,
    pub terminal_output: Vec<TerminalLineView>,
    pub execution_steps: Vec<serde_json::Value>,
    pub status_phrases: Vec<String>,
    pub tasks: Vec<AgentTask>,
    pub tool_calls: Vec<ToolCallView>,
}

/// Prepared execution context for an agent request.
#[derive(Debug, Clone)]
pub struct AgentExecutionContext {
    pub session: AgentSession,
    pub system_prompt: String,
    pub trust_mode: crate::core::agent::types::TrustMode,
}

/// Runtime read model used by approval commands.
#[derive(Debug, Clone)]
pub struct PendingActionsView {
    pub actions: Vec<PendingAction>,
}

/// Runtime read model used by registry commands.
#[derive(Debug, Clone)]
pub struct AgentCatalog {
    pub agents: Vec<Agent>,
}

/// Public view of a touched file in a session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTouchedView {
    pub path: String,
    pub modified: bool,
    pub action: String,
    pub diff: Option<String>,
}

/// Public view of a terminal output line in a session.
#[derive(Debug, Clone, Serialize)]
pub struct TerminalLineView {
    pub id: String,
    #[serde(rename = "type")]
    pub line_type: String,
    pub content: String,
    pub timestamp: String,
}

/// Public view of a recorded tool call in a session.
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallView {
    pub id: String,
    pub session_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub is_error: bool,
    pub duration_ms: Option<i64>,
    pub subtask_id: Option<String>,
}
