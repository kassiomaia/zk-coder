//! Handles the context resolution phase of the agent's execution.
//!
//! This module is responsible for preparing the context for the LLM call,
//! which includes summarizing the current subtask, managing the context window,
//! performing retrieval passes, and deriving the system prompt.

use log::info;
use serde_json::json;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::state::{ExecutionContext, ResolvedContext};
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::AgentRequest;
use crate::core::llm::entities::{ToolCall, ToolDefinition};
use crate::infrastructure::agent::events::{AgentEvent, ContextCompactionReason, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::context::{
    apply_context_policy_v2, estimate_context_tokens, maybe_compact_context, ContextPolicyInput,
    COMPRESSION_THRESHOLD, CONTEXT_WINDOW,
};
use crate::infrastructure::agent::runtime::{
    derive_system_prompt, AgentRuntimeError, SubtaskSummary,
};
use crate::infrastructure::tools::context::ToolExecutionContext;

/// Resolves the context for the current step of the execution.
///
/// This function performs several key tasks:
/// 1. Determines the current subtask and its allowed tools.
/// 2. Decides whether to perform a retrieval pass based on context window usage and subtask changes.
/// 3. Applies a context policy to compact the context window if necessary.
/// 4. Derives the system prompt based on the agent's system prompt and the current subtask.
/// 5. Filters the available tool definitions based on the current subtask's permissions.
///
/// # Arguments
///
/// * `deps` - The execution dependencies.
/// * `ctx` - The mutable execution context.
/// * `session` - The current agent session.
/// * `request` - The user's request that initiated the execution.
/// * `agent_system_prompt` - The base system prompt for the agent.
///
/// # Returns
///
/// A `Result` containing the `ResolvedContext` on success, or an `AgentRuntimeError` on failure.
pub async fn resolve_context(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
    session: &AgentSession,
    request: &AgentRequest,
    agent_system_prompt: &str,
) -> Result<ResolvedContext, AgentRuntimeError> {
    ctx.current_phase = ExecutionPhase::ResolvingContext;
    let tracer = deps.tracer();

    let (current_subtask_summary, current_subtask_id, tool_names) =
        if let Ok(subtasks) = deps.task_repo.get_subtasks(&ctx.task_id) {
            let total = subtasks.len();
            if let Some(current) = subtasks
                .iter()
                .find(|st| st.status == crate::core::agent::entities::TaskStatus::InProgress)
            {
                let summary = SubtaskSummary::from_subtask(current);
                let allowed = summary
                    .mode
                    .allowed_tools()
                    .into_iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>();
                (Some((summary, total)), Some(current.id.clone()), allowed)
            } else {
                (
                    None,
                    None,
                    deps.all_tool_definitions
                        .iter()
                        .map(|tool| tool.name.clone())
                        .collect::<Vec<_>>(),
                )
            }
        } else {
            (
                None,
                None,
                deps.all_tool_definitions
                    .iter()
                    .map(|tool| tool.name.clone())
                    .collect::<Vec<_>>(),
            )
        };

    let current_order = current_subtask_summary
        .as_ref()
        .map(|(summary, _)| summary.order);
    let subtask_changed = ctx.last_injected_subtask_order != current_order;
    ctx.current_subtask_id = current_subtask_id.clone();

    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::SubtaskContextResolved,
        json!({
            "subtaskChanged": subtask_changed,
            "activeSubtask": current_subtask_summary.as_ref().map(|(summary, total)| json!({
                "title": summary.title,
                "mode": summary.mode,
                "order": summary.order,
                "total": total,
            })),
            "allowedToolNames": tool_names,
        }),
    );

    let usage_ratio = estimate_context_tokens(&ctx.llm_context) as f64 / CONTEXT_WINDOW as f64;
    let should_retrieve = usage_ratio >= COMPRESSION_THRESHOLD || subtask_changed;
    let retrieval_snippets = if should_retrieve {
        let _ = tracer.append_trace(
            &ctx.session_id,
            &ctx.task_id,
            current_subtask_id.as_deref(),
            ctx.step_number,
            RuntimeTraceType::RetrievalStarted,
            json!({"usageRatio": usage_ratio, "subtaskChanged": subtask_changed}),
        );
        run_retrieval_pass(
            deps,
            session,
            &request.content,
            current_subtask_summary
                .as_ref()
                .map(|(summary, _)| summary.title.as_str()),
        )
        .await
    } else {
        Vec::new()
    };
    if should_retrieve {
        let _ = tracer.append_trace(
            &ctx.session_id,
            &ctx.task_id,
            current_subtask_id.as_deref(),
            ctx.step_number,
            RuntimeTraceType::RetrievalCompleted,
            json!({
                "snippetCount": retrieval_snippets.len(),
                "snippets": retrieval_snippets.iter().take(20).cloned().collect::<Vec<_>>(),
            }),
        );
    }

    let policy_input = ContextPolicyInput {
        force_pack: subtask_changed,
        task_goal: Some(request.content.clone()),
        active_subtask: current_subtask_summary
            .as_ref()
            .map(|(summary, _)| summary.title.clone()),
        retrieval_snippets,
    };

    if let Some(compaction) = apply_context_policy_v2(&mut ctx.llm_context, &policy_input) {
        let _ = deps.app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ContextCompacted {
                session_id: ctx.session_id.clone(),
                conversation_id: session.conversation_id.clone(),
                reason: if usage_ratio >= COMPRESSION_THRESHOLD {
                    ContextCompactionReason::TokenLimit
                } else {
                    ContextCompactionReason::SubtaskTransition
                },
                before_count: compaction.before_count,
                after_count: compaction.after_count,
                before_tokens: compaction.before_tokens,
                after_tokens: compaction.after_tokens,
                timestamp: chrono::Utc::now(),
            },
        );
        let _ = tracer.append_trace(
            &ctx.session_id,
            &ctx.task_id,
            current_subtask_id.as_deref(),
            ctx.step_number,
            RuntimeTraceType::ContextPolicyEvaluated,
            json!({
                "strategy": "v2",
                "reason": if usage_ratio >= COMPRESSION_THRESHOLD { "token_limit" } else { "subtask_transition" },
                "beforeCount": compaction.before_count,
                "afterCount": compaction.after_count,
                "beforeTokens": compaction.before_tokens,
                "afterTokens": compaction.after_tokens,
            }),
        );
    } else if usage_ratio >= COMPRESSION_THRESHOLD {
        if let Some(compaction) = maybe_compact_context(&mut ctx.llm_context) {
            let _ = deps.app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::ContextCompacted {
                    session_id: ctx.session_id.clone(),
                    conversation_id: session.conversation_id.clone(),
                    reason: ContextCompactionReason::TokenLimit,
                    before_count: compaction.before_count,
                    after_count: compaction.after_count,
                    before_tokens: compaction.before_tokens,
                    after_tokens: compaction.after_tokens,
                    timestamp: chrono::Utc::now(),
                },
            );
            let _ = tracer.append_trace(
                &ctx.session_id,
                &ctx.task_id,
                current_subtask_id.as_deref(),
                ctx.step_number,
                RuntimeTraceType::ContextPolicyEvaluated,
                json!({
                    "strategy": "legacy",
                    "reason": "token_limit",
                    "beforeCount": compaction.before_count,
                    "afterCount": compaction.after_count,
                    "beforeTokens": compaction.before_tokens,
                    "afterTokens": compaction.after_tokens,
                }),
            );
        }
    }

    ctx.last_injected_subtask_order = current_order;
    let system_prompt = derive_system_prompt(agent_system_prompt, &current_subtask_summary);
    info!(
        "[AgentRuntime] System prompt length: {}{}",
        system_prompt.len(),
        if current_subtask_summary.is_some() {
            " (with subtask context)"
        } else {
            ""
        }
    );

    let tool_definitions: Vec<ToolDefinition> = deps
        .all_tool_definitions
        .iter()
        .filter(|tool| tool_names.iter().any(|name| name == &tool.name))
        .cloned()
        .collect();
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ToolsetFiltered,
        json!({
            "toolCount": tool_definitions.len(),
            "tools": tool_definitions.iter().map(|tool| tool.name.clone()).collect::<Vec<_>>(),
        }),
    );

    Ok(ResolvedContext {
        current_subtask_summary,
        current_subtask_id,
        tool_definitions,
        system_prompt,
    })
}

async fn run_retrieval_pass(
    deps: &ExecutionDeps<'_>,
    session: &AgentSession,
    request_content: &str,
    active_subtask_title: Option<&str>,
) -> Vec<String> {
    let query = build_retrieval_query(request_content, active_subtask_title);
    if query.is_empty() {
        return Vec::new();
    }

    let call = ToolCall {
        id: uuid::Uuid::new_v4().to_string(),
        name: "search_grep".to_string(),
        arguments: json!({
            "query": query,
            "max_results": 12,
            "case_sensitive": false
        }),
    };
    let context = ToolExecutionContext::new(&session.conversation_id)
        .with_thread(session.thread_id.clone())
        .with_session(session.id.0.clone());
    let context = if let Some(workspace) = &session.workspace_root {
        context.with_workspace(workspace.clone())
    } else {
        context
    };

    let result = deps
        .tool_registry
        .execute(call, context, deps.app_handle.clone())
        .await;
    if result.is_error {
        return Vec::new();
    }

    result
        .output
        .get("matches")
        .and_then(|value| value.as_array())
        .map(|matches| {
            matches
                .iter()
                .take(8)
                .filter_map(|m| {
                    let path = m.get("path").and_then(|value| value.as_str())?;
                    let line = m.get("line").and_then(serde_json::Value::as_u64).unwrap_or(0);
                    let text = m.get("text").and_then(|value| value.as_str()).unwrap_or("");
                    Some(format!("{path}:{line} {text}", text = text.trim()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn build_retrieval_query(request_content: &str, active_subtask_title: Option<&str>) -> String {
    let mut raw = String::new();
    raw.push_str(request_content);
    raw.push(' ');
    if let Some(title) = active_subtask_title {
        raw.push_str(title);
    }

    let stop_words = [
        "the", "and", "for", "with", "this", "that", "from", "into", "about", "please", "would",
        "should", "could", "must", "task",
    ];
    let mut keywords = Vec::new();
    for token in raw.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
        let token = token.trim().to_lowercase();
        if token.len() < 4
            || stop_words.contains(&token.as_str())
            || keywords.iter().any(|existing| existing == &token)
        {
            continue;
        }
        keywords.push(token);
        if keywords.len() >= 8 {
            break;
        }
    }

    keywords.join(" ")
}
