//! Handles the response phase of the agent's execution.
//!
//! This module is responsible for processing the response from the LLM,
//! determining the next course of action, and handling the completion or
//! continuation of the execution.

use chrono::Utc;
use serde_json::json;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::state::{ExecutionContext, LlmCallResult};
use super::termination::{complete_execution, emit_streaming_message, fail_execution};
use crate::core::agent::execution::RuntimeTraceType;
use crate::core::agent::task::{AgentExecutionResult, ExecutionStep, StepAction, StepResult};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::{
    detect_repetitive_loop, AgentRuntimeError, MAX_REPETITIONS,
};

/// Represents the decision made after handling the LLM's response.
pub enum ResponseDecision {
    /// The execution has finished, and this is the final result.
    Finished(AgentExecutionResult),
    /// The execution should continue by executing the specified tools.
    ExecuteTools(crate::core::llm::entities::ChatResponse),
}

/// Handles the response from the LLM and determines the next step.
///
/// This function processes the `LlmCallResult` by:
/// 1.  Recording the LLM execution step and emitting an event.
/// 2.  Checking if the response contains tool calls.
/// 3.  If there are no tool calls, it completes the execution and returns the final result.
/// 4.  If there are tool calls, it checks for repetitive loops to prevent infinite execution.
/// 5.  If any intermediate content was generated, it streams it to the frontend.
/// 6.  Finally, it returns a decision to either finish the execution or proceed with tool execution.
///
/// # Arguments
///
/// * `deps` - The execution dependencies.
/// * `ctx` - The mutable execution context.
/// * `session` - The current agent session.
/// * `llm_result` - The result of the LLM call.
///
/// # Returns
///
/// A `Result` containing a `ResponseDecision` on success, or an `AgentRuntimeError` on failure.
pub async fn handle_response(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
    session: &crate::core::agent::session::AgentSession,
    llm_result: LlmCallResult,
) -> Result<ResponseDecision, AgentRuntimeError> {
    let tracer = deps.tracer();
    let response = llm_result.response;

    let llm_step = ExecutionStep {
        step_number: ctx.step_number,
        action: StepAction::LlmRequest {
            model: response.model.clone(),
        },
        result: Some(StepResult {
            success: true,
            output: json!({
                "content_length": response.content.len(),
                "tool_calls": response.tool_calls.len(),
            }),
            tokens_used: response.usage.as_ref().map(|usage| usage.total_tokens),
            input_tokens: response.usage.as_ref().map(|usage| usage.input_tokens),
            output_tokens: response.usage.as_ref().map(|usage| usage.output_tokens),
            duration_ms: llm_result.duration_ms,
        }),
        timestamp: Utc::now(),
    };

    if let Err(error) = deps.telemetry_repo.create_execution_step(
        &llm_step,
        &ctx.session_id,
        Some(&ctx.task_id),
        ctx.current_subtask_id.as_deref(),
    ) {
        log::warn!("[AgentRuntime] Failed to persist execution step: {error}");
    }
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ExecutionStep {
            session_id: ctx.session_id.clone(),
            step: llm_step.clone(),
        },
    );
    ctx.steps.push(llm_step);

    if response.tool_calls.is_empty() {
        let _ = tracer.append_trace(
            &ctx.session_id,
            &ctx.task_id,
            ctx.current_subtask_id.as_deref(),
            ctx.step_number,
            RuntimeTraceType::LlmResponseClassified,
            json!({"hasToolCalls": false, "willFinalize": true}),
        );
        let result = complete_execution(
            deps,
            ctx,
            &session.conversation_id,
            &session.thread_id,
            response.content,
        )
        .await?;
        return Ok(ResponseDecision::Finished(result));
    }

    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::LlmResponseClassified,
        json!({
            "hasToolCalls": true,
            "toolCallCount": response.tool_calls.len(),
            "toolNames": response.tool_calls.iter().map(|call| call.name.clone()).collect::<Vec<_>>(),
        }),
    );

    for tool_call in &response.tool_calls {
        if let Some(repeat_count) =
            detect_repetitive_loop(tool_call, &ctx.recent_tool_calls, MAX_REPETITIONS)
        {
            let msg = format!(
                "Detected repetitive loop: tool '{}' called with same arguments {} times",
                tool_call.name,
                repeat_count + 1
            );
            fail_execution(deps, ctx, &msg, "loop_detected", Some(response.model.clone()))?;
            return Err(AgentRuntimeError::LoopDetected);
        }
    }

    if !response.content.trim().is_empty() {
        emit_streaming_message(
            deps,
            &ctx.session_id,
            &session.conversation_id,
            &session.thread_id,
            &response.content,
            true,
            Some(format!(
                "streaming-tool-preamble-{}-{}",
                ctx.session_id, ctx.step_number
            )),
            true,
        )
        .await;
    }

    Ok(ResponseDecision::ExecuteTools(response))
}
