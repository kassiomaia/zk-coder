//! Handles the termination of an agent's execution, including completion, cancellation, and failure.
//!
//! Session-level orchestration lives here. Two concerns that used to crowd this
//! module have been extracted:
//!
//! - [`streaming`] — chunking and emission of `StreamingContent` events.
//! - [`subtasks`] — subtask state transitions (auto-complete, advance pending,
//!   per-subtask result streaming).
//!
//! The helpers those submodules expose are re-exported below so callers in
//! sibling execution phases keep importing from `super::termination::…`.

use chrono::Utc;
use serde_json::json;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::harness_events::emit_harness_state_updated;
use super::state::ExecutionContext;
use crate::application::agent::harness::failed_startup_verification;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::{SessionId, SessionState};
use crate::core::agent::task::{AgentExecutionResult, TaskStatus, UsageStats};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::AgentRuntimeError;

mod streaming;
mod subtasks;

pub(crate) use streaming::emit_streaming_message;
pub(crate) use subtasks::{advance_next_pending_subtask, auto_complete_in_progress_subtasks};

use streaming::{emit_prebuilt_streaming_events, final_result_streaming_events};
use subtasks::{emit_subtask_result_with_trace, has_remaining_in_progress_subtasks};

/// Updates the state of an agent session and emits a `SessionStateChanged` event.
pub fn update_session_state(
    deps: &ExecutionDeps<'_>,
    session_id: &SessionId,
    state: SessionState,
) -> Result<(), AgentRuntimeError> {
    deps.session_repo
        .update_state(session_id, &state)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionStateChanged {
            session_id: session_id.0.clone(),
            state,
        },
    );

    let _ = emit_harness_state_updated(deps, session_id);

    Ok(())
}

/// Cancels the execution of an agent.
///
/// This function updates the execution snapshot, records a trace event,
/// updates the task status, and emits a `TaskUpdated` event.
pub fn cancel_execution(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    reason: &str,
) -> Result<(), AgentRuntimeError> {
    let tracer = deps.tracer();
    let mut snapshot = ctx.snapshot(
        ExecutionPhase::Cancelled,
        None,
        Some("Execution cancelled".to_string()),
    );
    snapshot.current_subtask_id = ctx.current_subtask_id.clone();
    let _ = tracer.save_snapshot(&snapshot);
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ExecutionCancelled,
        json!({ "reason": reason }),
    );
    let _ = deps.task_repo.update_task_status(
        &ctx.task_id,
        crate::core::agent::task::TaskStatus::Cancelled,
    );
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: ctx.session_id.clone(),
            task_id: ctx.task_id.clone(),
            status: TaskStatus::Cancelled,
        },
    );
    Ok(())
}

/// Fails the execution of an agent.
///
/// This function updates the session state to `Error`, updates the task status,
/// and records a snapshot and trace event for the failure.
pub fn fail_execution(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    error: &str,
    kind: &str,
    last_model: Option<String>,
) -> Result<(), AgentRuntimeError> {
    deps.harness_state_store.record_startup_verification(
        &ctx.session_id,
        failed_startup_verification("Session needs verification before resuming", error),
    );

    update_session_state(
        deps,
        &SessionId(ctx.session_id.clone()),
        SessionState::Error {
            message: error.to_string(),
            recoverable: true,
        },
    )?;

    let _ = deps
        .task_repo
        .update_task_status(&ctx.task_id, crate::core::agent::task::TaskStatus::Failed);
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: ctx.session_id.clone(),
            task_id: ctx.task_id.clone(),
            status: TaskStatus::Failed,
        },
    );
    let tracer = deps.tracer();
    let _ = tracer.save_snapshot(&ctx.snapshot(
        ExecutionPhase::Failed,
        last_model,
        Some(error.to_string()),
    ));
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ExecutionFailed,
        json!({ "error": error, "kind": kind }),
    );
    Ok(())
}

/// Terminates the current run because the task exhausted its planned step
/// budget (Workstream δ). Distinct from [`fail_execution`] on three axes:
///
/// 1. Session state message carries the budget-exhausted copy so the UI can
///    present "work preserved — review and resume" instead of the generic
///    runaway-loop wording.
/// 2. `needs_review` is flagged on the session; the Phase 5 review infra
///    picks this up so a human can triage.
/// 3. Subtask state is left untouched — completed work stays completed,
///    in-progress work stays in-progress. The root task is also left alone
///    so resume/replan flows can reuse it.
///
/// The runtime trace uses [`RuntimeTraceType::StepBudgetExhausted`] with a
/// `{ used, budget }` payload so telemetry can separate budget exhaustion
/// from genuine `LoopDetected` terminations.
///
/// # Errors
///
/// Propagates a repository error from `update_session_state` when the
/// session row cannot be updated. The `set_needs_review` call is
/// best-effort — a failure is logged and does not abort the path so the
/// session-state update still lands.
pub fn mark_step_budget_exhausted(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    used: u32,
    budget: u32,
) -> Result<(), AgentRuntimeError> {
    let message = format!(
        "Task reached its step budget ({used}/{budget}). Work preserved — review and resume."
    );

    if let Err(e) = deps
        .session_repo
        .set_needs_review(&SessionId(ctx.session_id.clone()), true)
    {
        log::warn!(
            "[AgentRuntime] Failed to flag session {} needs_review on budget exhaustion: {e}",
            ctx.session_id
        );
    }

    update_session_state(
        deps,
        &SessionId(ctx.session_id.clone()),
        SessionState::Error {
            message: message.clone(),
            recoverable: true,
        },
    )?;

    // Deliberately NOT calling `task_repo.update_task_status(Failed)` or
    // touching any subtask: δ preserves the live subtask graph so the
    // session can be resumed after human review. A `TaskUpdated` event is
    // likewise skipped — nothing on the task actually changed.

    let tracer = deps.tracer();
    let _ = tracer.save_snapshot(&ctx.snapshot(ExecutionPhase::Failed, None, Some(message)));
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::StepBudgetExhausted,
        json!({ "used": used, "budget": budget }),
    );
    Ok(())
}

/// Completes the execution of an agent.
///
/// This function finalizes the execution by:
/// 1.  Auto-completing any subtask that is still `InProgress`.
/// 2.  Checking whether more `InProgress` subtasks remain after auto-completion.
/// 3.  If no remaining work – updating the session state to `Finished`, recording
///     a final snapshot/trace, emitting events and streaming the final content.
/// 4.  If remaining work exists – returning `(result, true)` so the caller can
///     restart the execution loop on the next subtask.
///
/// Returns `((result, has_remaining_work))`.
pub async fn complete_execution(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    conversation_id: &str,
    thread_id: &str,
    content: String,
) -> Result<(AgentExecutionResult, bool), AgentRuntimeError> {
    let result = AgentExecutionResult {
        content,
        steps: ctx.steps.clone(),
        usage: Some(UsageStats {
            input_tokens: ctx.total_input_tokens,
            output_tokens: ctx.total_output_tokens,
            total_tokens: ctx.total_input_tokens + ctx.total_output_tokens,
        }),
    };

    let auto_completed = auto_complete_in_progress_subtasks(deps, &ctx.task_id, &ctx.session_id);

    if auto_completed > 0 {
        log::info!(
            "[AgentExecution] Auto-completed {} InProgress subtask(s) for task {}",
            auto_completed,
            ctx.task_id
        );
    }

    if has_remaining_in_progress_subtasks(deps, &ctx.task_id) {
        log::info!(
            "[AgentExecution] Remaining InProgress subtasks detected for task {} – \
             session will continue",
            ctx.task_id
        );
        emit_subtask_result_with_trace(
            deps,
            ctx,
            conversation_id,
            thread_id,
            &result,
            auto_completed,
            false,
        )
        .await;
        return Ok((result, true));
    }

    // If no InProgress subtask remained, check for Pending ones and advance the next one.
    if advance_next_pending_subtask(deps, &ctx.task_id, &ctx.session_id) {
        log::info!(
            "[AgentExecution] Pending subtasks detected for task {} – session will continue",
            ctx.task_id
        );
        emit_subtask_result_with_trace(
            deps,
            ctx,
            conversation_id,
            thread_id,
            &result,
            auto_completed,
            true,
        )
        .await;
        return Ok((result, true));
    }

    deps.harness_state_store
        .record_clean_exit(&ctx.session_id, true);

    update_session_state(
        deps,
        &SessionId(ctx.session_id.clone()),
        SessionState::Finished {
            completed_at: Utc::now(),
        },
    )?;
    let _ = deps.task_repo.update_task_status(
        &ctx.task_id,
        crate::core::agent::task::TaskStatus::Completed,
    );

    let tracer = deps.tracer();
    let _ = tracer.save_snapshot(&ctx.snapshot(ExecutionPhase::Completed, None, None));
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        None,
        ctx.step_number,
        RuntimeTraceType::ExecutionCompleted,
        json!({
            "responseLength": result.content.len(),
            "totalSteps": result.steps.len(),
            "totalInputTokens": ctx.total_input_tokens,
            "totalOutputTokens": ctx.total_output_tokens,
        }),
    );

    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: ctx.session_id.clone(),
            task_id: ctx.task_id.clone(),
            status: TaskStatus::Completed,
        },
    );
    let events = final_result_streaming_events(
        &ctx.session_id,
        &ctx.task_id,
        ctx.step_number,
        conversation_id,
        thread_id,
        &result.content,
    );
    emit_prebuilt_streaming_events(deps, events).await;
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ExecutionComplete {
            session_id: ctx.session_id.clone(),
            conversation_id: conversation_id.to_string(),
            thread_id: thread_id.to_string(),
            result: result.clone(),
        },
    );

    Ok((result, false))
}
