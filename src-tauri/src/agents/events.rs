//! Agent event types for frontend communication.
//!
//! Events are emitted via Tauri's event system so the frontend
//! can reactively update the agent panel UI.

use serde::{Deserialize, Serialize};

use super::session::{PendingAction, SessionState};
use super::task::{AgentExecutionResult, ExecutionStep};

/// Events emitted by the agent system to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A new session was created.
    SessionCreated {
        session_id: String,
        agent_id: String,
        agent_name: String,
    },

    /// Session state changed.
    SessionStateChanged {
        session_id: String,
        state: SessionState,
    },

    /// Agent started executing a request.
    ExecutionStarted {
        session_id: String,
        request_id: String,
    },

    /// Agent completed one execution step.
    ExecutionStep {
        session_id: String,
        step: ExecutionStep,
    },

    /// Agent finished executing a request.
    ExecutionComplete {
        session_id: String,
        result: AgentExecutionResult,
    },

    /// Agent execution encountered an error.
    ExecutionError {
        session_id: String,
        error: String,
    },

    /// An action requires user approval.
    ApprovalRequired {
        session_id: String,
        action: PendingAction,
    },

    /// An action was approved by the user.
    ApprovalApproved {
        session_id: String,
        action_id: String,
    },

    /// An action was denied by the user.
    ApprovalDenied {
        session_id: String,
        action_id: String,
    },

    /// A tool was called.
    ToolCall {
        session_id: String,
        tool: String,
        arguments: serde_json::Value,
    },

    /// A tool returned a result.
    ToolResult {
        session_id: String,
        tool: String,
        result: serde_json::Value,
        is_error: bool,
    },

    /// Command output chunk (streamed during execution).
    CommandOutput {
        session_id: String,
        /// "stdout" or "stderr"
        stream: String,
        content: String,
    },

    /// Command execution finished.
    CommandFinished {
        session_id: String,
        exit_code: Option<i32>,
    },
}

/// Tauri event name for agent events.
pub const AGENT_EVENT_NAME: &str = "agent:event";
