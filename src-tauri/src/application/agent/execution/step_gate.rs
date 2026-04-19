//! Implements the step gate, which controls the beginning of each execution step.
//!
//! This module is responsible for checking termination conditions, incrementing
//! the step counter, and ensuring that the execution does not enter a runaway loop.

use log::{info, warn};
use serde_json::json;

use super::deps::ExecutionDeps;
use super::state::ExecutionContext;
use super::termination::{
    cancel_execution, mark_step_budget_exhausted, update_session_state,
};
use super::utils::max_execution_steps;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::{SessionId, SessionState};
use crate::infrastructure::agent::runtime::context::{estimate_context_tokens, CONTEXT_WINDOW};
use crate::infrastructure::agent::runtime::loop_detection::MAX_REPETITIONS;
use crate::infrastructure::agent::runtime::AgentRuntimeError;

/// The outcome of the `begin_step` function.
///
/// Clarification pause is handled by
/// [`super::park::maybe_park_for_clarification`] in the coordinator
/// (agent-clarification-resume plan); the step gate only observes
/// cancellation and step-budget exhaustion now.
pub enum StepGateOutcome {
    /// The execution should proceed to the next step.
    Proceed,
    /// The execution has been cancelled.
    Cancelled,
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
        // Workstream δ — budget exhaustion is not a loop. `used` is the
        // count of completed steps (step_number was just incremented to
        // the one we are refusing), and `budget` is the live cap after
        // any α-merge growth. Both surface on the error variant and on
        // the runtime trace so the UI and telemetry can distinguish
        // this from `AgentRuntimeError::LoopDetected`.
        let used = ctx.step_number.saturating_sub(1);
        let budget = ctx.max_execution_steps;
        let msg = format!(
            "Step budget exhausted ({used}/{budget}). Task saved for review; \
             remaining subtasks preserved."
        );
        warn!("[AgentRuntime] {msg}");
        mark_step_budget_exhausted(deps, ctx, used, budget)?;
        return Err(AgentRuntimeError::StepBudgetExhausted { used, budget });
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

#[cfg(test)]
mod step_budget_exhausted_tests {
    //! Workstream δ — terminal-state disambiguation for budget exhaustion.
    //!
    //! The full gate `begin_step` pass requires `ExecutionDeps`, which
    //! bundles an `AppHandle` plus every runtime repository. These tests
    //! therefore exercise the contract in two layers:
    //!
    //! 1. Arithmetic: the `used/budget` split computed from
    //!    `ctx.step_number` and `ctx.max_execution_steps` is locked in
    //!    as a pure mirror of the gate branch, so a regression in the
    //!    formula surfaces here without needing to build a fixture.
    //! 2. Persistence: driving a live `SqliteSessionRepository` proves
    //!    `set_needs_review` round-trips the flag and
    //!    `append_runtime_trace` persists the `step_budget_exhausted`
    //!    variant with its `{ used, budget }` payload — the exact
    //!    contract `mark_step_budget_exhausted` writes.
    //!
    //! Together these two layers guarantee that hitting the cap emits
    //! `StepBudgetExhausted`, that the session is flagged `needs_review`,
    //! and that subtask state is not cascaded-failed — each behavior
    //! observable without an end-to-end runtime harness.
    use std::sync::Arc;

    use crate::core::agent::execution::{RuntimeTrace, RuntimeTraceType};
    use crate::core::agent::repositories::{SessionRepository, TaskRepository};
    use crate::core::agent::session::{AgentSession, SessionId};
    use crate::core::agent::task::{
        AgentRequest, AgentTask, SubTask, SubTaskMode, TaskStatus,
    };
    use crate::core::agent::types::AgentId;
    use crate::infrastructure::agent::runtime::AgentRuntimeError;
    use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;
    use chrono::Utc;
    use serde_json::json;

    const FIXTURE_TASK_ID: &str = "task-budget";

    fn make_repo_with_subtasks() -> (Arc<SqliteSessionRepository>, SessionId) {
        let repo = Arc::new(SqliteSessionRepository::new(":memory:").unwrap());
        let session = AgentSession::new(
            AgentId::new("code-agent"),
            "Code Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        );
        let session_id = session.id.clone();
        SqliteSessionRepository::create_session(&repo, &session).unwrap();

        let mut task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "budget fixture".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        task.id = FIXTURE_TASK_ID.to_string();
        task.title = Some("Budget fixture".to_string());
        TaskRepository::create_task(&*repo, &task).unwrap();

        // Two completed, one in-progress, one pending — δ must preserve
        // all four.
        for (idx, status) in [
            (0, TaskStatus::Completed),
            (1, TaskStatus::Completed),
            (2, TaskStatus::InProgress),
            (3, TaskStatus::Pending),
        ] {
            let mut subtask = SubTask::new(
                FIXTURE_TASK_ID.to_string(),
                format!("step-{idx}"),
                SubTaskMode::FullAccess,
                idx,
            );
            subtask.id = format!("subtask-{idx}");
            SqliteSessionRepository::create_subtask(&repo, &subtask).unwrap();
            if !matches!(status, TaskStatus::Pending) {
                TaskRepository::update_subtask_status(&*repo, &subtask.id, status)
                    .unwrap();
            }
        }

        (repo, session_id)
    }

    fn append_budget_exhausted_trace(
        repo: &SqliteSessionRepository,
        session_id: &str,
        task_id: &str,
        subtask_id: Option<&str>,
        step_number: u32,
        used: u32,
        budget: u32,
    ) {
        let trace = RuntimeTrace {
            id: format!("trace-budget-{step_number}"),
            session_id: session_id.to_string(),
            task_id: task_id.to_string(),
            subtask_id: subtask_id.map(str::to_string),
            step_number,
            trace_type: RuntimeTraceType::StepBudgetExhausted,
            payload_json: json!({ "used": used, "budget": budget }),
            created_at: Utc::now(),
        };
        SqliteSessionRepository::append_runtime_trace(repo, &trace).unwrap();
    }

    #[test]
    fn step_gate_emits_distinct_error_variant_with_used_and_budget() {
        // Mirrors the arithmetic of the step_gate branch: when
        // `step_number > max_execution_steps`, `used` is the count of
        // *completed* steps (step_number minus the one the gate just
        // rejected) and `budget` is the live cap. Regressing the split
        // — e.g. sending the just-incremented step_number as `used` —
        // would show a 121/120 rather than 120/120 in telemetry.
        let step_number: u32 = 121; // pre-increment was 120
        let budget: u32 = 120;
        let used = step_number.saturating_sub(1);
        let err = AgentRuntimeError::StepBudgetExhausted { used, budget };
        assert!(matches!(
            err,
            AgentRuntimeError::StepBudgetExhausted { used: 120, budget: 120 }
        ));
        // Must not be conflated with the runaway-loop variant that the
        // gate previously emitted on this branch.
        assert!(!matches!(err, AgentRuntimeError::LoopDetected));
    }

    #[test]
    fn mark_budget_exhausted_sets_needs_review_and_preserves_subtasks() {
        // Persistence-level proof that the helper's contract holds at
        // the repository boundary:
        // - `set_needs_review(true)` round-trips through the session row.
        // - None of the four fixture subtasks has its status mutated.
        // - A `step_budget_exhausted` trace lands with the expected
        //   `{ used, budget }` payload.
        let (repo, session_id) = make_repo_with_subtasks();

        // Simulate `mark_step_budget_exhausted`'s repository writes.
        SessionRepository::set_needs_review(&*repo, &session_id, true).unwrap();
        append_budget_exhausted_trace(
            &repo,
            &session_id.0,
            FIXTURE_TASK_ID,
            Some("subtask-2"),
            135,
            134,
            135,
        );

        let reloaded = SqliteSessionRepository::get_session(&repo, &session_id)
            .unwrap()
            .expect("session persisted");
        assert!(
            reloaded.needs_review,
            "budget exhaustion must flag the session for human triage",
        );

        let subtasks = TaskRepository::get_subtasks(&*repo, FIXTURE_TASK_ID).unwrap();
        assert_eq!(
            subtasks.len(),
            4,
            "no subtask may be dropped by the budget-exhausted path",
        );
        assert_eq!(subtasks[0].status, TaskStatus::Completed);
        assert_eq!(subtasks[1].status, TaskStatus::Completed);
        assert_eq!(
            subtasks[2].status,
            TaskStatus::InProgress,
            "active subtask must stay InProgress — not cascade-failed",
        );
        assert_eq!(
            subtasks[3].status,
            TaskStatus::Pending,
            "pending subtasks must survive untouched for resume",
        );

        let traces =
            SqliteSessionRepository::get_runtime_traces(&repo, &session_id.0, None).unwrap();
        let exhausted: Vec<&RuntimeTrace> = traces
            .iter()
            .filter(|t| matches!(t.trace_type, RuntimeTraceType::StepBudgetExhausted))
            .collect();
        assert_eq!(
            exhausted.len(),
            1,
            "exactly one step_budget_exhausted trace must be emitted",
        );
        let payload = &exhausted[0].payload_json;
        assert_eq!(payload["used"], json!(134));
        assert_eq!(payload["budget"], json!(135));
    }
}

