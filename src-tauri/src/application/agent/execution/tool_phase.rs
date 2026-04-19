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
use crate::infrastructure::agent::runtime::permissions::YoloModeProvider;
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
    yolo_mode_provider: &dyn YoloModeProvider,
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

    let disallowed_by_mode: Vec<String> = ctx
        .current_allowed_tool_names
        .as_ref()
        .map(|allowed| {
            response
                .tool_calls
                .iter()
                .filter(|call| !allowed.iter().any(|name| name == &call.name))
                .map(|call| call.name.clone())
                .collect()
        })
        .unwrap_or_default();

    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ToolBatchStarted,
        json!({
            "toolCallCount": response.tool_calls.len(),
            "toolNames": response.tool_calls.iter().map(|call| call.name.clone()).collect::<Vec<_>>(),
            "disallowedByMode": disallowed_by_mode,
            "allowedToolNames": ctx.current_allowed_tool_names,
        }),
    );

    let tool_results = process_tool_calls_with_permissions(
        &response.tool_calls,
        session,
        &ctx.task_id,
        permission_resolver,
        yolo_mode_provider,
        ctx.current_allowed_tool_names.as_deref(),
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
        .get_session(&crate::core::agent::session::SessionId(
            ctx.session_id.clone(),
        ))
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    if matches!(
        session.state,
        crate::core::agent::session::SessionState::Cancelled { .. }
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
    // Workstream C.1/C.2 — record the most recent failure into the
    // subtask-scoped overlay so the packer can filter stale blockers on
    // subtask transitions instead of re-surfacing them via the raw
    // context scan.
    record_memorized_failure(ctx);
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ToolBatchCompleted,
        json!({
            "resultCount": tool_results.len(),
            "errors": tool_results.iter().filter(|result| result.is_error).count(),
            "disallowedByModeCount": disallowed_by_mode.len(),
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

/// Maximum number of entries retained in the `memorized_failures`
/// overlay. The filter always walks the most recent entries first, so
/// capping the tail is sufficient; older entries are dropped silently.
const MEMORIZED_FAILURES_CAP: usize = 32;

/// Extracts the most recent failure from the LLM context (if any) and
/// appends it to the `MemorizedFailure` overlay on `ctx`, tagged with
/// the current subtask id at record time.
///
/// Idempotent across calls on the same context tail: if the last entry
/// already captures the exact same failure (same tool, same args
/// fingerprint, same subtask, same message), the append is skipped so
/// a tool phase that runs back-to-back without a new error does not
/// duplicate the record. Only genuinely new failures grow the overlay.
fn record_memorized_failure(ctx: &mut super::state::ExecutionContext) {
    use crate::infrastructure::agent::runtime::context::extract_failure_record;
    let Some(record) = extract_failure_record(&ctx.llm_context) else {
        return;
    };
    let current_subtask_id = ctx.current_subtask_id.clone();
    if let Some(last) = ctx.memorized_failures.last() {
        if last.subtask_id == current_subtask_id
            && last.record.tool_name == record.tool_name
            && last.record.args_fingerprint == record.args_fingerprint
            && last.record.message == record.message
        {
            return;
        }
    }
    ctx.memorized_failures
        .push(crate::infrastructure::agent::runtime::context::MemorizedFailure {
            subtask_id: current_subtask_id,
            record,
        });
    let len = ctx.memorized_failures.len();
    if len > MEMORIZED_FAILURES_CAP {
        let drop_count = len - MEMORIZED_FAILURES_CAP;
        ctx.memorized_failures.drain(0..drop_count);
    }
}
