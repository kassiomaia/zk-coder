//! Agent event types for frontend communication.
//!
//! Events are emitted via Tauri's event system so the frontend
//! can reactively update the agent panel UI.

use serde::{Deserialize, Serialize};

use crate::core::agent::session::{PendingAction, SessionState};
use crate::core::agent::task::{
    AgentExecutionResult, AgentTask, ExecutionStep, SubTask, TaskStatus,
};
use crate::core::agent::{ExecutionSnapshot, RuntimeTrace};
use chrono::{DateTime, Utc};

/// Events emitted by the agent system to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A new session was created.
    SessionCreated {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The agent ID.
        #[serde(rename = "agentId")]
        agent_id: String,
        /// The agent display name.
        agent_name: String,
    },

    /// Session state changed.
    SessionStateChanged {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The new session state.
        state: SessionState,
    },

    /// A session was paused and needs user decision to continue.
    SessionPaused {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// Friendly message explaining the state.
        message: String,
        /// Timestamp when paused.
        #[serde(rename = "pausedAt")]
        paused_at: DateTime<Utc>,
        /// Current subtask being worked on (if any).
        #[serde(rename = "currentSubtaskId", skip_serializing_if = "Option::is_none")]
        current_subtask_id: Option<String>,
    },

    /// User chose to continue from paused state.
    RecoveryContinue {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
    },

    /// User chose to replan from paused state.
    RecoveryReplan {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
    },

    /// Agent started executing a request.
    ExecutionStarted {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The request ID.
        #[serde(rename = "requestId")]
        request_id: String,
    },

    /// Agent completed one execution step.
    ExecutionStep {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The execution step.
        step: ExecutionStep,
    },

    /// Agent finished executing a request.
    ExecutionComplete {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The conversation ID.
        #[serde(rename = "conversationId")]
        conversation_id: String,
        /// The thread ID.
        #[serde(rename = "threadId")]
        thread_id: String,
        /// The execution result.
        result: AgentExecutionResult,
    },

    /// Agent execution encountered an error.
    ExecutionError {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The error message.
        error: String,
    },

    /// An action requires user approval.
    ApprovalRequired {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The pending action.
        action: PendingAction,
    },

    /// An action was approved by the user.
    ApprovalApproved {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The action ID.
        #[serde(rename = "actionId")]
        action_id: String,
    },

    /// An action was denied by the user.
    ApprovalDenied {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The action ID.
        #[serde(rename = "actionId")]
        action_id: String,
    },

    /// A tool was called.
    ToolCall {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The tool call ID.
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        /// The tool name.
        tool: String,
        /// The tool arguments.
        arguments: serde_json::Value,
        /// Optional content from the tool call.
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
    },

    /// A tool returned a result.
    ToolResult {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The tool call ID.
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        /// The tool name.
        tool: String,
        /// The tool result.
        result: serde_json::Value,
        /// Whether the result is an error.
        is_error: bool,
        /// Optional content from the result.
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
    },

    /// Command output chunk (streamed during execution).
    CommandOutput {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// "stdout" or "stderr".
        stream: String,
        /// The output content.
        content: String,
    },

    /// Command execution finished.
    CommandFinished {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The exit code.
        #[serde(rename = "exitCode")]
        exit_code: Option<i32>,
    },

    /// A task was created for this session.
    TaskCreated {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The created task.
        task: AgentTask,
    },

    /// A task status was updated.
    TaskUpdated {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The task ID.
        #[serde(rename = "taskId")]
        task_id: String,
        /// The new task status.
        status: TaskStatus,
    },

    /// A subtask was created.
    SubtaskCreated {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The created subtask.
        subtask: SubTask,
    },

    /// A subtask was updated.
    SubtaskUpdated {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The updated subtask.
        subtask: SubTask,
    },

    /// A file was touched (read, written, or deleted) during execution.
    FileTouched {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The file path.
        #[serde(rename = "filePath")]
        file_path: String,
        /// The action performed.
        #[serde(rename = "action")]
        action: FileAction,
        /// Unified diff of changes (only present for write actions).
        #[serde(rename = "diff", skip_serializing_if = "Option::is_none")]
        diff: Option<String>,
    },

    /// Agent emitted a status phrase for UI display.
    StatusPhrase {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The status message to display.
        message: String,
        /// Timestamp when emitted.
        timestamp: DateTime<Utc>,
    },

    /// Agent narration for persistent display in the chat thread.
    /// Unlike StatusPhrase (transient spinner text), narrations survive execution cleanup.
    AgentNarration {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The conversation ID.
        #[serde(rename = "conversationId")]
        conversation_id: String,
        /// The thread ID.
        #[serde(rename = "threadId")]
        thread_id: String,
        /// Natural-language update (1-2 sentences).
        content: String,
        /// Timestamp when emitted.
        timestamp: DateTime<Utc>,
    },

    /// Content streaming chunk for assistant-visible text.
    /// This includes intermediate narration returned alongside tool calls
    /// as well as the final assistant response.
    StreamingContent {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The conversation ID.
        #[serde(rename = "conversationId")]
        conversation_id: String,
        /// The thread ID.
        #[serde(rename = "threadId")]
        thread_id: String,
        /// Content chunk (partial text).
        chunk: String,
        /// Whether this is the final chunk.
        #[serde(rename = "isFinal")]
        is_final: bool,
        /// Stable chat message id for this streamed content.
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        /// Whether the final streamed content should be persisted into chat history.
        #[serde(rename = "persistMessage", skip_serializing_if = "Option::is_none")]
        persist_message: Option<bool>,
    },

    /// Context was compacted or repacked for the next model step.
    ContextCompacted {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The conversation ID.
        #[serde(rename = "conversationId")]
        conversation_id: String,
        /// Why the runtime updated the working context.
        reason: ContextCompactionReason,
        /// Number of messages before compaction.
        #[serde(rename = "beforeCount")]
        before_count: usize,
        /// Number of messages after compaction.
        #[serde(rename = "afterCount")]
        after_count: usize,
        /// Estimated tokens before compaction.
        #[serde(rename = "beforeTokens")]
        before_tokens: usize,
        /// Estimated tokens after compaction.
        #[serde(rename = "afterTokens")]
        after_tokens: usize,
        /// Timestamp when compacted.
        timestamp: DateTime<Utc>,
    },

    /// A document artifact was created.
    ArtifactCreated {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The artifact ID.
        #[serde(rename = "artifactId")]
        artifact_id: String,
        /// The artifact type (e.g., "Docx", "Pptx", "Pdf").
        #[serde(rename = "artifactType")]
        artifact_type: String,
        /// The filename.
        filename: String,
        /// The file size in bytes.
        #[serde(rename = "sizeBytes")]
        size_bytes: u64,
        /// The file path.
        path: String,
    },
}

/// Development-only trace events emitted on a separate channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTraceEvent {
    /// Runtime trace event.
    RuntimeTrace {
        /// The runtime trace data.
        trace: RuntimeTrace,
    },
    /// Snapshot updated event.
    SnapshotUpdated {
        /// The execution snapshot.
        snapshot: ExecutionSnapshot,
    },
}

/// Action performed on a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    /// File was read.
    Read,
    /// File was written.
    Write,
    /// File was deleted.
    Delete,
}

/// Reason the runtime repacked or compacted the working context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextCompactionReason {
    /// Context was compacted because token usage reached the working limit.
    TokenLimit,
    /// Context was repacked because the active subtask changed.
    SubtaskTransition,
}

/// Tauri event name for agent events.
pub const AGENT_EVENT_NAME: &str = "agent:event";
/// Tauri event name for agent trace events.
pub const AGENT_TRACE_EVENT_NAME: &str = "agent:trace";
