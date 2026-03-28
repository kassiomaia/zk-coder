//! Handles the tool execution phase of the agent's execution.
//!
//! This module is responsible for processing the tool calls requested by the LLM,
//! managing permissions, and updating the execution context with the results.

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

/// Represents the outcome of the tool execution phase.
pub enum ToolPhaseOutcome {
    /// The execution should continue to the next step.
    Continue,
    /// The execution has been cancelled.
    Cancelled,
}

/// Runs the tools requested by the LLM and updates the execution context.
///
/// This function performs the following steps:
/// 1.  Adds the new tool calls to the list of recent calls for loop detection.
/// 2.  Updates the LLM context with the assistant's message containing the tool calls.
/// 3.  Processes the tool calls, handling permissions and queuing approvals if necessary.
/// 4.  Checks if the session was cancelled while waiting for tool execution.
/// 5.  Updates the LLM context with the results of the tool calls.
/// 6.  Records trace events for the tool batch execution.
///
/// # Arguments
///
/// * `deps` - The execution dependencies.
/// * `ctx` - The mutable execution context.
/// * `session` - The current agent session.
/// * `response` - The LLM response containing the tool calls.
/// * `permission_resolver` - The resolver for checking tool permissions.
///
/// # Returns
///
/// A `Result` containing a `ToolPhaseOutcome` on success, or an `AgentRuntimeError` on failure.
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
        .get_session(&crate::core::agent::entities::SessionId(
            ctx.session_id.clone(),
        ))
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
