//! The coordinator's park stage for user clarification replies
//! (agent-clarification-resume plan).
//!
//! When the agent emitted `agent_narrate { intent: "question" }`,
//! `permissions::handle_narrate` wrote a row into
//! `agent_pending_clarifications` and registered a `oneshot::Sender`
//! for the reply. This module is invoked by the coordinator after the
//! tool-result writeback: it detects the pending row, persists the
//! pause (session state + `ExecutionCheckpoint`), waits on the live
//! reply channel (with a timeout and cancellation arm), and cleans up
//! on resume.
//!
//! Control flow returned to the coordinator:
//!
//! - [`ParkOutcome::Continue`] — no pending clarification; proceed.
//! - [`ParkOutcome::Replied`] — the reply was delivered and appended
//!   to `ctx.llm_context`; the loop continues to the next LLM call.
//! - [`ParkOutcome::TimedOut`] — the timeout elapsed first; the
//!   current subtask was failed and the coordinator must terminate.
//! - [`ParkOutcome::Cancelled`] — the session was cancelled mid-park.
//!
//! Three writes happen on pause entry, in this order:
//!
//! 1. `SessionState` → `AwaitingClarification { prompt, asked_at, subtask_id }`.
//! 2. The `agent_pending_clarifications` row already exists (written
//!    by `handle_narrate`).
//! 3. `agent_execution_checkpoints` row inserted.
//!
//! And two clear-writes on resume / timeout, paired with the state
//! transition back to `Running` (resume) or `Paused` (timeout).

use std::time::Duration;

use chrono::Utc;
use log::warn;
use serde_json::json;

use super::deps::ExecutionDeps;
use super::state::ExecutionContext;
use super::termination::update_session_state;
use crate::application::agent::clarification::{
    clear_pending_clarification_persistent, configured_timeout_minutes,
    pending_clarification_record, take_reply_receiver,
};
use crate::core::agent::execution::RuntimeTraceType;
use crate::core::agent::session::{SessionId, SessionState};
use crate::core::llm::entities::ChatMessage as Message;
use crate::infrastructure::agent::runtime::AgentRuntimeError;

/// Outcome of the park stage.
#[derive(Debug)]
pub enum ParkOutcome {
    /// No clarification is pending; proceed to the next LLM call.
    Continue,
    /// The user replied; the reply has been appended to
    /// `ctx.llm_context` and session state has transitioned back to
    /// `Running`. The coordinator should continue the loop.
    Replied,
    /// The clarification timeout elapsed before a reply arrived. The
    /// active subtask was failed with reason
    /// `unanswered_clarification`; the coordinator must terminate.
    TimedOut,
    /// The session was cancelled while the park was active. The
    /// coordinator must terminate.
    Cancelled,
}

/// Detects and handles a pending clarification.
///
/// Called by the coordinator after the tool-result writeback
/// completes, before the next LLM call.
pub async fn maybe_park_for_clarification(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
) -> Result<ParkOutcome, AgentRuntimeError> {
    let pending = pending_clarification_record(&ctx.session_id)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    let Some(record) = pending else {
        return Ok(ParkOutcome::Continue);
    };

    let timeout_minutes = configured_timeout_minutes();
    let timeout_duration = timeout_for_remaining(&record.asked_at, timeout_minutes);
    if timeout_duration.is_none() && timeout_minutes > 0 {
        // Already past the deadline on entry.
        handle_clarification_timeout(deps, ctx, &record, timeout_minutes)?;
        return Ok(ParkOutcome::TimedOut);
    }

    // Pause-entry writes. The clarification row already exists (it was
    // inserted by `permissions::handle_narrate`). Here we transition
    // the session state and persist the `ExecutionCheckpoint`.
    let session_id_str = ctx.session_id.clone();
    let session_id = SessionId(session_id_str.clone());
    let checkpoint = ctx.to_checkpoint();
    deps.execution_checkpoint_repo
        .save_checkpoint(&checkpoint)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    update_session_state(
        deps,
        &session_id,
        SessionState::AwaitingClarification {
            prompt: record.prompt.clone(),
            asked_at: record.asked_at,
            subtask_id: record.subtask_id.clone(),
        },
    )?;

    let _ = deps.tracer().append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::AwaitingClarification,
        json!({
            "prompt": record.prompt,
            "askedAt": record.asked_at.to_rfc3339(),
        }),
    );

    let receiver = take_reply_receiver(&ctx.session_id);
    let Some(reply_rx) = receiver else {
        // No live receiver — the handle_narrate path must have raced
        // with a session teardown. Treat as a timeout-equivalent so
        // the coordinator terminates cleanly rather than hanging.
        warn!(
            "[AgentRuntime] park_for_clarification: no reply receiver for session {} — treating as timeout",
            ctx.session_id
        );
        handle_clarification_timeout(deps, ctx, &record, timeout_minutes)?;
        return Ok(ParkOutcome::TimedOut);
    };

    // Park. The cancellation arm polls session state at the same
    // cadence the tool-phase cancellation-check uses elsewhere in the
    // runtime, so an operator cancelling via the frontend IPC is
    // observed within the tick interval.
    let cancellation_poll = Duration::from_millis(500);
    let timeout_budget = timeout_duration;

    let wait_result = match timeout_budget {
        Some(duration) => park_with_timeout(reply_rx, &session_id_str, deps, duration, cancellation_poll).await,
        None => park_without_timeout(reply_rx, &session_id_str, deps, cancellation_poll).await,
    };

    match wait_result {
        ParkSignal::Reply(content) => {
            // Transactional resume: clear clarification rows + checkpoint,
            // transition session state back to Running, append the reply
            // to the LLM context so the next LLM call sees it.
            clear_pending_clarification_persistent(&ctx.session_id)
                .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
            deps.execution_checkpoint_repo
                .clear_checkpoint(&ctx.session_id)
                .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
            update_session_state(
                deps,
                &session_id,
                SessionState::Running {
                    current_step: ctx.step_number,
                    context_usage: ctx.context_usage_percent,
                },
            )?;
            ctx.llm_context.push(Message::user(&content));
            let _ = deps.tracer().append_trace(
                &ctx.session_id,
                &ctx.task_id,
                ctx.current_subtask_id.as_deref(),
                ctx.step_number,
                RuntimeTraceType::ClarificationResumed,
                json!({
                    "stepNumber": ctx.step_number,
                    "subtaskId": ctx.current_subtask_id,
                    "restored": false,
                }),
            );
            Ok(ParkOutcome::Replied)
        }
        ParkSignal::TimedOut => {
            handle_clarification_timeout(deps, ctx, &record, timeout_minutes)?;
            Ok(ParkOutcome::TimedOut)
        }
        ParkSignal::Cancelled => {
            clear_pending_clarification_persistent(&ctx.session_id)
                .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
            deps.execution_checkpoint_repo
                .clear_checkpoint(&ctx.session_id)
                .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
            Ok(ParkOutcome::Cancelled)
        }
    }
}

#[derive(Debug)]
enum ParkSignal {
    Reply(String),
    TimedOut,
    Cancelled,
}

fn timeout_for_remaining(
    asked_at: &chrono::DateTime<chrono::Utc>,
    timeout_minutes: i64,
) -> Option<Duration> {
    if timeout_minutes <= 0 {
        // A disabled timeout keeps the park alive indefinitely. We
        // return `Some(a very large Duration)` by way of `None` to
        // signal "no deadline"; the caller branches on it.
        return Some(Duration::from_secs(365 * 24 * 60 * 60));
    }
    let deadline = *asked_at + chrono::Duration::minutes(timeout_minutes);
    let remaining = deadline - Utc::now();
    if remaining <= chrono::Duration::zero() {
        None
    } else {
        Some(Duration::from_secs(remaining.num_seconds().max(1) as u64))
    }
}

async fn park_with_timeout(
    reply_rx: tokio::sync::oneshot::Receiver<String>,
    session_id: &str,
    deps: &ExecutionDeps<'_>,
    timeout: Duration,
    cancellation_poll: Duration,
) -> ParkSignal {
    let mut reply_rx = reply_rx;
    let sleep = tokio::time::sleep(timeout);
    tokio::pin!(sleep);
    let mut poll = tokio::time::interval(cancellation_poll);
    poll.tick().await; // skip the immediate first tick
    loop {
        tokio::select! {
            result = &mut reply_rx => {
                return match result {
                    Ok(content) => ParkSignal::Reply(content),
                    Err(_) => {
                        // Sender dropped; treat as cancellation rather than
                        // hanging forever or silently timing out.
                        ParkSignal::Cancelled
                    }
                };
            }
            _ = &mut sleep => {
                return ParkSignal::TimedOut;
            }
            _ = poll.tick() => {
                if session_is_cancelled(deps, session_id) {
                    return ParkSignal::Cancelled;
                }
            }
        }
    }
}

async fn park_without_timeout(
    reply_rx: tokio::sync::oneshot::Receiver<String>,
    session_id: &str,
    deps: &ExecutionDeps<'_>,
    cancellation_poll: Duration,
) -> ParkSignal {
    let mut reply_rx = reply_rx;
    let mut poll = tokio::time::interval(cancellation_poll);
    poll.tick().await;
    loop {
        tokio::select! {
            result = &mut reply_rx => {
                return match result {
                    Ok(content) => ParkSignal::Reply(content),
                    Err(_) => ParkSignal::Cancelled,
                };
            }
            _ = poll.tick() => {
                if session_is_cancelled(deps, session_id) {
                    return ParkSignal::Cancelled;
                }
            }
        }
    }
}

fn session_is_cancelled(deps: &ExecutionDeps<'_>, session_id: &str) -> bool {
    match deps
        .session_repo
        .get_session(&SessionId(session_id.to_string()))
    {
        Ok(session) => matches!(
            session.state,
            SessionState::Cancelled { .. }
        ),
        Err(_) => false,
    }
}

fn handle_clarification_timeout(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    record: &crate::core::agent::clarification::PendingClarificationRecord,
    timeout_minutes: i64,
) -> Result<(), AgentRuntimeError> {
    let subtask_id = ctx.current_subtask_id.clone();

    // Clear both layers of the clarification store and the checkpoint
    // before mutating other state so a concurrent reply attempt sees
    // `NotPending` and fails cleanly.
    clear_pending_clarification_persistent(&ctx.session_id)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    deps.execution_checkpoint_repo
        .clear_checkpoint(&ctx.session_id)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    // Transition session state → `Paused` so the frontend sees a
    // terminal-ish pause; it can be resumed via the recovery flow if
    // the user decides to try again.
    update_session_state(
        deps,
        &SessionId(ctx.session_id.clone()),
        SessionState::Paused {
            paused_at: Utc::now(),
            message: "Clarification timed out".to_string(),
            current_subtask_id: subtask_id.clone(),
        },
    )?;

    let tracer = deps.tracer();
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::SubtaskFailedByRuntime,
        json!({
            "reason": "unanswered_clarification",
            "prompt": record.prompt,
            "askedAt": record.asked_at.to_rfc3339(),
            "timeoutMinutes": timeout_minutes,
        }),
    );
    // Emit the frontend-facing timeout event so the UI can surface the
    // retry affordance.
    let _ = deps.app_handle.emit(
        crate::infrastructure::agent::events::AGENT_EVENT_NAME,
        crate::infrastructure::agent::events::AgentEvent::ClarificationTimedOut {
            session_id: ctx.session_id.clone(),
            subtask_id: subtask_id.clone(),
            prompt: record.prompt.clone(),
        },
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

use tauri::Emitter;
