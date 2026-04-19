//! Implements the step gate, which controls the beginning of each execution step.
//!
//! This module is responsible for checking termination conditions, incrementing
//! the step counter, and ensuring that the execution does not enter a runaway loop.

use chrono::Utc;
use log::{info, warn};
use serde_json::json;

use super::deps::ExecutionDeps;
use super::state::ExecutionContext;
use super::termination::{cancel_execution, fail_execution, update_session_state};
use super::utils::max_execution_steps;
use crate::application::agent::clarification::{
    clear_pending_clarification, configured_timeout_minutes, pending_clarification,
};
use crate::application::agent::read_models::HarnessMode;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::{SessionId, SessionState};
use crate::infrastructure::agent::runtime::context::{estimate_context_tokens, CONTEXT_WINDOW};
use crate::infrastructure::agent::runtime::loop_detection::MAX_REPETITIONS;
use crate::infrastructure::agent::runtime::AgentRuntimeError;

/// The outcome of the `begin_step` function.
pub enum StepGateOutcome {
    /// The execution should proceed to the next step.
    Proceed,
    /// The execution has been cancelled.
    Cancelled,
    /// The harness is paused awaiting a user clarification reply.
    /// The coordinator returns control to the caller without calling
    /// the LLM; the next `execute` invocation picks up after the reply
    /// clears the pending state via
    /// `clarification::clear_pending_clarification`.
    AwaitingClarification,
}

/// Begins a new step in the execution, checking for termination conditions.
///
/// This function acts as a gate at the beginning of each execution step,
/// performing the following checks:
/// 1.  Checks if the session has been cancelled.
/// 2.  Increments the step number and updates the execution phase.
/// 3.  Checks if the maximum number of execution steps has been reached to prevent loops.
/// 4.  Updates the session state and records a snapshot of the execution context.
///
/// # Arguments
///
/// * `deps` - The execution dependencies.
/// * `ctx` - The mutable execution context.
///
/// # Returns
///
/// A `Result` containing a `StepGateOutcome` on success, or an `AgentRuntimeError` on failure.
pub fn begin_step(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
) -> Result<StepGateOutcome, AgentRuntimeError> {
    let session = deps
        .session_repo
        .get_session(&crate::core::agent::session::SessionId(
            ctx.session_id.clone(),
        ))
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    if matches!(
        session.state,
        crate::core::agent::session::SessionState::Cancelled { .. }
    ) {
        cancel_execution(deps, ctx, "session_cancelled")?;
        return Ok(StepGateOutcome::Cancelled);
    }

    // Workstream E.2 / E.3 — check for a pending clarification before
    // every LLM call. Recording the pending state happens in
    // `permissions::handle_narrate` when `intent = "question"`; we
    // consult the registry here to decide whether to pause or time out.
    if let Some(pending) = pending_clarification(&ctx.session_id) {
        let timeout_minutes = configured_timeout_minutes();
        if pending.has_timed_out(Utc::now(), timeout_minutes) {
            handle_clarification_timeout(deps, ctx, &pending, timeout_minutes)?;
            // The subtask is marked Failed; yield back so the outer
            // driver re-enters `execute` with the updated task state.
            return Ok(StepGateOutcome::AwaitingClarification);
        }
        // Still pending — record the harness mode and pause. The
        // awaiting details travel on a sibling override so the public
        // `mode` enum stays a flat string on the wire for legacy
        // frontend comparisons.
        deps.harness_state_store
            .record_mode(&ctx.session_id, HarnessMode::AwaitingClarification);
        deps.harness_state_store.record_awaiting_clarification(
            &ctx.session_id,
            crate::application::agent::read_models::AwaitingClarificationDetails {
                prompt: pending.prompt.clone(),
                asked_at: pending.asked_at,
            },
        );
        let tracer = deps.tracer();
        let _ = tracer.append_trace(
            &ctx.session_id,
            &ctx.task_id,
            ctx.current_subtask_id.as_deref(),
            ctx.step_number,
            RuntimeTraceType::AwaitingClarification,
            json!({
                "prompt": pending.prompt,
                "askedAt": pending.asked_at.to_rfc3339(),
            }),
        );
        return Ok(StepGateOutcome::AwaitingClarification);
    }

    ctx.step_number += 1;
    ctx.current_phase = ExecutionPhase::PreparingStep;
    let tracer = deps.tracer();
    let _ = tracer.emit_transition(
        &ctx.session_id,
        &ctx.task_id,
        ctx.step_number,
        None,
        ExecutionPhase::PreparingStep,
        "step_incremented",
    );

    // Workstream β — monotonic growth of the dynamic step budget. If the
    // α merge path (`permissions::merge_model_plan_into_existing`)
    // appended new subtasks since bootstrap, the live subtask count
    // here will exceed the count captured in `bootstrap.rs`. Recomputing
    // the cap and taking the max preserves the "never shrink" invariant
    // (the formula is monotonic in `num_subtasks`, but we still clamp
    // defensively in case a future code path deletes subtasks — the cap
    // must not retroactively invalidate work the agent already did).
    if let Ok(live_subtasks) = deps.task_repo.get_subtasks(&ctx.task_id) {
        let recomputed = max_execution_steps(live_subtasks.len() as u32);
        if recomputed > ctx.max_execution_steps {
            ctx.max_execution_steps = recomputed;
        }
    }

    if ctx.step_number > ctx.max_execution_steps {
        // Phase-β wording: keep the "runaway loop" framing and the
        // `LoopDetected` terminal variant intact — Workstream δ rewrites
        // both (new `StepBudgetExhausted` variant, copy rewrite,
        // preserved subtask states). Touching either here would
        // duplicate work and conflict with δ's scope.
        let msg = format!(
            "Execution step limit reached ({}). Aborting to prevent runaway loop.",
            ctx.max_execution_steps
        );
        warn!("[AgentRuntime] {msg}");
        fail_execution(deps, ctx, &msg, "step_limit", None)?;
        return Err(AgentRuntimeError::LoopDetected);
    }

    info!(
        "[AgentRuntime] Step {} for session {}",
        ctx.step_number, ctx.session_id
    );
    let estimated_tokens = estimate_context_tokens(&ctx.llm_context);
    ctx.context_usage_percent =
        (estimated_tokens as f64 / CONTEXT_WINDOW as f64 * 100.0).min(100.0);

    update_session_state(
        deps,
        &SessionId(ctx.session_id.clone()),
        SessionState::Running {
            current_step: ctx.step_number,
            context_usage: ctx.context_usage_percent,
        },
    )?;
    let _ = tracer.save_snapshot(&ctx.snapshot(
        ExecutionPhase::PreparingStep,
        Some(ctx.model_name.clone()),
        None,
    ));
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        None,
        ctx.step_number,
        RuntimeTraceType::StepStarted,
        json!({
            "contextUsage": ctx.context_usage_percent,
            "estimatedTokens": estimated_tokens,
            "maxRepetitions": MAX_REPETITIONS
        }),
    );

    Ok(StepGateOutcome::Proceed)
}

/// Handles the Workstream E.3 timeout fallback.
///
/// Marks the current subtask `Failed` with reason
/// `unanswered_clarification`, clears the pending-clarification
/// registry so the next session is not blocked, and emits a
/// `SubtaskFailedByRuntime` trace. The whole-session state is not
/// mutated — other subtasks can still proceed once the task is
/// re-entered — so this path explicitly does *not* call
/// `fail_execution`.
fn handle_clarification_timeout(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    pending: &crate::application::agent::clarification::PendingClarification,
    timeout_minutes: i64,
) -> Result<(), AgentRuntimeError> {
    let subtask_id = ctx.current_subtask_id.clone();
    clear_pending_clarification(&ctx.session_id);

    let tracer = deps.tracer();
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::SubtaskFailedByRuntime,
        json!({
            "reason": "unanswered_clarification",
            "prompt": pending.prompt,
            "askedAt": pending.asked_at.to_rfc3339(),
            "timeoutMinutes": timeout_minutes,
        }),
    );
    if let Some(subtask_id) = subtask_id {
        deps.task_repo
            .update_subtask_status(
                &subtask_id,
                crate::core::agent::task::TaskStatus::Failed,
            )
            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod clarification_gate_tests {
    //! Workstream E.2 / E.3 — step-gate pause and timeout behavior.
    //!
    //! These tests exercise the pending-clarification registry and the
    //! `PendingClarification::has_timed_out` policy the gate relies on.
    //! The full gate pass-through through `begin_step` requires the
    //! heavyweight `ExecutionDeps` fixture, which is covered by the
    //! existing integration tests; the unit tests here lock in the
    //! deterministic logic the gate branches on.
    use crate::application::agent::clarification::{
        clear_pending_clarification, configured_timeout_minutes, pending_clarification,
        record_pending_clarification, record_pending_clarification_at, reset_for_tests,
        test_serializer, DEFAULT_CLARIFICATION_TIMEOUT_MINUTES,
    };
    use chrono::{Duration, Utc};

    #[test]
    fn synthetic_trace_of_narration_question_sets_pending_state() {
        // Simulates the `agent_narrate` intent=question code path by
        // recording directly; the step gate's `pending_clarification`
        // lookup must return the prompt so it can pause.
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        record_pending_clarification("session-pause", "What is the product name?");
        let pending = pending_clarification("session-pause")
            .expect("step gate must observe the pending clarification");
        assert_eq!(pending.prompt, "What is the product name?");
    }

    #[test]
    fn user_reply_path_clears_pending_state() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        record_pending_clarification("session-reply", "Do you want the A or B variant?");
        assert!(pending_clarification("session-reply").is_some());

        // The chat message-input pipeline calls `clear_pending_clarification`
        // when a user reply arrives; simulate that:
        clear_pending_clarification("session-reply");
        assert!(pending_clarification("session-reply").is_none());
    }

    #[test]
    fn timeout_regression_subtask_fails_cleanly_on_unanswered_clarification() {
        // Workstream E.3 regression reproducer. Empty workspace +
        // narration question + simulated timeout → the gate's timeout
        // branch decides to fail the subtask.
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        let asked_at =
            Utc::now() - Duration::minutes(DEFAULT_CLARIFICATION_TIMEOUT_MINUTES + 1);
        record_pending_clarification_at("session-timeout", "needs product name", asked_at);
        let pending = pending_clarification("session-timeout").expect("pending recorded");
        assert!(
            pending.has_timed_out(Utc::now(), configured_timeout_minutes()),
            "timeout branch must fire so the gate marks the subtask Failed",
        );
    }
}

