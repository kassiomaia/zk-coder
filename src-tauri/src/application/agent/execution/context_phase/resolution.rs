//! Context-assembly helpers for the context phase.
//!
//! This submodule owns the pieces of context resolution that touch
//! side effects or compute retrieval payloads — specifically:
//!
//! - Memory-refresh triggers (M3.3): `SubtaskCompleted`,
//!   `RetrievalBurst`, `CompactionBoundary`.
//! - Retrieval pass: builds a narrow query from the task goal + active
//!   subtask title and executes `search_grep` through the tool
//!   registry.
//! - Drop-message identification for the optional pre-compaction
//!   summary path.
//! - Compaction recording: traces every evaluation and emits the
//!   user-visible `ContextCompacted` event when the reduction was
//!   genuine and driven by token pressure.

use serde_json::json;
use tauri::Emitter;

use super::super::deps::ExecutionDeps;
use super::super::memory_refresh::{
    apply_memory_refresh, MemoryMutation, MemoryRefreshTrigger,
};
use super::super::state::ExecutionContext;
use crate::core::agent::execution::RuntimeTraceType;
use crate::core::agent::session::AgentSession;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::events::{AgentEvent, ContextCompactionReason, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::context::{
    CompactionDetails, ContextPackOutcome,
};
use crate::infrastructure::tools::context::ToolExecutionContext;

/// Fires the `SubtaskCompleted` memory-refresh trigger (M3.3) when the
/// previous subtask id differs from the current one **and** the previous
/// subtask is now `Completed` on disk. The trigger fires at most once
/// per completed subtask id — subsequent steps that happen to observe
/// the same transition skip it via
/// `ctx.last_completed_subtask_for_memory`.
pub(super) fn maybe_fire_subtask_completed(
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
pub(super) fn maybe_fire_retrieval_burst(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    current_subtask_id: Option<&str>,
    retrieval_snippets: &[String],
) {
    let count = ctx.retrieval_count_in_current_subtask;
    let paths = super::super::memory_refresh::snippet_paths(retrieval_snippets);
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
pub(super) fn maybe_fire_compaction_boundary(
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

pub(super) async fn run_retrieval_pass(
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
pub(super) fn identify_messages_to_drop(
    context: &[crate::core::llm::entities::ChatMessage],
) -> Vec<crate::core::llm::entities::ChatMessage> {
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
fn should_emit_compaction_event(
    details: &CompactionDetails,
    triggered_by_token_limit: bool,
) -> bool {
    matches!(details.outcome, ContextPackOutcome::Compacted)
        && triggered_by_token_limit
        && details.after_tokens < details.before_tokens
}

/// Records a compaction pass: always appends a trace; optionally emits
/// the user-visible `ContextCompacted` event when [`should_emit_compaction_event`] agrees.
pub(super) fn record_compaction(
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

    fn sample_details(
        outcome: ContextPackOutcome,
        before: usize,
        after: usize,
    ) -> CompactionDetails {
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
