//! Handles the LLM (Large Language Model) call phase of the agent's execution.
//!
//! This module is responsible for constructing the chat request, making the call
//! to the LLM with retry logic, and tracing the request and response for
//! observability.

use log::error;
use std::time::Instant;

use serde_json::json;

use super::deps::ExecutionDeps;
use super::state::{ExecutionContext, LlmCallResult, ResolvedContext};
use super::utils::{
    agent_temperature, truncate_text, LLM_MAX_OUTPUT_TOKENS, LLM_MAX_RETRIES, LLM_RETRY_BASE_MS,
};
use crate::core::agent::execution::{ExecutionPhase, LlmTracePayload, RuntimeTraceType};
use crate::core::llm::entities::{ChatRequest, ToolChoiceMode};
use crate::infrastructure::agent::registry::{harness_contract_prompt, quality_baseline_prompt};
use crate::infrastructure::agent::runtime::client::LlmRetryConfig;
use crate::infrastructure::agent::runtime::context::estimate_context_tokens;
use crate::infrastructure::agent::runtime::request_shape::{
    framing_hash, harness_section_hash, message_prefix_hash, quality_baseline_hash,
    system_prompt_hash, tool_catalog_hash,
};
use crate::infrastructure::agent::runtime::{chat_with_retry, AgentRuntimeError};

fn tool_choice_label(mode: ToolChoiceMode) -> String {
    match mode {
        ToolChoiceMode::Auto => "auto".to_string(),
        ToolChoiceMode::Required => "required".to_string(),
        ToolChoiceMode::None => "none".to_string(),
    }
}

/// Makes a call to the LLM with the resolved context.
///
/// This function constructs the `ChatRequest`, sends it to the LLM with retry logic,
/// and records detailed traces of the request and response for debugging and
/// observability purposes.
///
/// # Arguments
///
/// * `deps` - The execution dependencies.
/// * `ctx` - The mutable execution context.
/// * `resolved` - The resolved context from the previous phase.
/// * `client` - The LLM client to use for the call.
///
/// # Returns
///
/// A `Result` containing the `LlmCallResult` on success, or an `AgentRuntimeError` on failure.
pub async fn call_llm(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
    resolved: &ResolvedContext,
    client: &dyn crate::infrastructure::llm::LlmClient,
) -> Result<LlmCallResult, AgentRuntimeError> {
    let tracer = deps.tracer();
    ctx.current_phase = ExecutionPhase::CallingLlm;
    ctx.current_subtask_id = resolved.current_subtask_id.clone();

    // Consume any one-shot tool-choice override set by an earlier phase
    // (final-summary path or narration-loop nudge). Default is `Auto` so every
    // normal step leaves the model free to decide.
    let forced_tool_choice = ctx.pending_tool_choice_override.take();

    let mut chat_request = ChatRequest::new(&ctx.model_name, ctx.llm_context.clone())
        .with_system(&resolved.system_prompt)
        .with_temperature(agent_temperature())
        .with_max_tokens(LLM_MAX_OUTPUT_TOKENS);
    if !resolved.tool_definitions.is_empty() {
        chat_request = chat_request.with_tools(resolved.tool_definitions.clone());
    }
    if let Some(mode) = forced_tool_choice {
        chat_request = chat_request.with_tool_choice(mode);
    }

    let request_payload = LlmTracePayload {
        model: ctx.model_name.clone(),
        attempt: 0,
        temperature: agent_temperature(),
        max_tokens: LLM_MAX_OUTPUT_TOKENS,
        context_message_count: chat_request.messages.len(),
        estimated_context_tokens: estimate_context_tokens(&ctx.llm_context),
        active_subtask: resolved
            .current_subtask_summary
            .as_ref()
            .map(|(summary, _)| summary.title.clone()),
        allowed_tools: resolved
            .tool_definitions
            .iter()
            .map(|tool| tool.name.clone())
            .collect(),
        tool_count: resolved.tool_definitions.len(),
        system_prompt_hash: system_prompt_hash(&resolved.system_prompt),
        harness_section_hash: harness_section_hash(harness_contract_prompt()),
        quality_baseline_hash: quality_baseline_hash(quality_baseline_prompt()),
        framing_hash: framing_hash(&resolved.active_framing_section),
        tool_catalog_hash: tool_catalog_hash(&resolved.tool_definitions),
        tool_choice_mode: tool_choice_label(chat_request.tool_choice),
        message_prefix_hash: message_prefix_hash(&chat_request.messages),
        system_prompt: truncate_text(&resolved.system_prompt, 12000),
        request_messages: chat_request
            .messages
            .iter()
            .map(|message| {
                serde_json::to_value(message)
                    .unwrap_or_else(|_| json!({"error": "message_serialization_failed"}))
            })
            .collect(),
        tool_definitions: resolved
            .tool_definitions
            .iter()
            .map(|tool| {
                serde_json::to_value(tool)
                    .unwrap_or_else(|_| json!({"error": "tool_serialization_failed"}))
            })
            .collect(),
        response_content: None,
        response_tool_calls: Vec::new(),
        input_tokens: None,
        output_tokens: None,
        total_tokens: None,
        duration_ms: None,
        was_truncated: resolved.system_prompt.len() > 12000,
    };

    let _ = tracer.save_snapshot(&ctx.snapshot(
        ExecutionPhase::CallingLlm,
        Some(ctx.model_name.clone()),
        None,
    ));
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::LlmRequestPrepared,
        serde_json::to_value(&request_payload)
            .unwrap_or_else(|_| json!({"error": "llm_trace_serialization_failed"})),
    );

    let retry_config = LlmRetryConfig {
        max_retries: LLM_MAX_RETRIES,
        base_delay_ms: LLM_RETRY_BASE_MS,
    };
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::LlmRequestStarted,
        json!({"maxRetries": LLM_MAX_RETRIES, "baseDelayMs": LLM_RETRY_BASE_MS}),
    );

    let start = Instant::now();
    let response = chat_with_retry(client, &chat_request, &retry_config)
        .await
        .map_err(|e| {
            error!(
                "[LLM Phase] LLM call failed for session '{}', task '{}', step {}: {}",
                ctx.session_id, ctx.task_id, ctx.step_number, e
            );
            e
        })?;
    let duration_ms = start.elapsed().as_millis() as u64;
    if let Some(usage) = &response.usage {
        ctx.total_input_tokens += usage.input_tokens;
        ctx.total_output_tokens += usage.output_tokens;
    }

    let response_payload = LlmTracePayload {
        model: response.model.clone(),
        attempt: 0,
        temperature: agent_temperature(),
        max_tokens: LLM_MAX_OUTPUT_TOKENS,
        context_message_count: chat_request.messages.len(),
        estimated_context_tokens: estimate_context_tokens(&ctx.llm_context),
        active_subtask: resolved
            .current_subtask_summary
            .as_ref()
            .map(|(summary, _)| summary.title.clone()),
        allowed_tools: resolved
            .tool_definitions
            .iter()
            .map(|tool| tool.name.clone())
            .collect(),
        tool_count: resolved.tool_definitions.len(),
        system_prompt_hash: system_prompt_hash(&resolved.system_prompt),
        harness_section_hash: harness_section_hash(harness_contract_prompt()),
        quality_baseline_hash: quality_baseline_hash(quality_baseline_prompt()),
        framing_hash: framing_hash(&resolved.active_framing_section),
        tool_catalog_hash: tool_catalog_hash(&resolved.tool_definitions),
        tool_choice_mode: tool_choice_label(chat_request.tool_choice),
        message_prefix_hash: message_prefix_hash(&chat_request.messages),
        system_prompt: truncate_text(&resolved.system_prompt, 12000),
        request_messages: chat_request
            .messages
            .iter()
            .map(|message| {
                serde_json::to_value(message)
                    .unwrap_or_else(|_| json!({"error": "message_serialization_failed"}))
            })
            .collect(),
        tool_definitions: resolved
            .tool_definitions
            .iter()
            .map(|tool| {
                serde_json::to_value(tool)
                    .unwrap_or_else(|_| json!({"error": "tool_serialization_failed"}))
            })
            .collect(),
        response_content: Some(truncate_text(&response.content, 20000)),
        response_tool_calls: response
            .tool_calls
            .iter()
            .map(|call| {
                serde_json::to_value(call)
                    .unwrap_or_else(|_| json!({"error": "tool_call_serialization_failed"}))
            })
            .collect(),
        input_tokens: response.usage.as_ref().map(|usage| usage.input_tokens),
        output_tokens: response.usage.as_ref().map(|usage| usage.output_tokens),
        total_tokens: response.usage.as_ref().map(|usage| usage.total_tokens),
        duration_ms: Some(duration_ms),
        was_truncated: response.content.len() > 20000,
    };
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::LlmResponseReceived,
        serde_json::to_value(&response_payload)
            .unwrap_or_else(|_| json!({"error": "llm_trace_serialization_failed"})),
    );

    ctx.current_phase = ExecutionPhase::HandlingLlmResponse;
    let _ = tracer.save_snapshot(&ctx.snapshot(
        ExecutionPhase::HandlingLlmResponse,
        Some(response.model.clone()),
        None,
    ));

    Ok(LlmCallResult {
        response,
        duration_ms,
        request_payload,
        response_payload,
    })
}
