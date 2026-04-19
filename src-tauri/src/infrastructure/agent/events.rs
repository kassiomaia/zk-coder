//! Agent event types for frontend communication.
//!
//! Events are emitted via Tauri's event system so the frontend
//! can reactively update the agent panel UI.

use serde::{Deserialize, Serialize};

use crate::application::agent::read_models::AgentSessionHarnessState;
use crate::application::agent::service::planner::SubtaskPlan as SubTaskPlan;
use crate::core::agent::session::{PendingAction, SessionState};
use crate::core::agent::task::{
    AgentExecutionResult, AgentTask, ExecutionStep, SubTask, TaskStatus,
};
use crate::core::agent::{ExecutionSnapshot, RuntimeTrace};
use chrono::{DateTime, Utc};

/// Narration intent (Workstream E.1).
///
/// A `Narration` is informational and does not change the harness's execution
/// state. A `Question` signals that the agent needs information it does not
/// have; the runtime transitions the harness into
/// `HarnessMode::AwaitingClarification` (Workstream E.2) and pauses the main
/// loop until the user replies or the clarification timeout (E.3) elapses.
///
/// Defaults to `Narration` so pre-E tool calls (and on-disk event rows written
/// before the field existed) still deserialize without loss.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrationIntent {
    /// Informational update. Execution continues without pausing.
    #[default]
    Narration,
    /// Blocking question. Runtime pauses until the user replies.
    Question,
}

impl NarrationIntent {
    /// Parse the on-the-wire `intent` argument emitted by `agent_narrate`.
    ///
    /// Unknown or missing values fall back to the default `Narration` —
    /// pre-E tool calls that emit only `{message}` still parse cleanly and
    /// keep the old continue-after-narrate behavior.
    #[must_use]
    pub fn from_arg(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("question") => Self::Question,
            _ => Self::Narration,
        }
    }
}

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

    /// The user's reply to a pending clarification was delivered and
    /// appended to the LLM context. The coordinator is about to resume
    /// execution on the same task and subtask.
    ClarificationReplied {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The reply content as it was appended to the transcript and
        /// the LLM context.
        content: String,
        /// The subtask that was in progress when the question was asked.
        #[serde(rename = "subtaskId", skip_serializing_if = "Option::is_none")]
        subtask_id: Option<String>,
    },

    /// A pending clarification went unanswered past the configured
    /// timeout. The runtime fails the active subtask with
    /// `unanswered_clarification`; see `RuntimeTraceType::SubtaskFailedByRuntime`.
    ClarificationTimedOut {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The subtask that was failed on behalf of the agent.
        #[serde(rename = "subtaskId", skip_serializing_if = "Option::is_none")]
        subtask_id: Option<String>,
        /// The question that was never answered.
        prompt: String,
    },

    /// Emitted at startup when a paused clarification is rehydrated
    /// from persistent storage. The UI renders the existing prompt
    /// immediately so the user never loses the in-flight question to
    /// an app close.
    ClarificationRestored {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The question the agent had asked before the restart.
        prompt: String,
        /// UTC timestamp the question was originally asked at.
        #[serde(rename = "askedAt")]
        asked_at: DateTime<Utc>,
        /// The subtask the question was asked against.
        #[serde(rename = "subtaskId", skip_serializing_if = "Option::is_none")]
        subtask_id: Option<String>,
    },

    /// Session-facing long-running harness state changed.
    HarnessStateUpdated {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// Current harness state snapshot for the session.
        #[serde(rename = "harnessState")]
        harness_state: AgentSessionHarnessState,
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

    /// The agent resumed an existing plan instead of generating a new
    /// one. Emitted from the bootstrap phase when the resume planner
    /// returns `PlanAction::Continue`, so the frontend knows not to
    /// expect a `TaskPlanGenerated` / `TaskCreated` payload and can
    /// simply refresh the existing task's subtask statuses.
    ExecutionResumed {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The existing task ID being resumed.
        #[serde(rename = "taskId")]
        task_id: String,
        /// The id of the subtask that is now `InProgress` — the cursor
        /// the runtime will continue from.
        #[serde(rename = "nextSubtaskId", skip_serializing_if = "Option::is_none")]
        next_subtask_id: Option<String>,
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

    /// LLM-generated task plan with title, description, and subtasks.
    /// Emitted after initial planning so frontend can display the correct title immediately.
    TaskPlanGenerated {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// The task ID.
        #[serde(rename = "taskId")]
        task_id: String,
        /// LLM-generated title.
        title: String,
        /// LLM-generated description.
        description: String,
        /// Whether this task has subtasks.
        #[serde(rename = "isNonTrivial")]
        is_non_trivial: bool,
        /// Planned subtasks.
        subtasks: Vec<SubTaskPlan>,
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
    /// Unlike `StatusPhrase` (transient spinner text), narrations survive execution cleanup.
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
        /// Narration intent (Workstream E.1). `Question` signals that the
        /// harness has paused awaiting a user reply; the UI surfaces the
        /// message with a distinct "reply" affordance. Defaults to
        /// `Narration` on deserialization so pre-E event logs still parse.
        #[serde(default, rename = "intent")]
        intent: NarrationIntent,
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

    /// PDF render entered a new phase.
    PdfPhaseChanged {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// Phase label.
        phase: String,
    },

    /// PDF render incremental progress update.
    PdfProgress {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// Percentage complete (0–100).
        percent: u8,
    },

    /// PDF render emitted a non-fatal warning.
    PdfWarning {
        /// The session ID.
        #[serde(rename = "sessionId")]
        session_id: String,
        /// Human-readable warning text.
        message: String,
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
