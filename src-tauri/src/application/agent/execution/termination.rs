use chrono::Utc;
use serde_json::json;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::state::ExecutionContext;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::{SessionId, SessionState};
use crate::core::agent::task::{AgentExecutionResult, TaskStatus, UsageStats};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::AgentRuntimeError;

pub fn update_session_state(
    deps: &ExecutionDeps<'_>,
    session_id: &SessionId,
    state: SessionState,
) -> Result<(), AgentRuntimeError> {
    let core_state: crate::core::agent::entities::SessionState = state.clone().into();
    deps.session_repo
        .update_state(
            &crate::core::agent::entities::SessionId(session_id.0.clone()),
            &core_state,
        )
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionStateChanged {
            session_id: session_id.0.clone(),
            state,
        },
    );

    Ok(())
}

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
        crate::core::agent::entities::TaskStatus::Cancelled,
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

pub fn fail_execution(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    error: &str,
    kind: &str,
    last_model: Option<String>,
) -> Result<(), AgentRuntimeError> {
    update_session_state(
        deps,
        &SessionId(ctx.session_id.clone()),
        SessionState::Error {
            message: error.to_string(),
            recoverable: true,
        },
    )?;

    let _ = deps.task_repo.update_task_status(
        &ctx.task_id,
        crate::core::agent::entities::TaskStatus::Failed,
    );
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

pub fn complete_execution(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    conversation_id: &str,
    thread_id: &str,
    content: String,
) -> Result<AgentExecutionResult, AgentRuntimeError> {
    let result = AgentExecutionResult {
        content,
        steps: ctx.steps.clone(),
        usage: Some(UsageStats {
            input_tokens: ctx.total_input_tokens,
            output_tokens: ctx.total_output_tokens,
            total_tokens: ctx.total_input_tokens + ctx.total_output_tokens,
        }),
    };

    update_session_state(
        deps,
        &SessionId(ctx.session_id.clone()),
        SessionState::Finished {
            completed_at: Utc::now(),
        },
    )?;
    let _ = deps.task_repo.update_task_status(
        &ctx.task_id,
        crate::core::agent::entities::TaskStatus::Completed,
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
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ExecutionComplete {
            session_id: ctx.session_id.clone(),
            conversation_id: conversation_id.to_string(),
            thread_id: thread_id.to_string(),
            result: result.clone(),
        },
    );

    Ok(result)
}
