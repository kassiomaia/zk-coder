//! Contains models for observing and debugging the agent's execution process.
//!
//! These data structures are used for tracing, logging, and persisting the state
//! of an agent's execution, providing visibility into its internal operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents the current phase of an agent's execution run.
///
/// This enum is used to track the progress of an agent as it moves through
/// different stages of processing a task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPhase {
    /// The initial phase where the execution environment is being set up.
    Bootstrapping,
    /// The phase for preparing the context and parameters for the next step.
    PreparingStep,
    /// The phase where context from various sources is being resolved.
    ResolvingContext,
    /// The phase when the agent is making a call to the Large Language Model.
    CallingLlm,
    /// The phase for handling and interpreting the response from the LLM.
    HandlingLlmResponse,
    /// The phase when the agent is executing a batch of tools.
    ExecutingTools,
    /// The final phase indicating that the execution has completed successfully.
    Completed,
    /// A terminal phase indicating that the execution has failed.
    Failed,
    /// A terminal phase indicating that the execution was cancelled by the user.
    Cancelled,
}

/// Represents a transition between two execution phases.
///
/// This struct is used to log and trace the agent's movement from one phase
/// to another, providing a reason for the transition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionTransition {
    /// The phase from which the execution is transitioning. Can be `None` for the initial phase.
    pub from: Option<ExecutionPhase>,
    /// The phase to which the execution is transitioning.
    pub to: ExecutionPhase,
    /// A descriptive reason for the transition.
    pub reason: String,
}

/// A snapshot of the agent's execution state at a specific point in time.
///
/// This struct is used to persist the state of an execution, allowing it to be
/// resumed later or analyzed for debugging purposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSnapshot {
    /// The ID of the session this snapshot belongs to.
    pub session_id: String,
    /// The ID of the task being executed.
    pub task_id: String,
    /// The current step number in the execution process.
    pub step_number: u32,
    /// The current phase of the execution.
    pub phase: ExecutionPhase,
    /// The ID of the current subtask, if any.
    pub current_subtask_id: Option<String>,
    /// The percentage of the context window that is currently being used.
    pub context_usage_percent: f64,
    /// The total number of input tokens used so far.
    pub total_input_tokens: u32,
    /// The total number of output tokens generated so far.
    pub total_output_tokens: u32,
    /// The ID of the last model used for an LLM call.
    pub last_model: Option<String>,
    /// The last error message, if any.
    pub last_error: Option<String>,
    /// The timestamp of when this snapshot was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Defines the types of runtime trace events that can be recorded.
///
/// This enum provides a structured way to classify different types of events
/// that occur during an agent's execution, which is useful for debugging and analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTraceType {
    /// Indicates the start of an entire execution.
    ExecutionStarted,
    /// Indicates the start of a single step within the execution.
    StepStarted,
    /// Indicates the completion of a single step.
    StepFinished,
    /// Records a transition from one execution phase to another.
    PhaseTransition,
    /// Records the resolution of context for a subtask.
    SubtaskContextResolved,
    /// Indicates the start of a data retrieval process.
    RetrievalStarted,
    /// Indicates the completion of a data retrieval process.
    RetrievalCompleted,
    /// Records the evaluation of a context policy.
    ContextPolicyEvaluated,
    /// Records the filtering of the available toolset.
    ToolsetFiltered,
    /// Records a memory-refresh decision taken by the runtime (M3.3).
    ///
    /// The payload carries the trigger kind, whether a write actually
    /// occurred, how many findings/facts were added, whether the active
    /// plan changed, and any skipped-reason when the trigger fired but no
    /// mutation happened (e.g. the memory row does not yet exist for this
    /// session, or all proposed additions were duplicates). Non-trigger
    /// steps do not emit this trace.
    MemoryRefreshed,
    /// Records the preparation of a request to the LLM.
    LlmRequestPrepared,
    /// Indicates the start of a request to the LLM.
    LlmRequestStarted,
    /// Records that a retry for an LLM request has been scheduled.
    LlmRetryScheduled,
    /// Indicates that a response from the LLM has been received.
    LlmResponseReceived,
    /// Records the classification of the LLM's response.
    LlmResponseClassified,
    /// Indicates the start of the execution of a batch of tools.
    ToolBatchStarted,
    /// Indicates the completion of the execution of a batch of tools.
    ToolBatchCompleted,
    /// Indicates a spawn request was accepted for execution.
    SpawnStarted,
    /// Indicates a spawn request completed successfully or with a non-zero exit code.
    SpawnFinished,
    /// Indicates a spawn request was denied by policy or capability checks.
    SpawnDenied,
    /// Indicates a spawn request failed before or during execution.
    SpawnFailed,
    /// Indicates a spawn request timed out.
    SpawnTimedOut,
    /// Indicates a spawn request was cancelled by the session.
    SpawnCancelled,
    /// Indicates a spawn request was forcefully terminated.
    SpawnKilled,
    /// Indicates a spawn request was blocked because capacity was exhausted.
    SpawnBusy,
    /// Records that the execution was cancelled.
    ExecutionCancelled,
    /// Records that the execution has failed.
    ExecutionFailed,
    /// Records that the execution has completed successfully.
    ExecutionCompleted,
    /// A write-class tool call produced an output whose path did not
    /// match any `path_pattern` in the active subtask's contract
    /// (Workstream B.3). Payload fields:
    ///
    /// - `toolName` — the write-class tool that fired
    /// - `writtenPath` — the path the tool targeted (empty when the
    ///   tool does not declare a path in its arguments)
    /// - `expectedPatterns` — the contract patterns that did not match
    /// - `enforcement` — `"soft"` when the runtime only injected a
    ///   correction message, `"hard"` when the call was rejected
    ///   (B.4, gated on the hard-enforcement flag)
    SubtaskScopeDrift,
    /// The harness paused at the step gate because `agent_narrate` was
    /// called with `intent = "question"` (Workstream E.2). Payload:
    ///
    /// - `prompt` — the exact question the agent emitted
    /// - `askedAt` — RFC3339 timestamp
    AwaitingClarification,
    /// The runtime failed the current subtask on behalf of the agent
    /// because a clarification question went unanswered past the
    /// configured timeout (Workstream E.3). Payload:
    ///
    /// - `reason` — always `"unanswered_clarification"` for E.3; the
    ///   event is reserved for future runtime-driven subtask failure
    ///   reasons
    /// - `prompt` — the exact question that was never answered
    /// - `askedAt` — RFC3339 timestamp the question was asked at
    /// - `timeoutMinutes` — the configured timeout in effect
    SubtaskFailedByRuntime,
    /// Two `fs_replace_in_file` calls targeting the same path missed
    /// within the five-step sliding window (Workstream γ.2). The
    /// runtime injected a soft-nudge correction system message; no
    /// tool was re-invoked and no subtask state was changed. Payload:
    ///
    /// - `path` — the file path whose pair of misses fired the nudge
    ReplaceDriftDetected,
}

/// Represents a single, append-only entry in the runtime trace.
///
/// Each `RuntimeTrace` captures a specific event that occurred during execution,
/// providing detailed information for debugging and observability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTrace {
    /// A unique identifier for the trace entry.
    pub id: String,
    /// The ID of the session this trace belongs to.
    pub session_id: String,
    /// The ID of the task this trace is associated with.
    pub task_id: String,
    /// The ID of the subtask, if applicable.
    pub subtask_id: Option<String>,
    /// The step number at which this trace was recorded.
    pub step_number: u32,
    /// The type of the trace event.
    pub trace_type: RuntimeTraceType,
    /// The payload of the trace, containing detailed information about the event.
    pub payload_json: serde_json::Value,
    /// The timestamp of when the trace was created.
    pub created_at: DateTime<Utc>,
}

/// A detailed payload for tracing LLM requests and responses.
///
/// This struct provides a comprehensive set of information for diagnosing
/// issues related to LLM interactions, including prompts, responses, and performance metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LlmTracePayload {
    /// The model that was used for the request.
    pub model: String,
    /// The attempt number for this request (e.g., for retries).
    pub attempt: usize,
    /// The temperature setting used for the request.
    pub temperature: f32,
    /// The maximum number of tokens allowed for the response.
    pub max_tokens: u32,
    /// The number of messages in the context sent to the model.
    pub context_message_count: usize,
    /// The estimated number of tokens in the context.
    pub estimated_context_tokens: usize,
    /// The currently active subtask, if any.
    pub active_subtask: Option<String>,
    /// The list of tools that were allowed for this request.
    pub allowed_tools: Vec<String>,
    /// The number of tools in the catalog for this request (duplicates
    /// `allowed_tools.len()` but is the canonical field consumers should read
    /// when looking at request-shape stability).
    pub tool_count: usize,
    /// Hex-encoded SHA-256 of the system prompt. Stable inputs produce a
    /// stable hash; any drift in this value means the system prompt changed
    /// between adjacent steps, which invalidates the prompt-prefix cache.
    pub system_prompt_hash: String,
    /// Hex-encoded SHA-256 of the harness-contract section (Workstream H.5).
    ///
    /// Emitted alongside [`system_prompt_hash`] so the trace consumer can
    /// attribute prompt-prefix drift to the exact section that moved.
    /// Invariant across tasks, subtasks, and mode transitions.
    pub harness_section_hash: String,
    /// Hex-encoded SHA-256 of the genre-neutral quality-baseline section
    /// (Workstream H.5). Invariant across every task type.
    pub quality_baseline_hash: String,
    /// Hex-encoded SHA-256 of the framing section (Workstream H.5). In
    /// Phase 1 this is the default code framing and is therefore stable
    /// across subtask transitions. When Workstream H.2 lands per-task
    /// `TaskFraming`, this hash will move at task boundaries.
    pub framing_hash: String,
    /// Hex-encoded SHA-256 of the sorted tool catalog. A change in this hash
    /// between adjacent steps means the visible tool catalog was mutated.
    pub tool_catalog_hash: String,
    /// Runtime tool-choice intent for this request
    /// (`auto` / `required` / `none`). When the runtime forces a non-auto
    /// mode (for example on the final-summary path or narration-loop
    /// recovery), this field records that decision so a trace consumer can
    /// separate "model chose not to call a tool" from "runtime disallowed
    /// tool calls".
    pub tool_choice_mode: String,
    /// Hex-encoded SHA-256 of the message prefix (all messages except the
    /// final one). Stable prefix hash across adjacent steps means the
    /// provider-side prompt cache can stay warm; drift means an earlier
    /// message was mutated.
    pub message_prefix_hash: String,
    /// The system prompt that was used.
    pub system_prompt: String,
    /// The messages that were part of the request.
    pub request_messages: Vec<serde_json::Value>,
    /// The definitions of the tools provided to the model.
    pub tool_definitions: Vec<serde_json::Value>,
    /// The content of the response from the model.
    pub response_content: Option<String>,
    /// The tool calls that were part of the response.
    pub response_tool_calls: Vec<serde_json::Value>,
    /// The number of input tokens used.
    pub input_tokens: Option<u32>,
    /// The number of output tokens generated.
    pub output_tokens: Option<u32>,
    /// The total number of tokens used.
    pub total_tokens: Option<u32>,
    /// The duration of the request in milliseconds.
    pub duration_ms: Option<u64>,
    /// A flag indicating if the response was truncated.
    pub was_truncated: bool,
}
