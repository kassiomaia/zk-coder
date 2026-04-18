//! Handles the context resolution phase of the agent's execution.
//!
//! This module is responsible for preparing the context for the LLM call,
//! which includes summarizing the current subtask, managing the context window,
//! performing retrieval passes, and deriving the system prompt.

use log::info;
use serde_json::json;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::memory_refresh::{
    apply_memory_refresh, MemoryMutation, MemoryRefreshTrigger, RETRIEVAL_BURST_THRESHOLD,
};
use super::state::{ExecutionContext, ResolvedContext};
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::AgentRequest;
use crate::core::llm::entities::{ToolCall, ToolDefinition};
use crate::infrastructure::agent::events::{AgentEvent, ContextCompactionReason, AGENT_EVENT_NAME};
use crate::core::agent::session::SessionId;
use crate::infrastructure::agent::runtime::context::{
    adaptive_compression_threshold, apply_context_policy_v2, estimate_context_tokens,
    extract_failure_record, maybe_compact_context, model_context_window, CompactionDetails,
    ContextPackOutcome, ContextPolicyInput, ExecutionBrief, MemoryReference,
};
use crate::infrastructure::llm::LlmClient;
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
    let (
        current_subtask_summary,
        current_subtask_id,
        allowed_tool_names,
        all_subtasks_completed,
    ) = if loaded_subtasks.is_empty() {
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
        &ctx.llm_context,
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
    })
}

/// Fires the `SubtaskCompleted` memory-refresh trigger (M3.3) when the
/// previous subtask id differs from the current one **and** the previous
/// subtask is now `Completed` on disk. The trigger fires at most once
/// per completed subtask id — subsequent steps that happen to observe
/// the same transition skip it via
/// `ctx.last_completed_subtask_for_memory`.
fn maybe_fire_subtask_completed(
    deps: &ExecutionDeps<'_>,
    ctx: &mut ExecutionContext,
    previous_subtask_id: Option<&str>,
    current_subtask_id: Option<&str>,
    loaded_subtasks: &[crate::core::agent::task::SubTask],
    current_subtask_title: Option<&str>,
) {
    let Some(prev_id) = previous_subtask_id else {
        return;
    };
    if current_subtask_id == Some(prev_id) {
        return;
    }
    if ctx.last_completed_subtask_for_memory.as_deref() == Some(prev_id) {
        return;
    }
    let Some(prev_subtask) = loaded_subtasks.iter().find(|st| st.id == prev_id) else {
        return;
    };
    if prev_subtask.status != crate::core::agent::task::TaskStatus::Completed {
        return;
    }

    let trigger = MemoryRefreshTrigger::SubtaskCompleted {
        subtask_id: prev_subtask.id.clone(),
        title: prev_subtask.title.clone(),
        order: prev_subtask.order,
    };
    let mutation = MemoryMutation {
        append_findings: vec![prev_subtask.title.clone()],
        append_facts: Vec::new(),
        new_active_plan: current_subtask_title.map(str::to_string),
    };

    apply_memory_refresh(
        deps.memory_repo,
        deps.trace_repo,
        &ctx.session_id,
        &ctx.task_id,
        current_subtask_id,
        ctx.step_number,
        &trigger,
        mutation,
    );
    ctx.last_completed_subtask_for_memory = Some(prev_subtask.id.clone());
}

/// Fires the `RetrievalBurst` memory-refresh trigger (M3.3) once the
/// per-subtask retrieval counter crosses
/// `RETRIEVAL_BURST_THRESHOLD`. The mutation appends deduplicated file
/// paths from the most recent retrieval to `extracted_facts`.
fn maybe_fire_retrieval_burst(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    current_subtask_id: Option<&str>,
    retrieval_snippets: &[String],
) {
    let count = ctx.retrieval_count_in_current_subtask;
    let paths = super::memory_refresh::snippet_paths(retrieval_snippets);
    let trigger = MemoryRefreshTrigger::RetrievalBurst { count };
    let mutation = MemoryMutation {
        append_findings: Vec::new(),
        append_facts: paths,
        new_active_plan: None,
    };
    apply_memory_refresh(
        deps.memory_repo,
        deps.trace_repo,
        &ctx.session_id,
        &ctx.task_id,
        current_subtask_id,
        ctx.step_number,
        &trigger,
        mutation,
    );
}

/// Fires the `CompactionBoundary` memory-refresh trigger (M3.3) after a
/// compaction pass actually reduced the prefix. The mutation only moves
/// `active_plan` to the currently in-progress subtask title; no
/// findings/facts are added because the compaction event itself is the
/// boundary we want to audit.
fn maybe_fire_compaction_boundary(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    current_subtask_id: Option<&str>,
    current_subtask_title: Option<&str>,
) {
    let trigger = MemoryRefreshTrigger::CompactionBoundary;
    let mutation = MemoryMutation {
        append_findings: Vec::new(),
        append_facts: Vec::new(),
        new_active_plan: current_subtask_title.map(str::to_string),
    };
    apply_memory_refresh(
        deps.memory_repo,
        deps.trace_repo,
        &ctx.session_id,
        &ctx.task_id,
        current_subtask_id,
        ctx.step_number,
        &trigger,
        mutation,
    );
}

/// Synthesizes a deterministic `ExecutionBrief` (M4.1) from the current
/// runtime state. The brief is intentionally derived from durable /
/// inspectable state (the user's request, the subtask list, the
/// in-memory LLM context) rather than a separate stateful cache — so
/// two calls on the same state produce byte-identical briefs and
/// `message_prefix_hash` stays stable until the underlying state
/// actually moves.
fn build_execution_brief(
    request_content: &str,
    active_subtask_title: Option<&str>,
    loaded_subtasks: &[crate::core::agent::task::SubTask],
    llm_context: &[crate::core::llm::entities::ChatMessage],
) -> ExecutionBrief {
    const RECENTLY_COMPLETED_LIMIT: usize = 3;

    let mut completed: Vec<(i32, String)> = loaded_subtasks
        .iter()
        .filter(|st| st.status == crate::core::agent::task::TaskStatus::Completed)
        .map(|st| (st.order, st.title.clone()))
        .collect();
    completed.sort_by_key(|(order, _)| *order);
    let skip = completed.len().saturating_sub(RECENTLY_COMPLETED_LIMIT);
    let recently_completed: Vec<String> = completed
        .into_iter()
        .skip(skip)
        .map(|(_, title)| title)
        .collect();

    let next_intended_action = loaded_subtasks
        .iter()
        .filter(|st| st.status == crate::core::agent::task::TaskStatus::Pending)
        .min_by_key(|st| st.order)
        .map(|st| st.title.clone());

    let current_blocker = extract_failure_record(llm_context).map(|record| {
        format!(
            "{class}: {tool}",
            class = blocker_class_label(&record),
            tool = record.tool_name
        )
    });

    ExecutionBrief {
        task_goal: request_content.to_string(),
        active_subtask: active_subtask_title.map(str::to_string),
        recently_completed,
        next_intended_action,
        current_blocker,
    }
}

/// Short label used by the execution brief's `current_blocker` line.
/// Kept narrow on purpose — the M4.2 failure-memory block carries the
/// full structured breakdown, so the brief only needs a pointer.
fn blocker_class_label(
    record: &crate::infrastructure::agent::runtime::context::FailureRecord,
) -> &'static str {
    use crate::infrastructure::agent::runtime::context::FailureClass;
    match record.class {
        FailureClass::FileNotFound => "file_not_found",
        FailureClass::PermissionDenied => "permission_denied",
        FailureClass::Timeout => "timeout",
        FailureClass::ValidationFailed => "validation_failed",
        FailureClass::NotAllowed => "not_allowed",
        FailureClass::Other => "error",
    }
}

/// Loads the durable session memory (if any) and condenses it into a
/// `MemoryReference` suitable for inlining in the packed context. The
/// reference carries only counts + short truncated strings so the block
/// stays within its token budget even when the memory body grows
/// unbounded (M3.2).
fn lookup_memory_reference(
    deps: &ExecutionDeps<'_>,
    session_id: &str,
) -> Option<MemoryReference> {
    let session_id = SessionId::from(session_id);
    match deps.memory_repo.get_memory(&session_id) {
        Ok(Some(memory)) => Some(MemoryReference {
            session_id: memory.session_id.0.clone(),
            task_goal: memory.task_goal.clone(),
            active_plan: memory.active_plan.clone(),
            completed_findings_count: memory.completed_findings.len(),
            open_questions_count: memory.open_questions.len(),
            extracted_facts_count: memory.extracted_facts.len(),
            updated_at_rfc3339: memory.updated_at.to_rfc3339(),
        }),
        Ok(None) => None,
        Err(err) => {
            log::warn!(
                "[AgentRuntime] Failed to load session memory for {session_id}: {err}",
                session_id = session_id
            );
            None
        }
    }
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
                    let line = m
                        .get("line")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
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

/// Identifies messages that would be dropped by ContextPolicyV2.
///
/// Returns messages beyond the recent tail (excluding the system prompt at index 0).
fn identify_messages_to_drop(context: &[crate::core::llm::entities::ChatMessage]) -> Vec<crate::core::llm::entities::ChatMessage> {
    const KEEP_RECENT_TAIL_MESSAGES: usize = 12;
    let tail_start = context.len().saturating_sub(KEEP_RECENT_TAIL_MESSAGES + 4);
    if tail_start > 1 {
        context[1..tail_start].to_vec()
    } else {
        Vec::new()
    }
}

/// Whether a compaction pass should surface as a user-visible chat notice.
///
/// Only genuine reductions triggered by token pressure are shown. Forced
/// subtask-transition repacks and no-op reorganizations are kept as traces.
fn should_emit_compaction_event(details: &CompactionDetails, triggered_by_token_limit: bool) -> bool {
    matches!(details.outcome, ContextPackOutcome::Compacted)
        && triggered_by_token_limit
        && details.after_tokens < details.before_tokens
}

/// Records a compaction pass: always appends a trace; optionally emits
/// the user-visible `ContextCompacted` event when [`should_emit_compaction_event`] agrees.
fn record_compaction(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    session: &AgentSession,
    current_subtask_id: Option<&str>,
    strategy: &'static str,
    triggered_by_token_limit: bool,
    details: &CompactionDetails,
) {
    let outcome_label = match details.outcome {
        ContextPackOutcome::Compacted => "compacted",
        ContextPackOutcome::Reorganized => "reorganized",
        ContextPackOutcome::Unchanged => "unchanged",
    };
    let reason_label = if triggered_by_token_limit {
        "token_limit"
    } else {
        "subtask_transition"
    };

    let _ = deps.tracer().append_trace(
        &ctx.session_id,
        &ctx.task_id,
        current_subtask_id,
        ctx.step_number,
        RuntimeTraceType::ContextPolicyEvaluated,
        json!({
            "strategy": strategy,
            "outcome": outcome_label,
            "reason": reason_label,
            "beforeCount": details.before_count,
            "afterCount": details.after_count,
            "beforeTokens": details.before_tokens,
            "afterTokens": details.after_tokens,
        }),
    );

    if should_emit_compaction_event(details, triggered_by_token_limit) {
        let _ = deps.app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ContextCompacted {
                session_id: ctx.session_id.clone(),
                conversation_id: session.conversation_id.clone(),
                reason: ContextCompactionReason::TokenLimit,
                before_count: details.before_count,
                after_count: details.after_count,
                before_tokens: details.before_tokens,
                after_tokens: details.after_tokens,
                timestamp: chrono::Utc::now(),
            },
        );
    } else {
        log::debug!(
            "[AgentRuntime] context pack outcome={outcome_label} reason={reason_label} strategy={strategy} \
             before_tokens={} after_tokens={}",
            details.before_tokens,
            details.after_tokens,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_details(outcome: ContextPackOutcome, before: usize, after: usize) -> CompactionDetails {
        CompactionDetails {
            outcome,
            before_count: 20,
            after_count: 15,
            before_tokens: before,
            after_tokens: after,
            needs_reinjection: true,
        }
    }

    #[test]
    fn emits_only_when_compacted_and_token_limit_driven() {
        let details = sample_details(ContextPackOutcome::Compacted, 10_000, 4_000);
        assert!(should_emit_compaction_event(&details, true));
    }

    #[test]
    fn suppresses_when_outcome_is_reorganized() {
        let details = sample_details(ContextPackOutcome::Reorganized, 1_000, 1_000);
        assert!(!should_emit_compaction_event(&details, true));
    }

    #[test]
    fn suppresses_when_outcome_is_unchanged() {
        let details = sample_details(ContextPackOutcome::Unchanged, 800, 800);
        assert!(!should_emit_compaction_event(&details, true));
    }

    #[test]
    fn suppresses_subtask_transition_even_when_compacted() {
        let details = sample_details(ContextPackOutcome::Compacted, 10_000, 4_000);
        assert!(!should_emit_compaction_event(&details, false));
    }

    #[test]
    fn suppresses_when_after_tokens_not_lower() {
        let details = sample_details(ContextPackOutcome::Compacted, 4_000, 4_000);
        assert!(!should_emit_compaction_event(&details, true));
    }
}
