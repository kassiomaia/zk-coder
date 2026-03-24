//! Agent event types for frontend communication.
//!
//! Events are emitted via Tauri's event system so the frontend
//! can reactively update the agent panel UI.

use serde::{Deserialize, Serialize};

use super::session::{PendingAction, SessionState};
use super::task::{AgentExecutionResult, AgentTask, ExecutionStep, SubTask, TaskStatus};
use chrono::{DateTime, Utc};

/// Events emitted by the agent system to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A new session was created.
    SessionCreated {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "agentId")]
        agent_id: String,
        agent_name: String,
    },

    /// Session state changed.
    SessionStateChanged {
        #[serde(rename = "sessionId")]
        session_id: String,
        state: SessionState,
    },

    /// Agent started executing a request.
    ExecutionStarted {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "requestId")]
        request_id: String,
    },

    /// Agent completed one execution step.
    ExecutionStep {
        #[serde(rename = "sessionId")]
        session_id: String,
        step: ExecutionStep,
    },

    /// Agent finished executing a request.
    ExecutionComplete {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "conversationId")]
        conversation_id: String,
        #[serde(rename = "threadId")]
        thread_id: String,
        result: AgentExecutionResult,
    },

    /// Agent execution encountered an error.
    ExecutionError {
        #[serde(rename = "sessionId")]
        session_id: String,
        error: String,
    },

    /// An action requires user approval.
    ApprovalRequired {
        #[serde(rename = "sessionId")]
        session_id: String,
        action: PendingAction,
    },

    /// An action was approved by the user.
    ApprovalApproved {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "actionId")]
        action_id: String,
    },

    /// An action was denied by the user.
    ApprovalDenied {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "actionId")]
        action_id: String,
    },

    /// A tool was called.
    ToolCall {
        #[serde(rename = "sessionId")]
        session_id: String,
        tool: String,
        arguments: serde_json::Value,
    },

    /// A tool returned a result.
    ToolResult {
        #[serde(rename = "sessionId")]
        session_id: String,
        tool: String,
        result: serde_json::Value,
        is_error: bool,
    },

    /// Command output chunk (streamed during execution).
    CommandOutput {
        #[serde(rename = "sessionId")]
        session_id: String,
        /// "stdout" or "stderr"
        stream: String,
        content: String,
    },

    /// Command execution finished.
    CommandFinished {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "exitCode")]
        exit_code: Option<i32>,
    },

    /// A task was created for this session.
    TaskCreated {
        #[serde(rename = "sessionId")]
        session_id: String,
        task: AgentTask,
    },

    /// A task status was updated.
    TaskUpdated {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "taskId")]
        task_id: String,
        status: TaskStatus,
    },

    /// A subtask was created.
    SubtaskCreated {
        #[serde(rename = "sessionId")]
        session_id: String,
        subtask: SubTask,
    },

    /// A subtask was updated.
    SubtaskUpdated {
        #[serde(rename = "sessionId")]
        session_id: String,
        subtask: SubTask,
    },

    /// A file was touched (read, written, or deleted) during execution.
    FileTouched {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "filePath")]
        file_path: String,
        #[serde(rename = "action")]
        action: FileAction,
        /// Unified diff of changes (only present for write actions)
        #[serde(rename = "diff", skip_serializing_if = "Option::is_none")]
        diff: Option<String>,
    },

    /// Agent emitted a status phrase for UI display.
    StatusPhrase {
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The status message to display
        message: String,
        /// Timestamp when emitted
        timestamp: DateTime<Utc>,
    },

    /// Content streaming chunk for the final response.
    StreamingContent {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "conversationId")]
        conversation_id: String,
        #[serde(rename = "threadId")]
        thread_id: String,
        /// Content chunk (partial text)
        chunk: String,
        /// Whether this is the final chunk
        #[serde(rename = "isFinal")]
        is_final: bool,
    },

    /// Context was compacted to stay within token limits.
    ContextCompacted {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "conversationId")]
        conversation_id: String,
        /// Number of messages before compaction
        #[serde(rename = "beforeCount")]
        before_count: usize,
        /// Number of messages after compaction
        #[serde(rename = "afterCount")]
        after_count: usize,
        /// Estimated tokens before compaction
        #[serde(rename = "beforeTokens")]
        before_tokens: usize,
        /// Estimated tokens after compaction
        #[serde(rename = "afterTokens")]
        after_tokens: usize,
        /// Timestamp when compacted
        timestamp: DateTime<Utc>,
    },
}

/// Action performed on a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Read,
    Write,
    Delete,
}

/// Tauri event name for agent events.
pub const AGENT_EVENT_NAME: &str = "agent:event";
