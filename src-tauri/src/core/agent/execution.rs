//! Execution observability models for agent runtime.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current phase of an execution run.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPhase {
    Bootstrapping,
    PreparingStep,
    ResolvingContext,
    CallingLlm,
    HandlingLlmResponse,
    ExecutingTools,
    Completed,
    Failed,
    Cancelled,
}

/// Transition between two phases or named execution states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionTransition {
    pub from: Option<ExecutionPhase>,
    pub to: ExecutionPhase,
    pub reason: String,
}

/// Persisted checkpoint of the current execution state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSnapshot {
    pub session_id: String,
    pub task_id: String,
    pub step_number: u32,
    pub phase: ExecutionPhase,
    pub current_subtask_id: Option<String>,
    pub context_usage_percent: f64,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub last_model: Option<String>,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Structured runtime trace type for deep execution debugging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTraceType {
    ExecutionStarted,
    StepStarted,
    StepFinished,
    PhaseTransition,
    SubtaskContextResolved,
    RetrievalStarted,
    RetrievalCompleted,
    ContextPolicyEvaluated,
    ToolsetFiltered,
    LlmRequestPrepared,
    LlmRequestStarted,
    LlmRetryScheduled,
    LlmResponseReceived,
    LlmResponseClassified,
    ToolBatchStarted,
    ToolBatchCompleted,
    ExecutionCancelled,
    ExecutionFailed,
    ExecutionCompleted,
}

/// Append-only trace entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTrace {
    pub id: String,
    pub session_id: String,
    pub task_id: String,
    pub subtask_id: Option<String>,
    pub step_number: u32,
    pub trace_type: RuntimeTraceType,
    pub payload_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Full LLM trace payload for development diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LlmTracePayload {
    pub model: String,
    pub attempt: usize,
    pub temperature: f32,
    pub max_tokens: u32,
    pub context_message_count: usize,
    pub estimated_context_tokens: usize,
    pub active_subtask: Option<String>,
    pub allowed_tools: Vec<String>,
    pub system_prompt: String,
    pub request_messages: Vec<serde_json::Value>,
    pub tool_definitions: Vec<serde_json::Value>,
    pub response_content: Option<String>,
    pub response_tool_calls: Vec<serde_json::Value>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub duration_ms: Option<u64>,
    pub was_truncated: bool,
}
