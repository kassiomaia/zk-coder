//! Handles the context resolution phase of the agent's execution.
//!
//! This module is responsible for preparing the context for the LLM call,
//! which includes summarizing the current subtask, managing the context window,
//! performing retrieval passes, and deriving the system prompt.
//!
//! Two concerns that used to crowd this module have been extracted:
//!
//! - [`resolution`] — memory-refresh triggers, retrieval pass, drop
//!   identification, and compaction trace + event recording.
//! - [`projection`] — builders that turn durable state into the
//!   value objects consumed by the context policy
//!   (`ExecutionBrief`, `MemoryReference`).
//!
//! The helpers those submodules expose are re-used here only; the
//! public entry point for the phase remains [`resolve_context`].

use log::info;
use serde_json::json;

use super::deps::ExecutionDeps;
use super::memory_refresh::RETRIEVAL_BURST_THRESHOLD;
use super::state::{ExecutionContext, ResolvedContext};
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::AgentRequest;
use crate::core::llm::entities::ToolDefinition;
use crate::infrastructure::agent::registry::SystemPromptBuilder;
use crate::infrastructure::agent::runtime::context::{
    adaptive_compression_threshold, apply_context_policy_v2, estimate_context_tokens,
    maybe_compact_context, model_context_window, ContextPackOutcome, ContextPolicyInput,
};
use crate::infrastructure::agent::runtime::{
    derive_system_prompt, AgentRuntimeError, SubtaskSummary,
};
use crate::infrastructure::llm::LlmClient;

mod projection;
mod resolution;

use projection::{build_execution_brief, lookup_memory_reference};
use resolution::{
    identify_messages_to_drop, maybe_fire_compaction_boundary, maybe_fire_retrieval_burst,
    maybe_fire_subtask_completed, record_compaction, run_retrieval_pass,
};

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
/// * `client` - The LLM client for optional context summarization.
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
    client: &dyn LlmClient,
) -> Result<ResolvedContext, AgentRuntimeError> {
    ctx.current_phase = ExecutionPhase::ResolvingContext;
    let tracer = deps.tracer();

    // Snapshot the prior subtask id before it is overwritten — the
    // SubtaskCompleted memory-refresh trigger (M3.3) needs it to detect
    // a transition that also marked the previous subtask as Completed.
    let previous_subtask_id = ctx.current_subtask_id.clone();

    // The session-visible tool catalog is always the full canonical list from
    // `deps.all_tool_definitions`. Per-subtask permission filtering used to
    // happen here — it now lives in the runtime (tool_phase) via
    // `ctx.current_allowed_tool_names`, so the visible catalog and its hash
    // stay stable across subtask transitions (M2.2).
    let loaded_subtasks: Vec<crate::core::agent::task::SubTask> =
        deps.task_repo.get_subtasks(&ctx.task_id).unwrap_or_default();
    let (current_subtask_summary, current_subtask_id, allowed_tool_names, all_subtasks_completed) =
        if loaded_subtasks.is_empty() {
            (None, None, None, false)
        } else {
            let total = loaded_subtasks.len();
            if let Some(current) = loaded_subtasks
                .iter()
                .find(|st| st.status == crate::core::agent::task::TaskStatus::InProgress)
            {
                let summary = SubtaskSummary::from_subtask(current);
                // `allowed_tools()` returns `None` for FullAccess — that means
                // "no allowlist enforcement, every registered tool passes". We
                // forward the Option through to `ctx.current_allowed_tool_names`
                // so the permission filter in `tool_phase::run_tools` sees
                // `None` and skips the rejection branch entirely. Restricted
                // modes produce `Some(Vec<String>)` as before.
                let allowed = summary.mode.allowed_tools().map(|tools| {
                    tools
                        .into_iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>()
                });
                (
                    Some((summary, total)),
                    Some(current.id.clone()),
                    allowed,
                    false,
                )
            } else {
                let all_done = loaded_subtasks.iter().all(|st| {
                    matches!(
                        st.status,
                        crate::core::agent::task::TaskStatus::Completed
                            | crate::core::agent::task::TaskStatus::Failed
                            | crate::core::agent::task::TaskStatus::Cancelled
                    )
                });
                (None, None, None, all_done)
            }
        };
    ctx.current_allowed_tool_names = allowed_tool_names.clone();

    let current_order = current_subtask_summary
        .as_ref()
        .map(|(summary, _)| summary.order);
    let subtask_changed = ctx.last_injected_subtask_order != current_order;
    ctx.current_subtask_id = current_subtask_id.clone();

    // M3.3 — SubtaskCompleted refresh trigger. Fires at most once per
    // completed subtask (guarded by `last_completed_subtask_for_memory`).
    maybe_fire_subtask_completed(
        deps,
        ctx,
        previous_subtask_id.as_deref(),
        current_subtask_id.as_deref(),
        &loaded_subtasks,
        current_subtask_summary
            .as_ref()
            .map(|(summary, _)| summary.title.as_str()),
    );

    // Reset the per-subtask retrieval counter whenever the active subtask
    // changes so a RetrievalBurst trigger only observes retrievals that
    // happened inside the current subtask.
    if subtask_changed {
        ctx.retrieval_count_in_current_subtask = 0;
    }

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
            "allowedToolNames": allowed_tool_names,
        }),
    );

    let effective_window = model_context_window(&ctx.model_name);
    let threshold = adaptive_compression_threshold(&ctx.model_name);
    let usage_ratio = estimate_context_tokens(&ctx.llm_context) as f64 / effective_window as f64;
    let should_retrieve = usage_ratio >= threshold || subtask_changed;
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

        // M3.3 — RetrievalBurst refresh trigger. Only retrievals inside
        // the current subtask count (counter is reset on subtask change).
        ctx.retrieval_count_in_current_subtask =
            ctx.retrieval_count_in_current_subtask.saturating_add(1);
        if ctx.retrieval_count_in_current_subtask >= RETRIEVAL_BURST_THRESHOLD {
            maybe_fire_retrieval_burst(
                deps,
                ctx,
                current_subtask_id.as_deref(),
                &retrieval_snippets,
            );
            ctx.retrieval_count_in_current_subtask = 0;
        }
    }

    // Identify messages that would be dropped and optionally summarize them
    let dropped_messages = identify_messages_to_drop(&ctx.llm_context);
    let pre_computed_summary = if dropped_messages.len() >= 8 && usage_ratio >= threshold {
        crate::infrastructure::agent::runtime::summarize_dropped_messages(
            client,
            &ctx.model_name,
            &dropped_messages,
        )
        .await
        .and_then(|msg| msg.as_text().map(|s| s.to_string()))
    } else {
        None
    };

    let memory_reference = lookup_memory_reference(deps, &ctx.session_id);

    let execution_brief = build_execution_brief(
        &request.content,
        current_subtask_summary
            .as_ref()
            .map(|(summary, _)| summary.title.as_str()),
        &loaded_subtasks,
        &ctx.memorized_failures,
        current_subtask_id.as_deref(),
    );

    let policy_input = ContextPolicyInput {
        force_pack: subtask_changed,
        task_goal: Some(request.content.clone()),
        active_subtask: current_subtask_summary
            .as_ref()
            .map(|(summary, _)| summary.title.clone()),
        retrieval_snippets,
        model_name: ctx.model_name.clone(),
        pre_computed_summary,
        memory_reference,
        execution_brief: Some(execution_brief),
        memorized_failures: ctx.memorized_failures.clone(),
        current_subtask_id: current_subtask_id.clone(),
    };

    let mut compaction_fired = false;
    if let Some(compaction) = apply_context_policy_v2(&mut ctx.llm_context, &policy_input) {
        if matches!(compaction.outcome, ContextPackOutcome::Compacted) {
            compaction_fired = true;
        }
        record_compaction(
            deps,
            ctx,
            session,
            current_subtask_id.as_deref(),
            "v2",
            usage_ratio >= threshold,
            &compaction,
        );
    } else if usage_ratio >= threshold {
        if let Some(compaction) = maybe_compact_context(&mut ctx.llm_context, &ctx.model_name) {
            if matches!(compaction.outcome, ContextPackOutcome::Compacted) {
                compaction_fired = true;
            }
            record_compaction(
                deps,
                ctx,
                session,
                current_subtask_id.as_deref(),
                "legacy",
                true,
                &compaction,
            );
        }
    }

    // M3.3 — CompactionBoundary refresh trigger. Memory is the primary
    // state-carrier across compaction, so record the boundary even when
    // no content mutation is needed; `active_plan` is refreshed to the
    // currently in-progress subtask title if it drifted.
    if compaction_fired {
        maybe_fire_compaction_boundary(
            deps,
            ctx,
            current_subtask_id.as_deref(),
            current_subtask_summary
                .as_ref()
                .map(|(summary, _)| summary.title.as_str()),
        );
    }

    ctx.last_injected_subtask_order = current_order;

    // Workstream H.2.c — compose the static agent prompt from the
    // task's declared framing when present. `SystemPromptBuilder::
    // compose_for_task(None)` is byte-for-byte identical to the legacy
    // `compose_default_code_agent()` used at agent construction, so
    // tasks without a declared framing render the exact prompt they
    // have today. Tasks with a declared framing substitute their
    // rendered framing for the default code framing section.
    let task_framing = deps
        .task_repo
        .get_session_tasks(&ctx.session_id)
        .ok()
        .and_then(|tasks| tasks.into_iter().find(|t| t.id == ctx.task_id))
        .and_then(|t| t.framing);
    let static_prompt = if task_framing.is_some() {
        SystemPromptBuilder::compose_for_task(task_framing.as_ref())
    } else {
        // Legacy path: keep the agent-level frozen prompt. This avoids
        // burning the cache for in-flight sessions whose tasks were
        // planned before H.2.b shipped.
        agent_system_prompt.to_string()
    };
    let active_framing_section =
        SystemPromptBuilder::active_framing_section(task_framing.as_ref());
    let system_prompt = derive_system_prompt(&static_prompt, &current_subtask_summary);
    info!(
        "[AgentRuntime] System prompt length: {}{}",
        system_prompt.len(),
        if current_subtask_summary.is_some() {
            " (with subtask context)"
        } else {
            ""
        }
    );

    // Session-stable tool catalog (M2.2): the model always sees every tool the
    // registry exposes. Runtime enforcement in `tool_phase` rejects calls that
    // fall outside the current subtask mode's permitted set.
    let tool_definitions: Vec<ToolDefinition> = deps.all_tool_definitions.to_vec();
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ToolsetFiltered,
        json!({
            "toolCount": tool_definitions.len(),
            "tools": tool_definitions.iter().map(|tool| tool.name.clone()).collect::<Vec<_>>(),
            "allowedToolNames": allowed_tool_names,
        }),
    );

    Ok(ResolvedContext {
        current_subtask_summary,
        current_subtask_id,
        all_subtasks_completed,
        tool_definitions,
        system_prompt,
        active_framing_section,
    })
}
