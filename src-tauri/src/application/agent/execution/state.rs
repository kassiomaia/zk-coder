//! Defines the state management for an agent's execution.
//!
//! This module contains the `ExecutionContext` struct, which holds all the
//! runtime state for an agent's execution, as well as other related data
//! structures for managing the execution flow.

use chrono::Utc;

use crate::core::agent::execution::{
    ExecutionCheckpoint, ExecutionPhase, ExecutionSnapshot, LlmTracePayload,
};
use crate::core::agent::task::ExecutionStep;
use crate::core::llm::entities::{ChatMessage as Message, ChatResponse, ToolChoiceMode, ToolDefinition};
use crate::infrastructure::agent::runtime::context::MemorizedFailure;
use crate::infrastructure::agent::runtime::session_scope::RuntimeSessionScope;
use crate::infrastructure::agent::runtime::SubtaskSummary;

use super::replace_drift::ReplaceFailure;

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
    /// Rolling count of retrievals that have fired within the current
    /// subtask since the last memory refresh (M3.3). Reset to 0 on every
    /// subtask change; incremented by `context_phase` whenever a
    /// retrieval pass runs. When it crosses
    /// `memory_refresh::RETRIEVAL_BURST_THRESHOLD` the runtime fires a
    /// `RetrievalBurst` trigger and resets the counter.
    pub retrieval_count_in_current_subtask: u32,
    /// Snapshot of the last subtask id that `context_phase` observed as
    /// `Completed` and successfully fed into a `SubtaskCompleted` memory
    /// refresh. Guards against re-firing the trigger on every subsequent
    /// step that happens to have the same completed subtask still
    /// visible via `ctx.current_subtask_id`.
    pub last_completed_subtask_for_memory: Option<String>,
    /// Subtask-scoped failure memory overlay (Workstream C.1/C.2).
    /// Appended by `tool_phase::run_tools` after each failing tool
    /// result, capturing the `current_subtask_id` at record time.
    /// Consumed by `context_phase::resolve_context`, which filters to
    /// the active subtask (or unscoped) before handing the list to the
    /// packer via `ContextPolicyInput.memorized_failures`. Bounded to
    /// the most recent entries so unbounded session length cannot
    /// balloon this list.
    pub memorized_failures: Vec<MemorizedFailure>,
    /// Dynamic step budget for this task (Workstream β — fidelity
    /// follow-up). Computed once at bootstrap from the initial subtask
    /// count via
    /// [`super::utils::max_execution_steps`], then grown monotonically
    /// by `step_gate` whenever the α merge path appends new subtasks at
    /// runtime. Consulted before every LLM round-trip to decide whether
    /// to abort with a step-budget-exhausted error.
    ///
    /// Monotonic: this field never decreases after bootstrap; shrinking
    /// mid-task would retroactively invalidate work already performed.
    pub max_execution_steps: u32,
    /// Recent `fs_replace_in_file` misses used for the Workstream γ.2
    /// drift-aware nudge. Appended by `tool_phase` on every miss and
    /// pruned to the active sliding window by
    /// [`super::replace_drift::evaluate_replace_drift`]. When two
    /// misses on the same path land within the window, a soft
    /// correction system message is injected into the next step's
    /// context and the entries for that path are cleared so the same
    /// nudge does not re-fire.
    pub recent_replace_failures: Vec<ReplaceFailure>,
    /// Number of *consecutive* steps in which every tool call the LLM
    /// emitted carried the malformed-arguments sentinel — i.e. the
    /// provider returned unparseable JSON and `tool_phase` surfaced it as
    /// a recoverable tool-level error. Reset to 0 on any step where at
    /// least one tool call parsed cleanly. Incremented by `tool_phase`
    /// and consulted via [`super::utils::MALFORMED_ARGS_ABORT_THRESHOLD`]
    /// so the runtime bails instead of looping forever if the model
    /// cannot self-correct. See
    /// `docs/issues/agent-plan-subtasks-malformed-json/issue.md`.
    pub consecutive_malformed_tool_args: u32,
    /// Non-persisted runtime accumulator — per-session trackers
    /// previously held in process-global statics inside `permissions.rs`
    /// (`PLAN_RETRY_TRACKER`, `SUBTASK_RATIONALES`,
    /// `SUBTASK_FAILURE_COUNTS`, `DECOMPOSED_SUBTASKS`). Lifted into the
    /// context so state is tied to the owning session and cannot leak
    /// across sessions. Reset to [`RuntimeSessionScope::default`] on
    /// checkpoint resume (same treatment as `memorized_failures`).
    pub(crate) runtime_session_scope: RuntimeSessionScope,
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

    /// Captures the persistable state of this context into an
    /// [`ExecutionCheckpoint`] (agent-clarification-resume plan).
    ///
    /// Paired with [`Self::from_checkpoint`]. Persisted fields are the
    /// minimum a fresh coordinator needs to resume execution; fields
    /// deliberately omitted are rebuildable at resume time (see
    /// `from_checkpoint`).
    #[must_use]
    pub fn to_checkpoint(&self) -> ExecutionCheckpoint {
        ExecutionCheckpoint {
            schema_version: ExecutionCheckpoint::CURRENT_SCHEMA_VERSION,
            session_id: self.session_id.clone(),
            task_id: self.task_id.clone(),
            model_name: self.model_name.clone(),
            step_number: self.step_number,
            current_phase: self.current_phase,
            current_subtask_id: self.current_subtask_id.clone(),
            last_injected_subtask_order: self.last_injected_subtask_order,
            llm_context: self.llm_context.clone(),
            recent_tool_calls: self.recent_tool_calls.clone(),
            consecutive_empty_tool_iterations: self.consecutive_empty_tool_iterations,
            current_allowed_tool_names: self.current_allowed_tool_names.clone(),
            total_input_tokens: self.total_input_tokens,
            total_output_tokens: self.total_output_tokens,
            context_usage_percent: self.context_usage_percent,
            steps: self.steps.clone(),
            max_execution_steps: self.max_execution_steps,
        }
    }

    /// Rebuilds an [`ExecutionContext`] from a persisted
    /// [`ExecutionCheckpoint`]. The non-persisted runtime accumulators
    /// (tool-choice override, retrieval counter, memorized failures,
    /// replace-drift window) are reset to their bootstrap defaults;
    /// the coordinator rebuilds them as it proceeds.
    ///
    /// The LLM client, permission resolver, and `yolo_mode_provider`
    /// live outside [`ExecutionContext`] today, so this conversion is
    /// synchronous. Phase 6 of the agent-clarification-resume plan
    /// wires the surrounding `create_llm_client` call at the resume
    /// call-site rather than inside this helper.
    #[must_use]
    pub fn from_checkpoint(checkpoint: ExecutionCheckpoint) -> Self {
        Self {
            session_id: checkpoint.session_id,
            task_id: checkpoint.task_id,
            model_name: checkpoint.model_name,
            llm_context: checkpoint.llm_context,
            steps: checkpoint.steps,
            total_input_tokens: checkpoint.total_input_tokens,
            total_output_tokens: checkpoint.total_output_tokens,
            step_number: checkpoint.step_number,
            recent_tool_calls: checkpoint.recent_tool_calls,
            current_subtask_id: checkpoint.current_subtask_id,
            last_injected_subtask_order: checkpoint.last_injected_subtask_order,
            current_phase: checkpoint.current_phase,
            context_usage_percent: checkpoint.context_usage_percent,
            consecutive_empty_tool_iterations: checkpoint.consecutive_empty_tool_iterations,
            current_allowed_tool_names: checkpoint.current_allowed_tool_names,
            max_execution_steps: checkpoint.max_execution_steps,
            // Non-persisted runtime accumulators — reset to bootstrap
            // defaults. The coordinator rebuilds them as it proceeds.
            pending_tool_choice_override: None,
            retrieval_count_in_current_subtask: 0,
            last_completed_subtask_for_memory: None,
            memorized_failures: Vec::new(),
            recent_replace_failures: Vec::new(),
            consecutive_malformed_tool_args: 0,
            // Fresh per-session scope — matches the treatment of
            // `memorized_failures` (rebuilt by the coordinator).
            runtime_session_scope: RuntimeSessionScope::default(),
        }
    }
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    use crate::core::llm::entities::{ChatMessage, MessageContent, MessageRole, ToolCall};

    fn sample_context() -> ExecutionContext {
        ExecutionContext {
            session_id: "session-1".into(),
            task_id: "task-1".into(),
            model_name: "claude-opus-4-7".into(),
            llm_context: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("hello".into()),
                    tool_calls: vec![],
                    tool_result: None,
                },
                ChatMessage {
                    role: MessageRole::Assistant,
                    content: MessageContent::Text("calling tool".into()),
                    tool_calls: vec![ToolCall {
                        id: "call-1".into(),
                        name: "fs_read_file".into(),
                        arguments: serde_json::json!({"path": "/a"}),
                    }],
                    tool_result: None,
                },
            ],
            steps: vec![],
            total_input_tokens: 50,
            total_output_tokens: 17,
            step_number: 3,
            recent_tool_calls: vec![("fs_read_file".into(), 1)],
            current_subtask_id: Some("sub-1".into()),
            last_injected_subtask_order: Some(0),
            current_phase: ExecutionPhase::CallingLlm,
            context_usage_percent: 12.5,
            consecutive_empty_tool_iterations: 0,
            current_allowed_tool_names: Some(vec!["fs_read_file".into()]),
            pending_tool_choice_override: None,
            retrieval_count_in_current_subtask: 0,
            last_completed_subtask_for_memory: None,
            memorized_failures: Vec::new(),
            max_execution_steps: 12,
            recent_replace_failures: Vec::new(),
            consecutive_malformed_tool_args: 0,
            runtime_session_scope: RuntimeSessionScope::new(
                crate::core::agent::session::SessionId("session-1".into()),
            ),
        }
    }

    #[test]
    fn to_checkpoint_then_from_checkpoint_is_structurally_equivalent() {
        let ctx = sample_context();
        let checkpoint = ctx.to_checkpoint();
        let reconstructed = ExecutionContext::from_checkpoint(checkpoint);

        assert_eq!(reconstructed.session_id, ctx.session_id);
        assert_eq!(reconstructed.task_id, ctx.task_id);
        assert_eq!(reconstructed.model_name, ctx.model_name);
        assert_eq!(reconstructed.step_number, ctx.step_number);
        assert_eq!(reconstructed.current_phase, ctx.current_phase);
        assert_eq!(reconstructed.current_subtask_id, ctx.current_subtask_id);
        assert_eq!(
            reconstructed.last_injected_subtask_order,
            ctx.last_injected_subtask_order
        );
        assert_eq!(reconstructed.total_input_tokens, ctx.total_input_tokens);
        assert_eq!(reconstructed.total_output_tokens, ctx.total_output_tokens);
        assert_eq!(reconstructed.context_usage_percent, ctx.context_usage_percent);
        assert_eq!(
            reconstructed.consecutive_empty_tool_iterations,
            ctx.consecutive_empty_tool_iterations
        );
        assert_eq!(
            reconstructed.current_allowed_tool_names,
            ctx.current_allowed_tool_names
        );
        assert_eq!(reconstructed.max_execution_steps, ctx.max_execution_steps);
        assert_eq!(reconstructed.recent_tool_calls, ctx.recent_tool_calls);

        // llm_context serde round-trip preserves role/content/tool calls.
        let before = serde_json::to_value(&ctx.llm_context).unwrap();
        let after = serde_json::to_value(&reconstructed.llm_context).unwrap();
        assert_eq!(before, after);

        // Non-persisted runtime accumulators reset cleanly.
        assert!(reconstructed.memorized_failures.is_empty());
        assert!(reconstructed.recent_replace_failures.is_empty());
        assert_eq!(reconstructed.retrieval_count_in_current_subtask, 0);
        assert!(reconstructed.last_completed_subtask_for_memory.is_none());
        assert!(reconstructed.pending_tool_choice_override.is_none());
        // RuntimeSessionScope is also reset on resume — it holds
        // per-session trackers (plan retries, rationales, failure
        // counts, decomposed subtasks) that a fresh coordinator
        // rebuilds as it proceeds.
        assert!(reconstructed.runtime_session_scope.session_id().is_none());
    }

    #[test]
    fn checkpoint_schema_version_is_current() {
        let ctx = sample_context();
        let checkpoint = ctx.to_checkpoint();
        assert_eq!(
            checkpoint.schema_version,
            ExecutionCheckpoint::CURRENT_SCHEMA_VERSION
        );
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
    /// The rendered *framing section* used by `system_prompt`
    /// (Workstream H.2.c). Carries either the declared `TaskFraming`
    /// rendering or the genre-default code framing. `llm_phase` hashes
    /// this string to emit a framing-specific trace attribution.
    pub active_framing_section: String,
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
