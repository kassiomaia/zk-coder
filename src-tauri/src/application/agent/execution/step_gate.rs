use serde_json::json;

use super::deps::ExecutionDeps;
use super::state::ExecutionContext;
use super::termination::{cancel_execution, fail_execution, update_session_state};
use super::utils::MAX_EXECUTION_STEPS;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::SessionState;
use crate::infrastructure::agent::runtime::context::{estimate_context_tokens, CONTEXT_WINDOW};
use crate::infrastructure::agent::runtime::AgentRuntimeError;
use crate::infrastructure::agent::runtime::loop_detection::MAX_REPETITIONS;
use log::{info, warn};

use crate::core::agent::session::SessionId;

pub enum StepGateOutcome {
    Proceed,
    Cancelled,
}

pub fn begin_step(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
) -> Result<StepGateOutcome, AgentRuntimeError> {
    let session = deps
        .session_repo
        .get_session(&crate::core::agent::entities::SessionId(ctx.session_id.clone()))
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    if matches!(
        session.state,
        crate::core::agent::entities::SessionState::Cancelled { .. }
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

    if ctx.step_number > MAX_EXECUTION_STEPS {
        let msg = format!(
            "Execution step limit reached ({}). Aborting to prevent runaway loop.",
            MAX_EXECUTION_STEPS
        );
        warn!("[AgentRuntime] {}", msg);
        fail_execution(deps, ctx, &msg, "step_limit", None)?;
        return Err(AgentRuntimeError::LoopDetected);
    }

    info!("[AgentRuntime] Step {} for session {}", ctx.step_number, ctx.session_id);
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
