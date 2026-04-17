//! Defines the state management for an agent's execution.
//!
//! This module contains the `ExecutionContext` struct, which holds all the
//! runtime state for an agent's execution, as well as other related data
//! structures for managing the execution flow.

use chrono::Utc;

use crate::core::agent::execution::{ExecutionPhase, ExecutionSnapshot, LlmTracePayload};
use crate::core::agent::task::ExecutionStep;
use crate::core::llm::entities::{ChatMessage as Message, ChatResponse, ToolChoiceMode, ToolDefinition};
use crate::infrastructure::agent::runtime::SubtaskSummary;

/// Holds all the runtime state for an agent's execution.
pub struct ExecutionContext {
    /// The unique identifier for the session.
    pub session_id: String,
    /// The unique identifier for the task being executed.
    pub task_id: String,
    /// The name of the model being used for the execution.
    pub model_name: String,
    /// The context of the LLM, including all messages.
    pub llm_context: Vec<Message>,
    /// A list of all execution steps that have been performed.
    pub steps: Vec<ExecutionStep>,
    /// The total number of input tokens used so far.
    pub total_input_tokens: u32,
    /// The total number of output tokens generated so far.
    pub total_output_tokens: u32,
    /// The current step number in the execution process.
    pub step_number: u32,
    /// A list of recent tool calls for detecting loops.
    pub recent_tool_calls: Vec<(String, u64)>,
    /// The ID of the current subtask, if any.
    pub current_subtask_id: Option<String>,
    /// The order of the last subtask that was injected into the context.
    pub last_injected_subtask_order: Option<i32>,
    /// The current phase of the execution.
    pub current_phase: ExecutionPhase,
    /// The percentage of the context window that is currently being used.
    pub context_usage_percent: f64,
    /// Number of consecutive iterations where the LLM produced no tool calls
    /// while `InProgress` subtasks still remained. Used by the narration-loop
    /// guard in `response_phase` to escalate when the agent stops making
    /// progress. Reset to `0` on any iteration that finishes execution or
    /// runs tools.
    pub consecutive_empty_tool_iterations: u32,
    /// Set by `resolve_context` each step to the list of tools that the
    /// current subtask mode permits. `None` when no active subtask or when the
    /// runtime has no mode-based restriction (legacy behavior where all tools
    /// are allowed). `Some(vec![])` forces a no-tool state (final-summary
    /// path). Runtime enforcement in `tool_phase` rejects calls whose name is
    /// not in this list without invoking the tool executor.
    pub current_allowed_tool_names: Option<Vec<String>>,
    /// One-shot override for the next LLM request's tool-choice mode. Consumed
    /// (via `.take()`) inside `llm_phase::call_llm`. Producers: the
    /// final-summary path in `coordinator` (forces `None`) and the narration-
    /// loop `EmitFillerAndNudge` stage in `response_phase` (forces `Required`).
    pub pending_tool_choice_override: Option<ToolChoiceMode>,
}

impl ExecutionContext {
    /// Creates a snapshot of the current execution state.
    ///
    /// # Arguments
    ///
    /// * `phase` - The current phase of the execution.
    /// * `last_model` - The last model that was used, if any.
    /// * `last_error` - The last error message, if any.
    ///
    /// # Returns
    ///
    /// An `ExecutionSnapshot` representing the current state.
    #[must_use]
    pub fn snapshot(
        &self,
        phase: ExecutionPhase,
        last_model: Option<String>,
        last_error: Option<String>,
    ) -> ExecutionSnapshot {
        ExecutionSnapshot {
            session_id: self.session_id.clone(),
            task_id: self.task_id.clone(),
            step_number: self.step_number,
            phase,
            current_subtask_id: self.current_subtask_id.clone(),
            context_usage_percent: self.context_usage_percent,
            total_input_tokens: self.total_input_tokens,
            total_output_tokens: self.total_output_tokens,
            last_model,
            last_error,
            updated_at: Utc::now(),
        }
    }
}

/// Represents the resolved context after the context resolution phase.
#[derive(Clone)]
pub struct ResolvedContext {
    /// A summary of the current subtask, if any, along with the total number of subtasks.
    pub current_subtask_summary: Option<(SubtaskSummary, usize)>,
    /// The ID of the current subtask, if any.
    pub current_subtask_id: Option<String>,
    /// The list of tool definitions that are available for the current step.
    pub tool_definitions: Vec<ToolDefinition>,
    /// The system prompt to be used for the LLM call.
    pub system_prompt: String,
    /// `true` when subtasks exist but all are in a terminal state (Completed/Failed/Cancelled).
    /// The coordinator uses this to finish execution instead of calling the LLM again.
    pub all_subtasks_completed: bool,
}

/// The result of a call to the Large Language Model.
pub struct LlmCallResult {
    /// The response from the LLM.
    pub response: ChatResponse,
    /// The duration of the LLM call in milliseconds.
    pub duration_ms: u64,
    /// The payload of the request, for tracing purposes.
    pub request_payload: LlmTracePayload,
    /// The payload of the response, for tracing purposes.
    pub response_payload: LlmTracePayload,
}
