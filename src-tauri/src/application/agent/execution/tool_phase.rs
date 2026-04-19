//! Handles the tool execution phase of the agent's execution.
//!
//! This module is responsible for processing the tool calls requested by the LLM,
//! managing permissions, and updating the execution context with the results.

use serde_json::json;

use super::deps::ExecutionDeps;
use super::replace_drift::evaluate_replace_drift;
use super::scope_drift::{
    build_correction_message, evaluate_scope_drift, hard_enforced_error_result, EnforcementMode,
    ScopeDriftObservation,
};
use super::state::ExecutionContext;
use super::termination::cancel_execution;
use super::utils::MALFORMED_ARGS_ABORT_THRESHOLD;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::permissions::PermissionResolver;
use crate::core::agent::task::SubTaskContract;
use crate::core::llm::entities::{ChatMessage as Message, ChatResponse, ToolCall};
use crate::infrastructure::agent::runtime::permissions::YoloModeProvider;
use crate::infrastructure::agent::runtime::{
    add_to_recent_calls, process_tool_calls_with_permissions, AgentRuntimeError,
};
use crate::infrastructure::llm::malformed_args::extract_marker;
use crate::infrastructure::tools::OrchestratorToolResult;

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

    let mut tool_results = process_tool_calls_with_permissions(
        &response.tool_calls,
        session,
        &ctx.task_id,
        ctx.step_number,
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
        &mut ctx.runtime_session_scope,
    )
    .await?;

    // Track consecutive steps where *every* tool call carried the malformed-
    // arguments sentinel. The recovery path in `permissions.rs` returns an
    // error-flagged tool result per bad call so the LLM can self-correct;
    // this guard caps how many rounds we tolerate before aborting. Steps
    // without any tool calls do not reset the counter — they belong to a
    // different failure mode handled by the narration-loop guard.
    if !response.tool_calls.is_empty() {
        let all_malformed = response
            .tool_calls
            .iter()
            .all(|call| extract_marker(&call.arguments).is_some());
        if all_malformed {
            ctx.consecutive_malformed_tool_args =
                ctx.consecutive_malformed_tool_args.saturating_add(1);
            let _ = tracer.append_trace(
                &ctx.session_id,
                &ctx.task_id,
                ctx.current_subtask_id.as_deref(),
                ctx.step_number,
                RuntimeTraceType::ToolBatchCompleted,
                json!({
                    "malformedToolArgsStreak": ctx.consecutive_malformed_tool_args,
                    "threshold": MALFORMED_ARGS_ABORT_THRESHOLD,
                    "toolNames": response
                        .tool_calls
                        .iter()
                        .map(|call| call.name.clone())
                        .collect::<Vec<_>>(),
                }),
            );
            if ctx.consecutive_malformed_tool_args >= MALFORMED_ARGS_ABORT_THRESHOLD {
                let last_tool = response
                    .tool_calls
                    .last()
                    .map(|call| call.name.clone())
                    .unwrap_or_default();
                return Err(AgentRuntimeError::MalformedToolArgumentsExhausted {
                    attempts: ctx.consecutive_malformed_tool_args,
                    last_tool,
                });
            }
        } else {
            ctx.consecutive_malformed_tool_args = 0;
        }
    }

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

    // Workstream B.3 + B.4 — evaluate scope drift against the active
    // subtask's contract. Soft enforcement emits a trace and queues a
    // correction system message; hard enforcement (gated on the
    // `CODER_SUBTASK_CONTRACT_HARD_ENFORCEMENT` env flag) additionally
    // rewrites the tool result as a `subtask_scope_drift` error. The
    // flag ships disabled by default.
    let contract = load_active_contract(
        deps,
        ctx.task_id.as_str(),
        ctx.current_subtask_id.as_deref(),
    );
    let enforcement = EnforcementMode::from_env();
    let corrections = apply_scope_drift_detection(
        deps,
        ctx,
        &response.tool_calls,
        &mut tool_results,
        contract.as_ref(),
        enforcement,
    );

    // Workstream γ.2 — soft-nudge the model when repeated
    // `fs_replace_in_file` calls miss on the same file within a five-
    // step window. The file is almost certainly out of sync with the
    // model's working copy; read the tool results in place and append
    // the correction as a system message for the next step. The
    // tracker mutation (prune + append + clear on fire) lives inside
    // `evaluate_replace_drift` so this call site stays tiny.
    let replace_outcome = evaluate_replace_drift(
        &mut ctx.recent_replace_failures,
        &response.tool_calls,
        &tool_results,
        ctx.step_number,
    );
    if replace_outcome.nudge.is_some() {
        let _ = tracer.append_trace(
            &ctx.session_id,
            &ctx.task_id,
            ctx.current_subtask_id.as_deref(),
            ctx.step_number,
            RuntimeTraceType::ReplaceDriftDetected,
            json!({
                "path": replace_outcome.cleared_path,
            }),
        );
    }

    for result in &tool_results {
        ctx.llm_context.push(Message::tool_result(
            result.call_id.clone(),
            result.output.clone(),
            result.is_error,
        ));
    }
    for correction in corrections {
        ctx.llm_context.push(Message::system(correction));
    }
    if let Some(nudge) = replace_outcome.nudge {
        ctx.llm_context.push(Message::system(nudge));
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

/// Loads the active subtask's declared contract (if any). A missing
/// subtask, a subtask without a contract, or a repository lookup
/// failure all resolve to `None` — the scope-drift detector treats
/// that identically and short-circuits, preserving legacy behavior.
fn load_active_contract(
    deps: &ExecutionDeps<'_>,
    task_id: &str,
    current_subtask_id: Option<&str>,
) -> Option<SubTaskContract> {
    let subtask_id = current_subtask_id?;
    deps.task_repo
        .get_subtasks(task_id)
        .ok()?
        .into_iter()
        .find(|st| st.id == subtask_id)
        .and_then(|st| st.contract)
}

/// Runs scope-drift detection across every tool result in the batch.
/// Returns the list of correction system-message bodies that should
/// be appended to the context, in the same order the drifts
/// occurred. Hard enforcement (`EnforcementMode::Hard`) mutates
/// `tool_results` in place so the drifting write is surfaced as a
/// `subtask_scope_drift` error rather than a successful write.
fn apply_scope_drift_detection(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    tool_calls: &[ToolCall],
    tool_results: &mut [OrchestratorToolResult],
    contract: Option<&SubTaskContract>,
    enforcement: EnforcementMode,
) -> Vec<String> {
    if contract.is_none() {
        return Vec::new();
    }
    let tracer = deps.tracer();
    let mut corrections = Vec::new();
    for (call, result) in tool_calls.iter().zip(tool_results.iter_mut()) {
        let Some(observation) = evaluate_scope_drift(call, result, contract) else {
            continue;
        };
        emit_scope_drift_trace(&tracer, ctx, &observation, enforcement);
        if enforcement == EnforcementMode::Hard {
            *result = hard_enforced_error_result(call, &observation);
        }
        corrections.push(build_correction_message(&observation));
    }
    corrections
}

/// Appends a `subtask_scope_drift` runtime trace for the given
/// observation. Payload shape matches the documented schema on
/// `RuntimeTraceType::SubtaskScopeDrift`.
fn emit_scope_drift_trace(
    tracer: &super::tracing::RuntimeTraceService<'_>,
    ctx: &ExecutionContext,
    observation: &ScopeDriftObservation,
    enforcement: EnforcementMode,
) {
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::SubtaskScopeDrift,
        json!({
            "toolName": observation.tool_name,
            "writtenPath": observation.written_path,
            "expectedPatterns": observation.expected_patterns,
            "enforcement": enforcement.label(),
        }),
    );
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
