//! Context compaction policies (V1 and V2) and their orchestration.
//!
//! Owns the runtime-facing entry points (`maybe_compact_context`,
//! `apply_context_policy_v2`), the deterministic block extractors that
//! feed them, and the helper that selects between the typed
//! `MemorizedFailure` overlay and the legacy context-scan fallback.

use serde_json::Value as JsonValue;

use crate::core::llm::entities::{ChatMessage as Message, MessageRole};

use super::budget::{
    adaptive_compression_threshold, estimate_context_tokens, estimate_message_tokens,
    estimate_text_tokens, model_context_window, BUDGET_ACTIVE_SUBTASK_TOKENS,
    BUDGET_FAILURE_MEMORY_TOKENS, BUDGET_FILE_RELEVANCE_TOKENS, BUDGET_RECENT_DIALOGUE_TOKENS,
    BUDGET_TASK_SUMMARY_TOKENS, KEEP_RECENT_MESSAGES, KEEP_RECENT_TAIL_MESSAGES,
};
use super::failure::{select_failure_memory_body, MemorizedFailure};
use super::render::{
    format_execution_brief_block, format_memory_reference_block, ExecutionBrief, MemoryReference,
};
use super::{truncate_text, truncate_to_token_budget};

/// Minimum absolute token reduction to classify a pack pass as a real compaction.
pub const MIN_SAVINGS_TOKENS: usize = 200;
/// Minimum relative token reduction (fraction of before) to classify as real compaction.
pub const MIN_SAVINGS_RATIO: f64 = 0.05;
/// Forced packing is skipped when context usage is below this ratio of the window.
pub const TINY_CONTEXT_RATIO: f64 = 0.10;

/// Categorization of the outcome of a context packing pass.
///
/// This distinguishes meaningful compaction from benign reorganization,
/// so callers can decide whether to surface the change to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextPackOutcome {
    /// Packing produced a measurable token reduction.
    Compacted,
    /// Packing changed the context structure but did not reduce tokens meaningfully.
    Reorganized,
    /// Packing produced no change in counts or tokens.
    Unchanged,
}

/// Details about context compaction.
#[derive(Debug, Clone)]
pub struct CompactionDetails {
    /// Qualitative outcome of the packing pass.
    pub outcome: ContextPackOutcome,
    /// Number of messages before compaction.
    pub before_count: usize,
    /// Number of messages after compaction.
    pub after_count: usize,
    /// Estimated tokens before compaction.
    pub before_tokens: usize,
    /// Estimated tokens after compaction.
    pub after_tokens: usize,
    /// Whether reinjection of context is needed.
    pub needs_reinjection: bool,
}

/// Inputs for ContextPolicyV2 deterministic packing.
#[derive(Debug, Clone, Default)]
pub struct ContextPolicyInput {
    /// Force packing even if below threshold.
    pub force_pack: bool,
    /// The overall task goal.
    pub task_goal: Option<String>,
    /// The currently active subtask.
    pub active_subtask: Option<String>,
    /// Retrieved context snippets.
    pub retrieval_snippets: Vec<String>,
    /// The model name for adaptive threshold and context window size.
    pub model_name: String,
    /// Pre-computed summary of dropped messages (if available).
    pub pre_computed_summary: Option<String>,
    /// Compact reference to the session's durable working memory
    /// artifact. When `Some`, the packer inlines a single bounded block
    /// that identifies the memory without copying its full contents;
    /// compaction rebuilds the block from this reference on every pass.
    pub memory_reference: Option<MemoryReference>,
    /// "What matters now" brief inlined near the attention tail
    /// (Milestone 4.1). When `Some`, the packer emits a single
    /// deterministic block derived from this snapshot.
    pub execution_brief: Option<ExecutionBrief>,
    /// Subtask-scoped failure memory overlay (Workstream C.1/C.2).
    /// When non-empty, the packer selects the most recent entry whose
    /// `subtask_id == current_subtask_id` or is unscoped, and renders
    /// its `FailureRecord` via `render_failure_memory`. When empty,
    /// the packer falls back to the legacy context-scan path.
    pub memorized_failures: Vec<MemorizedFailure>,
    /// The currently active subtask id, used to filter
    /// `memorized_failures`. `None` when no subtask is in progress.
    pub current_subtask_id: Option<String>,
}

/// Classifies a packing pass by comparing before/after counts and tokens.
pub(super) fn classify_outcome(
    before_tokens: usize,
    after_tokens: usize,
    before_count: usize,
    after_count: usize,
) -> ContextPackOutcome {
    if before_tokens == after_tokens && before_count == after_count {
        return ContextPackOutcome::Unchanged;
    }
    let saved = before_tokens.saturating_sub(after_tokens);
    let ratio = if before_tokens == 0 {
        0.0
    } else {
        saved as f64 / before_tokens as f64
    };
    if saved >= MIN_SAVINGS_TOKENS && ratio >= MIN_SAVINGS_RATIO {
        ContextPackOutcome::Compacted
    } else {
        ContextPackOutcome::Reorganized
    }
}

/// Compacts the message context if it exceeds the compression threshold.
///
/// Returns `Some` with compaction details if compaction was performed,
/// `None` if the context was below the threshold.
///
/// # Algorithm
///
/// 1. Preserve the first message (usually system prompt)
/// 2. Keep the most recent messages
/// 3. Remove old messages in between
pub fn maybe_compact_context(context: &mut Vec<Message>, model_name: &str) -> Option<CompactionDetails> {
    let before_count = context.len();
    let before_tokens = estimate_context_tokens(context);

    // Calculate if we need compaction
    let effective_window = model_context_window(model_name);
    let threshold = adaptive_compression_threshold(model_name);
    let usage_ratio = before_tokens as f64 / effective_window as f64;
    if usage_ratio < threshold {
        return None;
    }

    // Compact: keep first message + recent messages
    if before_count <= KEEP_RECENT_MESSAGES + 1 {
        return None;
    }

    let mut new_context = Vec::new();

    // Always keep the first message (usually system prompt)
    if let Some(first) = context.first() {
        new_context.push(first.clone());
    }

    if let Some(summary) = build_compaction_summary(context) {
        new_context.push(summary);
    }

    // Keep the most recent messages
    let recent_start = before_count.saturating_sub(KEEP_RECENT_MESSAGES);
    for msg in context.iter().skip(recent_start) {
        new_context.push(msg.clone());
    }

    let after_count = new_context.len();
    let after_tokens = estimate_context_tokens(&new_context);

    *context = new_context;

    Some(CompactionDetails {
        outcome: classify_outcome(before_tokens, after_tokens, before_count, after_count),
        before_count,
        after_count,
        before_tokens,
        after_tokens,
        needs_reinjection: true, // Always signal for re-injection after compaction
    })
}

/// Applies ContextPolicyV2 deterministic packing when context is large or explicitly forced.
///
/// Packing order:
/// 1) first system prompt
/// 2) task summary block
/// 3) active subtask block
/// 4) file relevance block
/// 5) failure memory block
/// 6) pre-computed summary block (if available)
/// 7) recent dialogue tail
pub fn apply_context_policy_v2(
    context: &mut Vec<Message>,
    input: &ContextPolicyInput,
) -> Option<CompactionDetails> {
    let before_count = context.len();
    let before_tokens = estimate_context_tokens(context);
    let effective_window = model_context_window(&input.model_name);
    let threshold = adaptive_compression_threshold(&input.model_name);
    let usage_ratio = before_tokens as f64 / effective_window as f64;
    if usage_ratio < threshold && !input.force_pack {
        return None;
    }

    // Skip packing when the context is tiny and packing was merely forced by
    // a subtask transition. Repacking a near-empty context cannot yield a
    // meaningful reduction and the synthetic blocks can even grow it.
    //
    // Exception (M3.2): when a memory reference exists, the packer must
    // still run so the reference block is inlined — otherwise compaction
    // could silently drop the only pointer to durable memory.
    //
    // Exception (M4.1): the same applies to the execution brief — it is
    // the mechanism that reintroduces the objective near the tail; a
    // tiny-context skip would suppress it on subtask transitions.
    if input.force_pack
        && usage_ratio < TINY_CONTEXT_RATIO
        && input.retrieval_snippets.is_empty()
        && input.pre_computed_summary.is_none()
        && input.memory_reference.is_none()
        && input.execution_brief.is_none()
    {
        return None;
    }

    let mut packed: Vec<Message> = Vec::new();
    if let Some(first) = context.first() {
        packed.push(first.clone());
    }

    let derived_task_goal = input
        .task_goal
        .clone()
        .or_else(|| extract_task_goal(context));
    if let Some(task_goal) = derived_task_goal {
        let text = truncate_to_token_budget(&task_goal, BUDGET_TASK_SUMMARY_TOKENS);
        if !text.is_empty() {
            packed.push(Message::system(format!(
                "ContextPolicyV2 task summary:\n{}",
                text
            )));
        }
    }

    let derived_active_subtask = input.active_subtask.clone().or_else(|| {
        let (subs, active) = extract_plan_subtasks(context);
        match (subs, active) {
            (Some(list), Some(i)) => list.get(i).cloned(),
            _ => None,
        }
    });
    if let Some(active_subtask) = derived_active_subtask {
        let text = truncate_to_token_budget(&active_subtask, BUDGET_ACTIVE_SUBTASK_TOKENS);
        if !text.is_empty() {
            packed.push(Message::system(format!(
                "ContextPolicyV2 active subtask:\n{}",
                text
            )));
        }
    }

    let ranked_snippets = rank_retrieval_snippets(&input.retrieval_snippets);
    if !ranked_snippets.is_empty() {
        let mut lines: Vec<String> = Vec::new();
        let mut used_tokens = 0usize;
        for snippet in ranked_snippets {
            let tokens = estimate_text_tokens(&snippet);
            if used_tokens + tokens > BUDGET_FILE_RELEVANCE_TOKENS {
                break;
            }
            used_tokens += tokens;
            lines.push(snippet);
        }
        if !lines.is_empty() {
            packed.push(Message::system(format!(
                "ContextPolicyV2 file relevance:\n{}",
                lines.join("\n")
            )));
        }
    }

    let failure_memory_body = select_failure_memory_body(
        &input.memorized_failures,
        input.current_subtask_id.as_deref(),
        context,
    );
    if let Some(failure_memory) = failure_memory_body {
        let text = truncate_to_token_budget(&failure_memory, BUDGET_FAILURE_MEMORY_TOKENS);
        if !text.is_empty() {
            packed.push(Message::system(format!(
                "ContextPolicyV2 failure memory:\n{}",
                text
            )));
        }
    }

    if let Some(ref summary) = input.pre_computed_summary {
        let text = truncate_to_token_budget(summary, 1_200);
        if !text.is_empty() {
            packed.push(Message::system(format!(
                "ContextPolicyV2 LLM summary:\n{}",
                text
            )));
        }
    }

    if let Some(ref memory) = input.memory_reference {
        let block = format_memory_reference_block(memory);
        if !block.is_empty() {
            packed.push(Message::system(block));
        }
    }

    // M4.1 execution brief sits last among the synthetic blocks so the
    // "what matters now" summary is as close to the attention tail as
    // possible without being interleaved with live dialogue.
    if let Some(ref brief) = input.execution_brief {
        let block = format_execution_brief_block(brief);
        if !block.is_empty() {
            packed.push(Message::system(block));
        }
    }

    let tail = collect_recent_dialogue_tail(context, BUDGET_RECENT_DIALOGUE_TOKENS);
    packed.extend(tail);

    let after_count = packed.len();
    let after_tokens = estimate_context_tokens(&packed);
    *context = packed;

    Some(CompactionDetails {
        outcome: classify_outcome(before_tokens, after_tokens, before_count, after_count),
        before_count,
        after_count,
        before_tokens,
        after_tokens,
        needs_reinjection: true,
    })
}

pub(super) fn build_compaction_summary(context: &[Message]) -> Option<Message> {
    let task_goal = extract_task_goal(context);
    let (subtasks, active_index) = extract_plan_subtasks(context);
    let file_actions = extract_recent_file_actions(context, 3);
    let last_error = extract_last_tool_error(context);

    if task_goal.is_none() && subtasks.is_none() && file_actions.is_empty() && last_error.is_none()
    {
        return None;
    }

    let mut lines = Vec::new();
    lines.push("Context summary (auto-generated):".to_string());

    if let Some(goal) = task_goal {
        lines.push(format!("Task goal: {}", goal));
    }

    if let Some(subtasks) = &subtasks {
        if let Some(active) = active_index {
            let active_title = subtasks
                .get(active)
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            lines.push(format!(
                "Active subtask: {} (step {}/{})",
                active_title,
                active + 1,
                subtasks.len()
            ));
        } else {
            lines.push(format!("Subtasks planned: {}", subtasks.join("; ")));
        }
    }

    if !file_actions.is_empty() {
        let joined = file_actions.join("; ");
        lines.push(format!("Recent file actions: {}", joined));
    }

    if let Some(error) = last_error {
        lines.push(format!("Last error: {}", error));
    }

    Some(Message::system(lines.join("\n")))
}

pub(super) fn extract_task_goal(context: &[Message]) -> Option<String> {
    for msg in context {
        if msg.role == MessageRole::User {
            if let Some(text) = msg.as_text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(truncate_text(trimmed, 240));
                }
            }
        }
    }
    None
}

pub(super) fn extract_plan_subtasks(context: &[Message]) -> (Option<Vec<String>>, Option<usize>) {
    let mut plan_subtasks: Option<Vec<String>> = None;
    let mut plan_index: Option<usize> = None;

    for (idx, msg) in context.iter().enumerate() {
        for call in &msg.tool_calls {
            if call.name == "agent_plan_subtasks" {
                if let Some(subtasks) = parse_subtask_titles(&call.arguments) {
                    plan_subtasks = Some(subtasks);
                    plan_index = Some(idx);
                }
            }
        }
    }

    let Some(subtasks) = &plan_subtasks else {
        return (None, None);
    };

    let mut advances = 0usize;
    if let Some(plan_idx) = plan_index {
        for msg in context.iter().skip(plan_idx + 1) {
            for call in &msg.tool_calls {
                if call.name == "agent_advance_to_next_subtask" {
                    advances += 1;
                }
            }
        }
    }

    let active_index = if advances < subtasks.len() {
        Some(advances)
    } else {
        None
    };

    (plan_subtasks, active_index)
}

pub(super) fn parse_subtask_titles(arguments: &JsonValue) -> Option<Vec<String>> {
    let subtasks = arguments.get("subtasks")?.as_array()?;
    let mut titles = Vec::new();
    for item in subtasks {
        let title = item.get("title")?.as_str()?.trim();
        if title.is_empty() {
            continue;
        }
        titles.push(truncate_text(title, 80));
    }
    if titles.is_empty() {
        None
    } else {
        Some(titles)
    }
}

pub(super) fn extract_recent_file_actions(context: &[Message], limit: usize) -> Vec<String> {
    let mut actions = Vec::new();
    for msg in context.iter().rev() {
        for call in &msg.tool_calls {
            let action = match call.name.as_str() {
                "fs_write_file" => "write",
                "fs_replace_in_file" => "edit",
                "fs_delete_file" => "delete",
                _ => continue,
            };
            let path = call
                .arguments
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            actions.push(format!("{} {}", action, path));
            if actions.len() >= limit {
                return actions;
            }
        }
    }
    actions.reverse();
    actions
}

pub(super) fn extract_last_tool_error(context: &[Message]) -> Option<String> {
    let mut call_map = std::collections::HashMap::new();
    for msg in context {
        for call in &msg.tool_calls {
            call_map.insert(call.id.clone(), (call.name.clone(), call.arguments.clone()));
        }
    }

    for msg in context.iter().rev() {
        if let Some(result) = &msg.tool_result {
            if result.is_error {
                let tool_name = call_map
                    .get(&result.call_id)
                    .map(|(name, _)| name.as_str())
                    .unwrap_or("tool");
                let error_text = result
                    .output
                    .get("error")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| truncate_text(&result.output.to_string(), 160));
                return Some(format!("{}: {}", tool_name, error_text));
            }
        }
    }
    None
}

pub(super) fn rank_retrieval_snippets(snippets: &[String]) -> Vec<String> {
    let mut scored: Vec<(i32, String)> = snippets
        .iter()
        .filter_map(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return None;
            }
            let path_boost = if trimmed.contains(".rs:")
                || trimmed.contains(".ts:")
                || trimmed.contains(".tsx:")
                || trimmed.contains(".js:")
                || trimmed.contains(".svelte:")
            {
                10
            } else {
                0
            };
            let len_penalty = (trimmed.len() / 80) as i32;
            Some((path_boost - len_penalty, trimmed.to_string()))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, s)| s).collect()
}

pub(super) fn collect_recent_dialogue_tail(context: &[Message], max_tokens: usize) -> Vec<Message> {
    let mut tail: Vec<Message> = Vec::new();
    let mut used = 0usize;
    let mut kept = 0usize;

    for msg in context.iter().rev() {
        if kept >= KEEP_RECENT_TAIL_MESSAGES {
            break;
        }
        if msg.role == MessageRole::System
            && msg
                .as_text()
                .is_some_and(|s| s.starts_with("ContextPolicyV2 "))
        {
            continue;
        }
        let tokens = estimate_message_tokens(msg);
        if used + tokens > max_tokens {
            break;
        }
        used += tokens;
        tail.push(msg.clone());
        kept += 1;
    }

    // Preserve tool_use/tool_result pairing. The backward walk can keep a
    // `Tool` message (small tool_result) while dropping its paired assistant
    // `tool_use` if that assistant message was too large for the budget or
    // fell outside the `KEEP_RECENT_TAIL_MESSAGES` cap. Shipping an orphan
    // tool_result to the provider produces a 400 error ("tool result's tool
    // id ... not found"). Drop any orphans before returning.
    let kept_tool_call_ids: std::collections::HashSet<String> = tail
        .iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .flat_map(|m| m.tool_calls.iter().map(|tc| tc.id.clone()))
        .collect();
    tail.retain(|m| match (&m.role, &m.tool_result) {
        (MessageRole::Tool, Some(result)) => kept_tool_call_ids.contains(&result.call_id),
        _ => true,
    });

    tail.reverse();
    tail
}
