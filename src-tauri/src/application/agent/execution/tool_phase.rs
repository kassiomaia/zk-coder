use serde_json::json;

use super::deps::ExecutionDeps;
use super::state::ExecutionContext;
use super::termination::cancel_execution;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::permissions::PermissionResolver;
use crate::core::llm::entities::{ChatMessage as Message, ChatResponse};
use crate::infrastructure::agent::runtime::{
    add_to_recent_calls, process_tool_calls_with_permissions, AgentRuntimeError,
};

pub enum ToolPhaseOutcome {
    Continue,
    Cancelled,
}

pub async fn run_tools(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
    session: &crate::core::agent::session::AgentSession,
    response: ChatResponse,
    permission_resolver: &PermissionResolver,
) -> Result<ToolPhaseOutcome, AgentRuntimeError> {
    let tracer = deps.tracer();
    add_to_recent_calls(&response.tool_calls, &mut ctx.recent_tool_calls, 10);
    ctx.llm_context.push(Message::assistant_with_tool_calls(
        response.content,
        response.tool_calls.clone(),
    ));

    ctx.current_phase = ExecutionPhase::ExecutingTools;
    let _ = tracer.save_snapshot(&ctx.snapshot(
        ExecutionPhase::ExecutingTools,
        Some(response.model.clone()),
        None,
    ));
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ToolBatchStarted,
        json!({
            "toolCallCount": response.tool_calls.len(),
            "toolNames": response.tool_calls.iter().map(|call| call.name.clone()).collect::<Vec<_>>(),
        }),
    );

    let tool_results = process_tool_calls_with_permissions(
        &response.tool_calls,
        session,
        &ctx.task_id,
        permission_resolver,
        deps.app_handle,
        deps.approval_queue,
        deps.tool_registry,
        deps.file_snapshot_registry,
        deps.session_repo,
        deps.telemetry_repo,
        deps.task_repo,
    )
    .await?;

    let session = deps
        .session_repo
        .get_session(&crate::core::agent::entities::SessionId(ctx.session_id.clone()))
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    if matches!(
        session.state,
        crate::core::agent::entities::SessionState::Cancelled { .. }
    ) {
        cancel_execution(deps, ctx, "session_cancelled_after_tools")?;
        return Ok(ToolPhaseOutcome::Cancelled);
    }

    for result in &tool_results {
        ctx.llm_context.push(Message::tool_result(
            result.call_id.clone(),
            result.output.clone(),
            result.is_error,
        ));
    }
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ToolBatchCompleted,
        json!({
            "resultCount": tool_results.len(),
            "errors": tool_results.iter().filter(|result| result.is_error).count(),
        }),
    );
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::StepFinished,
        json!({"phase": "executing_tools"}),
    );

    Ok(ToolPhaseOutcome::Continue)
}
