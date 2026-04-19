//! Handles the response phase of the agent's execution.
//!
//! This module is responsible for processing the response from the LLM,
//! determining the next course of action, and handling the completion or
//! continuation of the execution.

use chrono::Utc;
use serde_json::json;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::message_ids::{streaming_message_id, StreamingMessageIdInput, StreamingPhase};
use super::state::{ExecutionContext, LlmCallResult};
use super::termination::{
    advance_next_pending_subtask, auto_complete_in_progress_subtasks, complete_execution,
    emit_streaming_message, fail_execution,
};
use crate::core::agent::execution::RuntimeTraceType;
use crate::core::agent::task::{AgentExecutionResult, ExecutionStep, StepAction, StepResult};
use crate::core::llm::entities::{ChatMessage as Message, ToolChoiceMode};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::{
    detect_repetitive_loop_in_state, AgentRuntimeError, MAX_REPETITIONS,
};

/// Hard ceiling for the narration-loop guard.
///
/// When the LLM produces zero tool calls while `InProgress` subtasks remain, the
/// execution loop cannot make progress on its own. The guard escalates through
/// [`NarrationLoopStage`] and fails the execution once the counter reaches
/// this value. Tunable in one place so manual repro sessions can raise it.
const NARRATION_LOOP_LIMIT: u32 = 4;

/// One escalation stage of the narration-loop guard.
///
/// Selected by [`classify_narration_loop_stage`] from the current value of
/// [`ExecutionContext::consecutive_empty_tool_iterations`]. Kept as a pure enum
/// so the classification is unit-testable without touching `ExecutionDeps`.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum NarrationLoopStage {
    /// First empty-tool-call iteration — emit the "Continuing..." filler and
    /// allow the loop to restart. Current baseline behavior.
    EmitFiller,
    /// Second iteration — still emit the filler, but also inject a strong user
    /// nudge into `llm_context` telling the LLM it MUST call a tool.
    EmitFillerAndNudge,
    /// Third iteration — the LLM ignored the nudge. Force-advance the current
    /// subtask via the termination helpers and let the loop pick up the next.
    ForceAdvance,
    /// Fourth iteration or above — fail the execution cleanly with
    /// [`AgentRuntimeError::NarrationLoop`].
    Fail,
}

/// Maps a running counter of consecutive empty-tool-call iterations to an
/// escalation stage. Pure function, no side effects — tested in isolation.
pub(super) const fn classify_narration_loop_stage(counter: u32) -> NarrationLoopStage {
    match counter {
        0 | 1 => NarrationLoopStage::EmitFiller,
        2 => NarrationLoopStage::EmitFillerAndNudge,
        3 => NarrationLoopStage::ForceAdvance,
        _ => NarrationLoopStage::Fail,
    }
}

/// Text injected into `llm_context` at stage [`NarrationLoopStage::EmitFillerAndNudge`].
///
/// Kept as a module constant so the test harness can assert that exactly this
/// string is pushed, and so the wording can be tuned in one place.
const NARRATION_LOOP_NUDGE: &str = "You produced a response with no tool calls while a subtask is still InProgress. You MUST make progress by either (a) calling a tool to do the work, or (b) calling `agent_mark_subtask_status` to mark the current subtask as Completed, or (c) calling `agent_advance_to_next_subtask` to move on. Narrating intent without calling a tool is not progress and will cause the execution to fail.";

/// Represents the decision made after handling the LLM's response.
pub enum ResponseDecision {
    /// The execution has finished, and this is the final result.
    Finished(AgentExecutionResult),
    /// The execution should continue by executing the specified tools.
    ExecuteTools(crate::core::llm::entities::ChatResponse),
    /// The LLM finished its current work but there are remaining subtasks.
    /// The execution loop should restart so the next `InProgress` subtask is picked up.
    ContinueWithNextSubtask,
}

/// Handles the response from the LLM and determines the next step.
///
/// This function processes the `LlmCallResult` by:
/// 1.  Recording the LLM execution step and emitting an event.
/// 2.  Checking if the response contains tool calls.
/// 3.  If there are no tool calls, it completes the execution and returns the final result.
/// 4.  If there are tool calls, it checks for repetitive loops to prevent infinite execution.
/// 5.  If any intermediate content was generated, it streams it to the frontend.
/// 6.  Finally, it returns a decision to either finish the execution or proceed with tool execution.
///
/// # Arguments
///
/// * `deps` - The execution dependencies.
/// * `ctx` - The mutable execution context.
/// * `session` - The current agent session.
/// * `llm_result` - The result of the LLM call.
///
/// # Returns
///
/// A `Result` containing a `ResponseDecision` on success, or an `AgentRuntimeError` on failure.
pub async fn handle_response(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
    session: &crate::core::agent::session::AgentSession,
    llm_result: LlmCallResult,
) -> Result<ResponseDecision, AgentRuntimeError> {
    let tracer = deps.tracer();
    let response = llm_result.response;

    let llm_step = ExecutionStep {
        step_number: ctx.step_number,
        action: StepAction::LlmRequest {
            model: response.model.clone(),
        },
        result: Some(StepResult {
            success: true,
            output: json!({
                "content_length": response.content.len(),
                "tool_calls": response.tool_calls.len(),
            }),
            tokens_used: response.usage.as_ref().map(|usage| usage.total_tokens),
            input_tokens: response.usage.as_ref().map(|usage| usage.input_tokens),
            output_tokens: response.usage.as_ref().map(|usage| usage.output_tokens),
            duration_ms: llm_result.duration_ms,
        }),
        timestamp: Utc::now(),
    };

    if let Err(error) = deps.telemetry_repo.create_execution_step(
        &llm_step,
        &ctx.session_id,
        Some(&ctx.task_id),
        ctx.current_subtask_id.as_deref(),
    ) {
        log::warn!("[AgentRuntime] Failed to persist execution step: {error}");
    }
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ExecutionStep {
            session_id: ctx.session_id.clone(),
            step: llm_step.clone(),
        },
    );
    ctx.steps.push(llm_step);

    if response.tool_calls.is_empty() {
        let _ = tracer.append_trace(
            &ctx.session_id,
            &ctx.task_id,
            ctx.current_subtask_id.as_deref(),
            ctx.step_number,
            RuntimeTraceType::LlmResponseClassified,
            json!({"hasToolCalls": false, "willFinalize": true}),
        );
        let (result, has_remaining) = complete_execution(
            deps,
            ctx,
            &session.conversation_id,
            &session.thread_id,
            response.content,
        )
        .await?;

        if has_remaining {
            return handle_narration_loop_escalation(deps, ctx, session, &response.model).await;
        }

        ctx.consecutive_empty_tool_iterations = 0;
        return Ok(ResponseDecision::Finished(result));
    }

    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::LlmResponseClassified,
        json!({
            "hasToolCalls": true,
            "toolCallCount": response.tool_calls.len(),
            "toolNames": response.tool_calls.iter().map(|call| call.name.clone()).collect::<Vec<_>>(),
        }),
    );

    for tool_call in &response.tool_calls {
        if let Some(repeat_count) = detect_repetitive_loop_in_state(
            tool_call,
            &ctx.recent_tool_calls,
            MAX_REPETITIONS,
            ctx.current_allowed_tool_names.as_deref(),
        ) {
            let msg = format!(
                "Detected repetitive loop: tool '{}' called with same arguments {} times",
                tool_call.name,
                repeat_count + 1
            );
            fail_execution(
                deps,
                ctx,
                &msg,
                "loop_detected",
                Some(response.model.clone()),
            )?;
            return Err(AgentRuntimeError::LoopDetected);
        }
    }

    if !response.content.trim().is_empty() {
        let preamble_id = streaming_message_id(&StreamingMessageIdInput {
            phase: StreamingPhase::ToolPreamble,
            session_id: &ctx.session_id,
            task_id: &ctx.task_id,
            subtask_id: ctx.current_subtask_id.as_deref(),
            step_number: ctx.step_number,
        });
        emit_streaming_message(
            deps,
            &ctx.session_id,
            &session.conversation_id,
            &session.thread_id,
            &response.content,
            true,
            preamble_id,
            true,
        )
        .await;
    }

    ctx.consecutive_empty_tool_iterations = 0;
    Ok(ResponseDecision::ExecuteTools(response))
}

/// Drives the narration-loop escalation state machine when the LLM finishes an
/// iteration with zero tool calls while `InProgress` subtasks still remain.
///
/// This function is the sole caller of [`classify_narration_loop_stage`] in
/// production and owns all side effects for the four escalation stages
/// (filler, nudge, forced advance, fail). Extracted from `handle_response` so
/// the main function keeps its top-level control flow readable and stays under
/// clippy's `too_many_lines` budget.
async fn handle_narration_loop_escalation(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
    session: &crate::core::agent::session::AgentSession,
    model: &str,
) -> Result<ResponseDecision, AgentRuntimeError> {
    ctx.consecutive_empty_tool_iterations = ctx
        .consecutive_empty_tool_iterations
        .saturating_add(1);
    let stage = classify_narration_loop_stage(ctx.consecutive_empty_tool_iterations);

    let _ = deps.tracer().append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::LlmResponseClassified,
        json!({
            "narrationLoopStage": format!("{stage:?}"),
            "consecutiveEmptyToolIterations": ctx.consecutive_empty_tool_iterations,
            "narrationLoopLimit": NARRATION_LOOP_LIMIT,
        }),
    );

    match stage {
        NarrationLoopStage::EmitFiller => {
            emit_continuation_filler(deps, ctx, session).await;
            Ok(ResponseDecision::ContinueWithNextSubtask)
        }
        NarrationLoopStage::EmitFillerAndNudge => {
            emit_continuation_filler(deps, ctx, session).await;
            ctx.llm_context
                .push(Message::user(NARRATION_LOOP_NUDGE.to_string()));
            // Force the next LLM call to require a tool (M2.3). Provider
            // adapters map this to `tool_choice: required` (OpenAI) or
            // `{"type":"any"}` (Anthropic). The nudge text alone has proven
            // insufficient in practice; forcing tool use raises the recovery
            // probability while still allowing the model to pick which tool.
            ctx.pending_tool_choice_override = Some(ToolChoiceMode::Required);
            let _ = deps.tracer().append_trace(
                &ctx.session_id,
                &ctx.task_id,
                ctx.current_subtask_id.as_deref(),
                ctx.step_number,
                RuntimeTraceType::LlmResponseClassified,
                json!({
                    "forcedToolChoiceMode": "required",
                    "reason": "narration_loop_nudge",
                }),
            );
            Ok(ResponseDecision::ContinueWithNextSubtask)
        }
        NarrationLoopStage::ForceAdvance => {
            let auto_completed =
                auto_complete_in_progress_subtasks(deps, &ctx.task_id, &ctx.session_id);
            let advanced = advance_next_pending_subtask(deps, &ctx.task_id, &ctx.session_id);
            log::warn!(
                "[AgentRuntime] Narration loop force-advance fired for task {} \
                 (auto_completed={auto_completed}, advanced_pending={advanced})",
                ctx.task_id
            );
            ctx.consecutive_empty_tool_iterations = 0;
            Ok(ResponseDecision::ContinueWithNextSubtask)
        }
        NarrationLoopStage::Fail => {
            let msg = format!(
                "Detected narration loop: LLM produced no tool calls across {} \
                 consecutive iterations while subtasks remained InProgress",
                ctx.consecutive_empty_tool_iterations
            );
            fail_execution(deps, ctx, &msg, "narration_loop", Some(model.to_string()))?;
            Err(AgentRuntimeError::NarrationLoop)
        }
    }
}

/// Emits the "Continuing with next subtask..." streaming filler used by the
/// narration-loop guard at stages [`NarrationLoopStage::EmitFiller`] and
/// [`NarrationLoopStage::EmitFillerAndNudge`]. Extracted so the two stages share
/// one emission path and the hardcoded copy lives in a single place.
async fn emit_continuation_filler(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    session: &crate::core::agent::session::AgentSession,
) {
    let continuation_id = streaming_message_id(&StreamingMessageIdInput {
        phase: StreamingPhase::SubtaskContinuation,
        session_id: &ctx.session_id,
        task_id: &ctx.task_id,
        subtask_id: ctx.current_subtask_id.as_deref(),
        step_number: ctx.step_number,
    });
    emit_streaming_message(
        deps,
        &ctx.session_id,
        &session.conversation_id,
        &session.thread_id,
        "\n\nContinuing with next subtask...\n",
        false,
        continuation_id,
        false,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::{classify_narration_loop_stage, NarrationLoopStage, NARRATION_LOOP_LIMIT};

    #[test]
    fn zero_counter_emits_filler() {
        // Before the first increment the classifier must not escalate.
        assert_eq!(
            classify_narration_loop_stage(0),
            NarrationLoopStage::EmitFiller
        );
    }

    #[test]
    fn first_offense_emits_filler() {
        assert_eq!(
            classify_narration_loop_stage(1),
            NarrationLoopStage::EmitFiller
        );
    }

    #[test]
    fn second_offense_injects_nudge() {
        assert_eq!(
            classify_narration_loop_stage(2),
            NarrationLoopStage::EmitFillerAndNudge
        );
    }

    #[test]
    fn third_offense_forces_advance() {
        assert_eq!(
            classify_narration_loop_stage(3),
            NarrationLoopStage::ForceAdvance
        );
    }

    #[test]
    fn fourth_offense_fails() {
        assert_eq!(
            classify_narration_loop_stage(NARRATION_LOOP_LIMIT),
            NarrationLoopStage::Fail
        );
    }

    #[test]
    fn above_limit_still_fails() {
        assert_eq!(
            classify_narration_loop_stage(NARRATION_LOOP_LIMIT + 1),
            NarrationLoopStage::Fail
        );
        assert_eq!(
            classify_narration_loop_stage(u32::MAX),
            NarrationLoopStage::Fail
        );
    }

    #[test]
    fn narration_loop_limit_is_tuned_conservatively() {
        // Guard against accidental relaxation: 4 means stage 1+2+3 run before
        // the hard failure, giving the LLM three chances to recover. Bumping
        // this value should be a deliberate decision with a test update.
        assert_eq!(NARRATION_LOOP_LIMIT, 4);
    }
}
