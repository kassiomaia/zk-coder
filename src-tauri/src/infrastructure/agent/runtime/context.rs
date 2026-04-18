//! Context management utilities for agent runtime.
//!
//! Provides context compaction and token estimation for managing
//! LLM context window during agent execution.

#[allow(unused_imports)]
use crate::core::llm::entities::{
    ChatMessage as Message, ContentBlock, MessageContent, MessageRole, ToolCall, ToolResult,
};
use serde_json::Value as JsonValue;

/// Approximate context window size for token budgeting (conservative 128k).
pub const CONTEXT_WINDOW: usize = 128_000;

/// Threshold (60%) at which to compact the context.
pub const COMPRESSION_THRESHOLD: f64 = 0.60;

/// Returns the known context window size for a given model name.
/// Falls back to CONTEXT_WINDOW if the model is unknown.
pub fn model_context_window(model_name: &str) -> usize {
    let name = model_name.to_lowercase();
    if name.contains("claude") {
        200_000
    } else if name.contains("gemini-1.5") {
        1_000_000
    } else if name.contains("gpt-4-32k") {
        32_768
    } else if name.contains("gpt-4") {
        128_000
    } else if name.contains("gpt-3.5-turbo-16k") {
        16_384
    } else if name.contains("gpt-3.5") {
        4_096
    } else {
        CONTEXT_WINDOW // conservative default
    }
}

/// Computes the compression threshold ratio based on the model's context window.
/// Larger context windows compact later (0.70–0.80), smaller ones compact earlier (0.50–0.60).
pub fn adaptive_compression_threshold(model_name: &str) -> f64 {
    let window = model_context_window(model_name);
    match window {
        w if w >= 500_000 => 0.80,
        w if w >= 128_000 => 0.70,
        w if w >= 32_000 => 0.60,
        _ => 0.50,
    }
}

/// Number of recent messages to preserve during compaction.
pub const KEEP_RECENT_MESSAGES: usize = 15;
/// Number of recent dialogue messages to preserve in ContextPolicyV2 tail block.
pub const KEEP_RECENT_TAIL_MESSAGES: usize = 12;

/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_TASK_SUMMARY_TOKENS: usize = 180;
/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_ACTIVE_SUBTASK_TOKENS: usize = 120;
/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_FILE_RELEVANCE_TOKENS: usize = 800;
/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_FAILURE_MEMORY_TOKENS: usize = 140;
/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_RECENT_DIALOGUE_TOKENS: usize = 1600;
/// Hard token budget for the ContextPolicyV2 memory reference block (M3.2).
/// Sized to carry a compact identifier, a truncated active-plan summary, and
/// knowledge-item counts without inlining full memory contents.
pub const BUDGET_MEMORY_REFERENCE_TOKENS: usize = 180;
/// Hard token budget for the ContextPolicyV2 execution brief block (M4.1).
/// Sized to carry goal / active / recent / next / blocker lines near the
/// attention tail without letting any single field dominate the prompt.
pub const BUDGET_EXECUTION_BRIEF_TOKENS: usize = 240;

/// Flat token overhead per message (role + metadata framing).
pub const PER_MESSAGE_OVERHEAD: usize = 10;
/// Flat token framing cost added for each tool call on a message.
pub const PER_TOOL_CALL_FRAMING: usize = 4;
/// Flat token framing cost added for each tool result on a message.
pub const PER_TOOL_RESULT_FRAMING: usize = 4;
/// Conservative per-image token cost for multi-modal messages.
pub const IMAGE_TOKEN_COST: usize = 1_200;

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

/// Classifies a packing pass by comparing before/after counts and tokens.
fn classify_outcome(
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

/// Compact descriptor of a session's durable working-memory artifact
/// (Milestone 3.2). Used by `ContextPolicyV2` to inline a deterministic
/// reference block instead of copying the full memory body into every
/// prompt.
///
/// All fields must be bounded: `task_goal` and `active_plan` are
/// truncated inside the packer; the three `_count` fields replace the
/// unbounded `Vec<String>` fields on `SessionMemory` so the block stays
/// within its token budget even as the memory grows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryReference {
    /// The session the memory is attached to.
    pub session_id: String,
    /// Task goal (short user-facing sentence). Truncated by the packer.
    pub task_goal: String,
    /// A textual summary of the active plan. Truncated by the packer.
    pub active_plan: String,
    /// Number of completed findings recorded.
    pub completed_findings_count: usize,
    /// Number of open questions recorded.
    pub open_questions_count: usize,
    /// Number of high-value extracted facts recorded.
    pub extracted_facts_count: usize,
    /// RFC 3339 timestamp of the last memory update. Stable across
    /// packing passes when the underlying memory has not changed, so it
    /// can be used to reason about cache hits.
    pub updated_at_rfc3339: String,
}

/// Compact "what matters now" snapshot inlined near the recent-context
/// tail by `apply_context_policy_v2` (Milestone 4.1). The brief
/// reintroduces the current objective close to the model's attention
/// tail so long-running execution does not lose its goal across
/// compaction. All free-text fields are truncated inside the packer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecutionBrief {
    /// The overall task goal (short user-facing sentence).
    pub task_goal: String,
    /// Title of the currently in-progress subtask, if any.
    pub active_subtask: Option<String>,
    /// Short list of the most recent completed work items (successful
    /// tool calls, completed subtask titles). Order is oldest → newest.
    pub recently_completed: Vec<String>,
    /// The next intended action (typically the next pending subtask
    /// title). Empty when no further action is planned.
    pub next_intended_action: Option<String>,
    /// The current blocker, if any (usually a normalized error summary
    /// from `build_failure_memory`). Empty when unblocked.
    pub current_blocker: Option<String>,
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
}

/// Estimates the token count for a message context.
///
/// Counts the payload that the provider will actually serialize: text
/// content (including text inside `Blocks`), images (as a flat cost),
/// tool call arguments, and tool result outputs. A rough `len / 4` rule
/// approximates tokenization; the estimate is used for budgeting only.
pub fn estimate_context_tokens(context: &[Message]) -> usize {
    context.iter().map(estimate_message_tokens).sum()
}

/// Estimates the token cost of a single message, including tool calls and tool results.
pub fn estimate_message_tokens(msg: &Message) -> usize {
    let mut total = PER_MESSAGE_OVERHEAD;
    total = total.saturating_add(estimate_content_tokens(&msg.content));
    for call in &msg.tool_calls {
        total = total.saturating_add(estimate_tool_call_tokens(call));
    }
    if let Some(result) = &msg.tool_result {
        total = total.saturating_add(estimate_tool_result_tokens(result));
    }
    total
}

fn estimate_content_tokens(content: &MessageContent) -> usize {
    match content {
        MessageContent::Text(text) => text.len() / 4,
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .map(|block| match block {
                ContentBlock::Text { text } => text.len() / 4,
                ContentBlock::Image { .. } => IMAGE_TOKEN_COST,
            })
            .sum(),
    }
}

fn estimate_tool_call_tokens(call: &ToolCall) -> usize {
    call.name.len() / 4 + estimate_json_tokens(&call.arguments) + PER_TOOL_CALL_FRAMING
}

fn estimate_tool_result_tokens(result: &ToolResult) -> usize {
    estimate_json_tokens(&result.output) + PER_TOOL_RESULT_FRAMING
}

fn estimate_json_tokens(value: &JsonValue) -> usize {
    serde_json::to_string(value)
        .map(|s| s.len() / 4)
        .unwrap_or(0)
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

    if let Some(failure_memory) = build_failure_memory(context) {
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

fn build_compaction_summary(context: &[Message]) -> Option<Message> {
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

fn extract_task_goal(context: &[Message]) -> Option<String> {
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

fn extract_plan_subtasks(context: &[Message]) -> (Option<Vec<String>>, Option<usize>) {
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

fn parse_subtask_titles(arguments: &JsonValue) -> Option<Vec<String>> {
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

fn extract_recent_file_actions(context: &[Message], limit: usize) -> Vec<String> {
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

fn extract_last_tool_error(context: &[Message]) -> Option<String> {
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

/// Normalized classification of a tool failure. Drives the retryability
/// verdict carried by the M4.2 failure-memory block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// Target path / resource not found.
    FileNotFound,
    /// Access denied / not authorized.
    PermissionDenied,
    /// Operation exceeded its time budget.
    Timeout,
    /// Arguments failed validation (schema, format, range, etc.).
    ValidationFailed,
    /// Subtask mode rejected the call at runtime (M2.2 enforcement).
    NotAllowed,
    /// Default bucket when no specific class matches.
    Other,
}

impl FailureClass {
    /// Classifies an error message using bounded case-insensitive
    /// substring matching. Substring rules over parsing because tool
    /// errors arrive in many shapes; classification degrades to
    /// `Other` rather than failing.
    fn classify(error_text: &str) -> Self {
        let lower = error_text.to_lowercase();
        if lower.contains("subtask_mode_disallowed") || lower.contains("not allowed") {
            return Self::NotAllowed;
        }
        if lower.contains("no such file")
            || lower.contains("not found")
            || lower.contains("does not exist")
            || lower.contains("enoent")
        {
            return Self::FileNotFound;
        }
        if lower.contains("permission denied")
            || lower.contains("forbidden")
            || lower.contains("unauthorized")
            || lower.contains("eacces")
        {
            return Self::PermissionDenied;
        }
        if lower.contains("timed out") || lower.contains("timeout") {
            return Self::Timeout;
        }
        if lower.contains("invalid")
            || lower.contains("validation")
            || lower.contains("schema")
            || lower.contains("missing required")
        {
            return Self::ValidationFailed;
        }
        Self::Other
    }

    /// Stable snake_case label emitted into the failure-memory block.
    fn label(self) -> &'static str {
        match self {
            Self::FileNotFound => "file_not_found",
            Self::PermissionDenied => "permission_denied",
            Self::Timeout => "timeout",
            Self::ValidationFailed => "validation_failed",
            Self::NotAllowed => "not_allowed",
            Self::Other => "other",
        }
    }

    /// Whether it is safe to retry the same call with the same args.
    /// Conservative: classes that are known to be transient (Timeout)
    /// or unclassified (Other) are marked retryable; classes whose
    /// resolution requires changing the call (missing file, bad args,
    /// denied access, disallowed by mode) are marked non-retryable.
    fn retryable_unchanged(self) -> bool {
        matches!(self, Self::Timeout | Self::Other)
    }
}

/// Short stable fingerprint of a tool-call arguments JSON blob. The
/// output is 8 lowercase hex chars derived from the canonicalized JSON
/// so equivalent argument objects produce the same fingerprint across
/// runs. Used by M4.2 to detect "identical call retried" patterns.
fn arguments_fingerprint(args: &JsonValue) -> String {
    let canonical = serde_json::to_string(args).unwrap_or_default();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in canonical.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{:08x}", hash as u32)
}

/// Structured breakdown of the most recent failure, used by
/// `build_failure_memory` to emit a bounded, typed M4.2 block and by
/// callers that want the components individually (for example the
/// execution brief's `current_blocker` field).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureRecord {
    /// Failing tool name (or `"tool"` if the paired call could not be
    /// located in context).
    pub tool_name: String,
    /// Stable 8-char fingerprint of the call's arguments.
    pub args_fingerprint: String,
    /// Normalized class of the failure.
    pub class: FailureClass,
    /// Truncated error message extracted from the tool result.
    pub message: String,
    /// True when a previous call with the same tool + args fingerprint
    /// already failed earlier in the same context.
    pub repeat_detected: bool,
}

/// Walks the context backwards to find the most recent `is_error` tool
/// result and extract a structured `FailureRecord`. Returns `None` when
/// no tool error is visible. Detects repeat-failure patterns by
/// scanning earlier context for the same `tool + args_fingerprint`
/// pair paired with another error result.
pub fn extract_failure_record(context: &[Message]) -> Option<FailureRecord> {
    let mut call_map: std::collections::HashMap<String, (String, JsonValue)> =
        std::collections::HashMap::new();
    for msg in context {
        for call in &msg.tool_calls {
            call_map.insert(call.id.clone(), (call.name.clone(), call.arguments.clone()));
        }
    }

    // Error results (call_id, error_text) collected oldest → newest so
    // we can scan for prior occurrences of the latest error's
    // tool/args fingerprint.
    let mut error_results: Vec<(String, String)> = Vec::new();
    for msg in context {
        if let Some(result) = &msg.tool_result {
            if result.is_error {
                let error_text = result
                    .output
                    .get("error")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| truncate_text(&result.output.to_string(), 160));
                error_results.push((result.call_id.clone(), error_text));
            }
        }
    }
    let (last_call_id, last_error_text) = error_results.last()?.clone();
    let (tool_name, args) = call_map
        .get(&last_call_id)
        .cloned()
        .unwrap_or_else(|| ("tool".to_string(), JsonValue::Null));
    let args_fingerprint = arguments_fingerprint(&args);
    let class = FailureClass::classify(&last_error_text);
    let message = truncate_text(&last_error_text, 160);

    let mut repeat_detected = false;
    for (call_id, _error) in error_results.iter().take(error_results.len() - 1) {
        if let Some((prev_tool, prev_args)) = call_map.get(call_id) {
            if prev_tool == &tool_name && arguments_fingerprint(prev_args) == args_fingerprint {
                repeat_detected = true;
                break;
            }
        }
    }

    Some(FailureRecord {
        tool_name,
        args_fingerprint,
        class,
        message,
        repeat_detected,
    })
}

fn build_failure_memory(context: &[Message]) -> Option<String> {
    let record = extract_failure_record(context)?;
    let retry_line = if record.class.retryable_unchanged() {
        "retry: safe to retry (likely transient)"
    } else {
        "retry: change args before retry"
    };
    let mut out = String::from("Last error:");
    out.push_str(&format!("\ntool: {}", record.tool_name));
    out.push_str(&format!("\nargs_fp: {}", record.args_fingerprint));
    out.push_str(&format!("\ntype: {}", record.class.label()));
    out.push_str(&format!("\nmessage: {}", record.message));
    out.push_str(&format!("\n{retry_line}"));
    if record.repeat_detected {
        out.push_str("\nnote: identical call has failed before — do not retry unchanged");
    }
    Some(out)
}

fn rank_retrieval_snippets(snippets: &[String]) -> Vec<String> {
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

fn collect_recent_dialogue_tail(context: &[Message], max_tokens: usize) -> Vec<Message> {
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

fn truncate_text(text: &str, max_len: usize) -> String {
    crate::infrastructure::utils::truncate_text(text, max_len)
}

fn estimate_text_tokens(text: &str) -> usize {
    text.len() / 4 + 4
}

fn truncate_to_token_budget(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens.saturating_mul(4);
    truncate_text(text.trim(), max_chars)
}

/// Builds the deterministic text of the ContextPolicyV2 memory reference
/// block. The format is fixed so the same `MemoryReference` yields the
/// same bytes across packing passes — important for message-prefix hash
/// stability. Content is bounded to `BUDGET_MEMORY_REFERENCE_TOKENS`.
fn format_memory_reference_block(memory: &MemoryReference) -> String {
    // Roughly split the budget between the two free-text fields and the
    // metadata line. The metadata line is short and fixed-shape, so
    // `BUDGET_MEMORY_REFERENCE_TOKENS - 60` is a generous cap for the
    // free-text portions combined.
    let goal_budget = BUDGET_MEMORY_REFERENCE_TOKENS / 3;
    let plan_budget = BUDGET_MEMORY_REFERENCE_TOKENS / 3;
    let goal = truncate_to_token_budget(&memory.task_goal, goal_budget);
    let plan = truncate_to_token_budget(&memory.active_plan, plan_budget);
    let mut out = String::from("ContextPolicyV2 session memory reference:\n");
    out.push_str(&format!(
        "session={} updated_at={} findings={} questions={} facts={}",
        memory.session_id,
        memory.updated_at_rfc3339,
        memory.completed_findings_count,
        memory.open_questions_count,
        memory.extracted_facts_count,
    ));
    if !goal.is_empty() {
        out.push_str("\ngoal: ");
        out.push_str(&goal);
    }
    if !plan.is_empty() {
        out.push_str("\nplan: ");
        out.push_str(&plan);
    }
    out
}

/// Builds the deterministic text of the ContextPolicyV2 execution brief
/// block (M4.1). Missing fields are omitted, not rendered with empty
/// values, so the block only grows when the state behind it exists.
/// The total block is capped at `BUDGET_EXECUTION_BRIEF_TOKENS`; each
/// free-text field gets a proportional sub-budget.
fn format_execution_brief_block(brief: &ExecutionBrief) -> String {
    let per_field_budget = BUDGET_EXECUTION_BRIEF_TOKENS / 4;
    let mut out = String::from("ContextPolicyV2 execution brief:");
    let goal = truncate_to_token_budget(&brief.task_goal, per_field_budget);
    if !goal.is_empty() {
        out.push_str("\ngoal: ");
        out.push_str(&goal);
    }
    if let Some(ref active) = brief.active_subtask {
        let text = truncate_to_token_budget(active, per_field_budget);
        if !text.is_empty() {
            out.push_str("\nactive: ");
            out.push_str(&text);
        }
    }
    if !brief.recently_completed.is_empty() {
        let joined = brief
            .recently_completed
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        let text = truncate_to_token_budget(&joined, per_field_budget);
        if !text.is_empty() {
            out.push_str("\nrecent: ");
            out.push_str(&text);
        }
    }
    if let Some(ref next) = brief.next_intended_action {
        let text = truncate_to_token_budget(next, per_field_budget);
        if !text.is_empty() {
            out.push_str("\nnext: ");
            out.push_str(&text);
        }
    }
    if let Some(ref blocker) = brief.current_blocker {
        let text = truncate_to_token_budget(blocker, per_field_budget);
        if !text.is_empty() {
            out.push_str("\nblocker: ");
            out.push_str(&text);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_constants() {
        assert_eq!(CONTEXT_WINDOW, 128_000);
        assert_eq!(COMPRESSION_THRESHOLD, 0.60);
        assert_eq!(KEEP_RECENT_MESSAGES, 15);
    }

    #[test]
    fn test_estimate_context_tokens_empty() {
        let context: Vec<Message> = vec![];
        assert_eq!(estimate_context_tokens(&context), 0);
    }

    #[test]
    fn test_estimate_context_tokens_single_message() {
        let context = vec![Message::user("Hello, world!")];
        let estimate = estimate_context_tokens(&context);
        assert!(estimate > 0);
        // ~13 chars / 4 = ~3, +10 overhead = ~13
        assert!(estimate < 20); // Should be reasonable
    }

    #[test]
    fn test_estimate_context_tokens_multiple_messages() {
        let context = vec![
            Message::system("You are a helpful assistant."),
            Message::user("Hello!"),
            Message::assistant("Hi there!"),
        ];
        let estimate = estimate_context_tokens(&context);
        assert!(estimate > 0);
        // Should be roughly sum of all content / 4 + overhead
        assert!(estimate < 100); // Should be reasonable for short messages
    }

    #[test]
    fn test_estimate_context_tokens_with_long_content() {
        let long_content = "A".repeat(1000);
        let context = vec![Message::user(&long_content)];
        let estimate = estimate_context_tokens(&context);
        // ~1000 chars / 4 = ~250, +10 overhead = ~260
        assert!(estimate > 200);
        assert!(estimate < 300);
    }

    #[test]
    fn test_estimate_context_tokens_with_tool_calls() {
        let context = vec![Message::assistant_with_tool_calls(
            "I'll read the file.".to_string(),
            vec![ToolCall {
                id: "call_1".to_string(),
                name: "fs_read_file".to_string(),
                arguments: json!({"path": "/test.txt"}),
            }],
        )];

        let estimate = estimate_context_tokens(&context);
        assert!(estimate > 0);
    }

    #[test]
    fn test_context_compaction_below_threshold() {
        let mut context = vec![
            Message::system("You are a helpful assistant."),
            Message::user("Hello!"),
            Message::assistant("Hi there!"),
        ];

        let result = maybe_compact_context(&mut context, "gpt-4");
        assert!(result.is_none());
        assert_eq!(context.len(), 3); // Unchanged
    }

    #[test]
    fn test_truncate_text_handles_multibyte_boundary() {
        let text = format!("{}{}{}", "a".repeat(499), "─", "b");

        let truncated = truncate_text(&text, 500);

        assert!(truncated.ends_with("..."));
        assert!(truncated.starts_with(&"a".repeat(499)));
    }

    #[test]
    fn test_context_compaction_exactly_at_threshold() {
        // Create context just at threshold edge (60% of 128k = ~76800 tokens)
        // At ~30 chars per message, need ~2560 messages
        let mut context = vec![Message::system("System")];
        for i in 0..1280 {
            context.push(Message::user(&format!("User message {}", i)));
            context.push(Message::assistant(&format!("Assistant response {}", i)));
        }

        let before_len = context.len();
        let result = maybe_compact_context(&mut context, "gpt-4");

        // At exactly threshold, should still compact
        // (the function checks usage_ratio < threshold for no compaction)
        // So at >= threshold, it should compact
        if result.is_some() {
            assert!(context.len() < before_len);
        }
    }

    #[test]
    fn test_context_compaction_above_threshold() {
        // Create a context large enough to trigger compaction
        // For gpt-4 (128k @ 60% = ~76800 tokens), need enough to exceed threshold
        // Using longer messages to reach threshold faster
        let mut context = vec![Message::system("System prompt")];
        for i in 0..3500 {
            context.push(Message::user(&format!("User message {} with some additional content to increase token count", i)));
            context.push(Message::assistant(&format!(
                "Assistant response {} with more content here to exceed threshold",
                i
            )));
        }

        let result = maybe_compact_context(&mut context, "gpt-4");

        assert!(result.is_some());
        let details = result.unwrap();
        assert!(details.after_count < details.before_count);
        assert_eq!(context.len(), details.after_count);

        // First message preserved
        assert_eq!(context[0].role, MessageRole::System);

        // Recent messages preserved (+1 for summary)
        assert!(context.len() <= KEEP_RECENT_MESSAGES + 2);
    }

    #[test]
    fn test_context_compaction_keeps_first_and_recent_messages() {
        // Create a large context to trigger compaction (>60% of 128k tokens = ~76800 tokens)
        // Need enough content per message to exceed the threshold
        let long_content = "A".repeat(100); // 100 chars per message
        let mut context = vec![Message::system("System")];
        for i in 0..2000 {
            context.push(Message::user(format!(
                "User {} message with long content: {}",
                i, long_content
            )));
            context.push(Message::assistant(format!(
                "Assistant {} response with long content: {}",
                i, long_content
            )));
        }

        let before_len = context.len();
        let result = maybe_compact_context(&mut context, "gpt-4");

        // Verify compaction occurred
        assert!(
            result.is_some(),
            "Compaction should occur with enough content"
        );
        assert!(
            context.len() < before_len,
            "Context should be smaller after compaction"
        );

        // First message should be preserved
        assert_eq!(context[0].as_text(), Some("System"));

        // After compaction, we should have first message + summary + recent messages
        assert!(context.len() <= KEEP_RECENT_MESSAGES + 2);
    }

    #[test]
    fn test_context_compaction_details() {
        let mut context = vec![Message::system("System")];
        // Need enough messages to exceed 60% of gpt-4's 128k context window
        // Using longer messages to reach token threshold faster
        for i in 0..3500 {
            context.push(Message::user(&format!("User message {} with extended content", i)));
            context.push(Message::assistant(&format!("Assistant response {} with additional text", i)));
        }

        let result = maybe_compact_context(&mut context, "gpt-4");
        assert!(result.is_some());

        let details = result.unwrap();
        assert!(details.before_count > details.after_count);
        assert!(details.before_tokens > details.after_tokens);
        assert!(details.needs_reinjection);
    }

    #[test]
    fn test_context_compaction_small_context() {
        // Small context should not compact even if we try
        let mut context = vec![
            Message::system("System"),
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];

        let result = maybe_compact_context(&mut context, "gpt-4");
        assert!(result.is_none(), "Small context should not compact");
        assert_eq!(context.len(), 3);
    }

    #[test]
    fn test_context_compaction_exactly_keep_recent_plus_one() {
        // If context is exactly KEEP_RECENT_MESSAGES + 1, no compaction needed
        let mut context = vec![Message::system("System")];
        for i in 0..KEEP_RECENT_MESSAGES {
            context.push(Message::user(&format!("Message {}", i)));
        }

        // This should be exactly KEEP_RECENT_MESSAGES + 1
        assert_eq!(context.len(), KEEP_RECENT_MESSAGES + 1);

        let result = maybe_compact_context(&mut context, "gpt-4");
        // With small messages, we're likely below token threshold
        assert!(result.is_none());
    }

    #[test]
    fn test_context_compaction_preserves_system_message() {
        let mut context = vec![Message::system("Important system prompt")];
        for i in 0..3000 {
            context.push(Message::user(&format!("User {}", i)));
            context.push(Message::assistant(&format!("Assistant {}", i)));
        }

        maybe_compact_context(&mut context, "gpt-4");

        // System message should always be first
        assert_eq!(context[0].role, MessageRole::System);
        assert_eq!(context[0].as_text(), Some("Important system prompt"));
    }

    #[test]
    fn test_context_compaction_with_mixed_message_types() {
        let mut context = vec![Message::system("System")];
        // Need enough messages to exceed 60% of CONTEXT_WINDOW (76800 tokens)
        // Need longer messages to exceed 60% of gpt-4's 128k window
        for i in 0..3500 {
            context.push(Message::user(&format!("User message {} with extended content to reach threshold", i)));
            context.push(Message::assistant_with_tool_calls(
                format!("Response {} with additional text", i),
                vec![ToolCall {
                    id: format!("call_{}", i),
                    name: "test_tool".to_string(),
                    arguments: json!({"index": i}),
                }],
            ));
        }

        let result = maybe_compact_context(&mut context, "gpt-4");
        assert!(result.is_some());

        // Should still have system message
        assert_eq!(context[0].role, MessageRole::System);
        // Should be reduced to system + summary + recent
        assert!(context.len() <= KEEP_RECENT_MESSAGES + 2);
    }

    #[test]
    fn test_compaction_details_needs_reinjection_flag() {
        let mut context = vec![Message::system("System")];
        // Need enough messages to exceed 60% of gpt-4's 128k context window
        // With longer messages to ensure we hit the threshold reliably
        for i in 0..5000 {
            context.push(Message::user(&format!("User message {} with extended content to reach token limit threshold", i)));
        }

        let result = maybe_compact_context(&mut context, "gpt-4");
        assert!(result.is_some());

        let details = result.unwrap();
        assert!(
            details.needs_reinjection,
            "Compaction should always signal re-injection needed"
        );
    }

    #[test]
    fn test_context_window_constant() {
        // Verify the context window is set correctly
        assert!(CONTEXT_WINDOW > 0);
        assert_eq!(CONTEXT_WINDOW, 128_000);
    }

    #[test]
    fn test_compression_threshold_constant() {
        // Threshold should be between 0 and 1
        assert!(COMPRESSION_THRESHOLD > 0.0);
        assert!(COMPRESSION_THRESHOLD <= 1.0);
        assert_eq!(COMPRESSION_THRESHOLD, 0.60);
    }

    #[test]
    fn test_keep_recent_messages_constant() {
        // Should keep a reasonable number of recent messages
        assert!(KEEP_RECENT_MESSAGES > 0);
        assert!(KEEP_RECENT_MESSAGES < 100);
        assert_eq!(KEEP_RECENT_MESSAGES, 15);
    }

    #[test]
    fn test_context_compaction_token_estimation_consistency() {
        let mut context = vec![Message::system("System")];
        for i in 0..1000 {
            context.push(Message::user(&format!("Message {}", i)));
        }

        let before_tokens = estimate_context_tokens(&context);

        let result = maybe_compact_context(&mut context, "gpt-4");
        if let Some(details) = result {
            assert_eq!(details.before_tokens, before_tokens);
            assert!(details.after_tokens < before_tokens);
        }
    }

    fn large_json_string(approx_bytes: usize) -> String {
        "x".repeat(approx_bytes)
    }

    #[test]
    fn test_estimate_counts_tool_call_arguments() {
        let big_payload = large_json_string(4_000);
        let msg_with = Message::assistant_with_tool_calls(
            "text".to_string(),
            vec![ToolCall {
                id: "call_1".into(),
                name: "fs_write".into(),
                arguments: json!({ "contents": big_payload }),
            }],
        );
        let msg_without = Message::assistant("text");

        let with_tokens = estimate_message_tokens(&msg_with);
        let without_tokens = estimate_message_tokens(&msg_without);

        // The 4KB JSON payload should contribute ~1000 tokens under len/4.
        assert!(
            with_tokens >= without_tokens + 900,
            "tool-call JSON should be counted (with={with_tokens}, without={without_tokens})"
        );
    }

    #[test]
    fn test_estimate_counts_tool_result_output() {
        let big_output = json!({ "data": large_json_string(4_000) });
        let tool_msg = Message::tool_result("call_1", big_output, false);
        let tokens = estimate_message_tokens(&tool_msg);

        // Old estimator would have returned exactly PER_MESSAGE_OVERHEAD (10).
        assert!(
            tokens > PER_MESSAGE_OVERHEAD + 900,
            "tool-result JSON should be counted, got {tokens}"
        );
    }

    #[test]
    fn test_estimate_counts_blocks_content() {
        let msg = Message::multimodal(
            MessageRole::User,
            vec![
                ContentBlock::text("A".repeat(400)),
                ContentBlock::text("B".repeat(400)),
            ],
        );
        let tokens = estimate_message_tokens(&msg);
        // Old estimator returned 0 content for Blocks; with ~800 chars it should be ~200 tokens.
        assert!(tokens > 150, "text inside Blocks must be counted, got {tokens}");
    }

    #[test]
    fn test_estimate_image_block_has_fixed_cost() {
        let huge_base64 = "A".repeat(100_000);
        let msg = Message::multimodal(
            MessageRole::User,
            vec![ContentBlock::image_base64("image/png", huge_base64)],
        );
        let tokens = estimate_message_tokens(&msg);
        // Image cost is flat — not proportional to base64 length.
        let lower = PER_MESSAGE_OVERHEAD + IMAGE_TOKEN_COST;
        let upper = PER_MESSAGE_OVERHEAD + IMAGE_TOKEN_COST + 10;
        assert!(
            (lower..=upper).contains(&tokens),
            "image cost should be fixed near IMAGE_TOKEN_COST, got {tokens}"
        );
    }

    fn sample_memory_reference() -> MemoryReference {
        MemoryReference {
            session_id: "sess-42".to_string(),
            task_goal: "investigate auth regression".to_string(),
            active_plan: "review auth.rs; patch login flow; add tests".to_string(),
            completed_findings_count: 2,
            open_questions_count: 1,
            extracted_facts_count: 3,
            updated_at_rfc3339: "2026-04-17T12:00:00+00:00".to_string(),
        }
    }

    fn memory_block_from(messages: &[Message]) -> Option<&str> {
        messages.iter().find_map(|m| match &m.content {
            MessageContent::Text(text)
                if m.role == MessageRole::System
                    && text.starts_with("ContextPolicyV2 session memory reference:") =>
            {
                Some(text.as_str())
            }
            _ => None,
        })
    }

    fn build_above_threshold_context() -> Vec<Message> {
        // Large enough to exceed the 70% threshold for gpt-4 so the
        // packer runs without needing `force_pack`.
        let padding = "x".repeat(400);
        let mut context = vec![Message::system("System")];
        for i in 0..1_000 {
            context.push(Message::user(format!("msg {i} {padding}")));
        }
        context
    }

    #[test]
    fn policy_v2_inlines_memory_reference_block_when_provided() {
        let mut context = build_above_threshold_context();
        let input = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            memory_reference: Some(sample_memory_reference()),
            ..ContextPolicyInput::default()
        };
        let result = apply_context_policy_v2(&mut context, &input);
        assert!(result.is_some(), "large context should pack");
        let block = memory_block_from(&context).expect("memory block must be present");
        assert!(block.contains("session=sess-42"));
        assert!(block.contains("findings=2"));
        assert!(block.contains("questions=1"));
        assert!(block.contains("facts=3"));
        assert!(block.contains("investigate auth regression"));
    }

    #[test]
    fn policy_v2_omits_memory_reference_block_when_none() {
        let mut context = build_above_threshold_context();
        let input = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            memory_reference: None,
            ..ContextPolicyInput::default()
        };
        let result = apply_context_policy_v2(&mut context, &input);
        assert!(result.is_some());
        assert!(
            memory_block_from(&context).is_none(),
            "no memory reference input => no memory block in packed context",
        );
    }

    #[test]
    fn policy_v2_memory_reference_block_is_deterministic() {
        // Two independent packing passes with the same MemoryReference
        // must produce byte-identical blocks. Drives message_prefix_hash
        // stability (Milestone 1.3 invariant).
        let mut ctx_a = build_above_threshold_context();
        let mut ctx_b = build_above_threshold_context();
        let input_a = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            memory_reference: Some(sample_memory_reference()),
            ..ContextPolicyInput::default()
        };
        let input_b = input_a.clone();
        apply_context_policy_v2(&mut ctx_a, &input_a).unwrap();
        apply_context_policy_v2(&mut ctx_b, &input_b).unwrap();
        assert_eq!(memory_block_from(&ctx_a), memory_block_from(&ctx_b));
    }

    #[test]
    fn policy_v2_memory_reference_block_truncates_oversized_fields() {
        let huge = "y".repeat(10_000);
        let memory = MemoryReference {
            task_goal: huge.clone(),
            active_plan: huge,
            ..sample_memory_reference()
        };
        let mut context = build_above_threshold_context();
        let input = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            memory_reference: Some(memory),
            ..ContextPolicyInput::default()
        };
        apply_context_policy_v2(&mut context, &input).unwrap();
        let block = memory_block_from(&context).expect("memory block must exist");
        // Budget (180) split in thirds => ~60 tokens each => ~240 chars each;
        // a 10_000-char field must be truncated well below its own length.
        // 1_500 chars is a generous upper bound that still proves truncation.
        assert!(
            block.len() < 1_500,
            "memory block should be truncated; got {} chars",
            block.len()
        );
    }

    #[test]
    fn policy_v2_tiny_forced_context_packs_when_memory_reference_is_present() {
        // Without a memory_reference, a tiny forced context short-circuits
        // and skips packing. With one, the packer must run so the memory
        // block is inlined — otherwise compaction could silently drop the
        // only pointer to durable memory.
        let mut context = vec![
            Message::system("System"),
            Message::user("Hi"),
            Message::assistant("Hello"),
        ];
        let input = ContextPolicyInput {
            force_pack: true,
            model_name: "gpt-4".to_string(),
            memory_reference: Some(sample_memory_reference()),
            ..ContextPolicyInput::default()
        };
        let result = apply_context_policy_v2(&mut context, &input);
        assert!(result.is_some(), "memory-bearing tiny forced pack must run");
        assert!(
            memory_block_from(&context).is_some(),
            "memory reference must survive the packing pass",
        );
    }

    #[test]
    fn policy_v2_repeated_packing_does_not_duplicate_memory_reference_block() {
        // Safety net against an earlier pass's ContextPolicyV2 block
        // leaking through into the next pack, causing the memory
        // reference to appear twice. The tail collector filters out
        // existing "ContextPolicyV2 " system messages; this test pins
        // that invariant end-to-end for the memory block.
        //
        // The second call uses force_pack so it runs even when the
        // first pass already shrank the context below the compaction
        // threshold.
        let mut context = build_above_threshold_context();
        let input = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            memory_reference: Some(sample_memory_reference()),
            ..ContextPolicyInput::default()
        };
        apply_context_policy_v2(&mut context, &input).unwrap();
        let force_input = ContextPolicyInput {
            force_pack: true,
            ..input
        };
        apply_context_policy_v2(&mut context, &force_input).unwrap();
        let count = context
            .iter()
            .filter(|m| {
                m.role == MessageRole::System
                    && m.as_text()
                        .is_some_and(|t| t.starts_with("ContextPolicyV2 session memory reference:"))
            })
            .count();
        assert_eq!(count, 1, "memory block must not duplicate across packs");
    }

    #[test]
    fn test_apply_policy_v2_skips_tiny_forced_context() {
        let mut context = vec![
            Message::system("System"),
            Message::user("Hi"),
            Message::assistant("Hello"),
        ];
        let input = ContextPolicyInput {
            force_pack: true,
            model_name: "gpt-4".to_string(),
            ..ContextPolicyInput::default()
        };
        let result = apply_context_policy_v2(&mut context, &input);
        assert!(result.is_none(), "tiny forced context must skip packing");
        assert_eq!(context.len(), 3, "context must not be mutated when skipping");
    }

    #[test]
    fn test_apply_policy_v2_classifies_unchanged_correctly() {
        // Build a context that exceeds threshold but whose packed form matches input shape.
        // Hitting an exact match is hard; assert that classify_outcome agrees on identity.
        assert_eq!(
            classify_outcome(500, 500, 10, 10),
            ContextPackOutcome::Unchanged,
        );
    }

    #[test]
    fn test_classify_outcome_reorganized_when_insufficient_savings() {
        // 50-token savings on a 1000-token context: below both thresholds.
        assert_eq!(
            classify_outcome(1_000, 950, 10, 8),
            ContextPackOutcome::Reorganized,
        );
    }

    #[test]
    fn test_classify_outcome_compacted_when_real_savings() {
        // 6000-token savings on a 10000-token context: 60% > MIN_SAVINGS_RATIO.
        assert_eq!(
            classify_outcome(10_000, 4_000, 50, 10),
            ContextPackOutcome::Compacted,
        );
    }

    #[test]
    fn test_maybe_compact_context_populates_outcome() {
        // Build a context well above gpt-4's 70% threshold (~89.6k tokens).
        let padding = "x".repeat(400);
        let mut context = vec![Message::system("System")];
        for i in 0..1_000 {
            context.push(Message::user(format!("msg {i} {padding}")));
        }

        let result = maybe_compact_context(&mut context, "gpt-4");
        let details = result.expect("large context should compact");
        assert_eq!(details.outcome, ContextPackOutcome::Compacted);
        assert!(details.after_tokens < details.before_tokens);
    }

    // --- collect_recent_dialogue_tail pairing invariant ---
    //
    // These tests enforce the invariant that the returned tail never contains a
    // tool_result whose paired tool_use (on an assistant message) was dropped.
    // Violating this invariant is what caused the production 400 on session
    // 0ceae112 step 17: "tool result's tool id(call_function_...) not found".

    fn assistant_with_call(id: &str, args_padding_bytes: usize) -> Message {
        let content = "x".repeat(args_padding_bytes);
        Message::assistant_with_tool_calls(
            "writing".to_string(),
            vec![ToolCall {
                id: id.to_string(),
                name: "fs_write_file".to_string(),
                arguments: json!({"path": "docs/foo.md", "content": content}),
            }],
        )
    }

    fn tool_result_for(id: &str) -> Message {
        Message::tool_result(id, json!({"bytes_written": 42}), false)
    }

    fn orphan_tool_result_ids(tail: &[Message]) -> Vec<String> {
        let assistant_ids: std::collections::HashSet<String> = tail
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .flat_map(|m| m.tool_calls.iter().map(|tc| tc.id.clone()))
            .collect();
        tail.iter()
            .filter(|m| m.role == MessageRole::Tool)
            .filter_map(|m| m.tool_result.as_ref().map(|r| r.call_id.clone()))
            .filter(|id| !assistant_ids.contains(id))
            .collect()
    }

    #[test]
    fn collect_recent_tail_drops_orphan_when_tool_use_exceeds_token_budget() {
        // Reproduces the production incident: an fs_write_file tool_use carrying
        // a large `content` argument blows past the 1600-token tail budget, so
        // the backward walk `break`s on that assistant message. Without the fix,
        // its cheap tool_result (already pushed) is left as an orphan.
        //
        // ~15_000-byte arguments => ~3750 tokens, well beyond 1600.
        let context = vec![
            Message::user("please write the file".to_string()),
            assistant_with_call("call_X", 15_000),
            tool_result_for("call_X"),
            Message::user("next step please".to_string()),
            Message::assistant("ok, continuing".to_string()),
        ];

        let tail = collect_recent_dialogue_tail(&context, BUDGET_RECENT_DIALOGUE_TOKENS);

        assert!(
            orphan_tool_result_ids(&tail).is_empty(),
            "tail must not contain orphan tool_results; got tail: {tail:#?}",
        );
    }

    #[test]
    fn collect_recent_tail_drops_orphan_when_tool_use_outside_message_window() {
        // KEEP_RECENT_TAIL_MESSAGES caps the walk at 12 kept messages. If an
        // assistant tool_use sits just outside that window while its tool_result
        // sits just inside, the pair splits. Distinct from the token-budget
        // path above.
        //
        // Layout (21 messages, indices 0..20):
        //   idx  0-8 : small chatter
        //   idx  9   : assistant tool_use  call_Y  <- MUST be at the boundary
        //   idx 10   : tool_result         call_Y
        //   idx 11-20: 10 more small messages
        //
        // Backward walk: 20,19,...,10 = 11 messages kept, then idx 9 would be
        // the 12th and fits the cap. So to push idx 10 in but idx 9 out we need
        // one more message past idx 20. Use 22 messages total; assistant at
        // idx 9, tool_result at idx 10, chatter idx 11..21.
        let mut context: Vec<Message> = (0..9)
            .map(|i| Message::user(format!("chatter {i}")))
            .collect();
        context.push(assistant_with_call("call_Y", 4));
        context.push(tool_result_for("call_Y"));
        for i in 0..11 {
            context.push(Message::user(format!("tail chatter {i}")));
        }
        assert_eq!(context.len(), 22);

        let tail = collect_recent_dialogue_tail(&context, 100_000);

        assert!(
            tail.len() <= KEEP_RECENT_TAIL_MESSAGES,
            "tail length {} exceeds KEEP_RECENT_TAIL_MESSAGES {}",
            tail.len(),
            KEEP_RECENT_TAIL_MESSAGES,
        );
        assert!(
            orphan_tool_result_ids(&tail).is_empty(),
            "tail must not contain orphan tool_results; got tail: {tail:#?}",
        );
    }

    #[test]
    fn collect_recent_tail_keeps_matched_pair_when_both_fit() {
        // Baseline: when both members of the pair fit, both must be preserved.
        let context = vec![
            assistant_with_call("call_Z", 20),
            tool_result_for("call_Z"),
            Message::user("thanks".to_string()),
        ];

        let tail = collect_recent_dialogue_tail(&context, BUDGET_RECENT_DIALOGUE_TOKENS);

        let has_assistant = tail
            .iter()
            .any(|m| m.role == MessageRole::Assistant && !m.tool_calls.is_empty());
        let has_tool_result = tail.iter().any(|m| m.role == MessageRole::Tool);
        assert!(
            has_assistant && has_tool_result,
            "both pair members should survive; got tail: {tail:#?}",
        );
        assert!(orphan_tool_result_ids(&tail).is_empty());
    }

    #[test]
    fn collect_recent_tail_reproduces_session_0ceae112_incident() {
        // Exact shape of the production failure: fs_write_file to
        // docs/cxone/quick-reference.md with ~11593 bytes of content, followed
        // by a tiny tool_result, followed by one more tool round-trip, then a
        // subtask transition. Before the fix, compaction leaves the 11593-byte
        // tool_use out of the tail while its tool_result stays in.
        let huge_content = "#".repeat(11_593);
        let context = vec![
            Message::user("extract business logic".to_string()),
            Message::assistant_with_tool_calls(
                "creating the quick reference".to_string(),
                vec![ToolCall {
                    id: "call_function_snasv7li0myh_1".to_string(),
                    name: "fs_write_file".to_string(),
                    arguments: json!({
                        "path": "docs/cxone/quick-reference.md",
                        "content": huge_content,
                    }),
                }],
            ),
            Message::tool_result(
                "call_function_snasv7li0myh_1",
                json!({"bytes_written": 11_593, "path": "docs/cxone/quick-reference.md"}),
                false,
            ),
            Message::assistant_with_tool_calls(
                "listing directory".to_string(),
                vec![ToolCall {
                    id: "call_function_list_1".to_string(),
                    name: "fs_list_files".to_string(),
                    arguments: json!({"path": "docs/cxone"}),
                }],
            ),
            Message::tool_result(
                "call_function_list_1",
                json!({"count": 8}),
                false,
            ),
            Message::assistant_with_tool_calls(
                "advancing".to_string(),
                vec![ToolCall {
                    id: "call_function_advance_1".to_string(),
                    name: "agent_advance_to_next_subtask".to_string(),
                    arguments: json!({}),
                }],
            ),
            Message::tool_result(
                "call_function_advance_1",
                json!({"success": true}),
                false,
            ),
            Message::user("now compile the knowledge base".to_string()),
        ];

        let tail = collect_recent_dialogue_tail(&context, BUDGET_RECENT_DIALOGUE_TOKENS);

        let orphans = orphan_tool_result_ids(&tail);
        assert!(
            orphans.is_empty(),
            "session 0ceae112 incident must not repeat: orphan tool_call_ids in tail: {orphans:?}; tail: {tail:#?}",
        );
    }

    // --- M4.1 execution brief tests ---

    fn sample_execution_brief() -> ExecutionBrief {
        ExecutionBrief {
            task_goal: "fix failing login flow".to_string(),
            active_subtask: Some("patch token refresh".to_string()),
            recently_completed: vec![
                "audit auth.rs".to_string(),
                "repro failing test".to_string(),
            ],
            next_intended_action: Some("add regression test".to_string()),
            current_blocker: None,
        }
    }

    fn brief_block_from(messages: &[Message]) -> Option<&str> {
        messages.iter().find_map(|m| match &m.content {
            MessageContent::Text(text)
                if m.role == MessageRole::System
                    && text.starts_with("ContextPolicyV2 execution brief:") =>
            {
                Some(text.as_str())
            }
            _ => None,
        })
    }

    #[test]
    fn policy_v2_inlines_execution_brief_block_when_provided() {
        let mut context = build_above_threshold_context();
        let input = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            execution_brief: Some(sample_execution_brief()),
            ..ContextPolicyInput::default()
        };
        apply_context_policy_v2(&mut context, &input).expect("large context should pack");
        let block = brief_block_from(&context).expect("brief block must be present");
        assert!(block.contains("goal: fix failing login flow"));
        assert!(block.contains("active: patch token refresh"));
        assert!(block.contains("recent: audit auth.rs; repro failing test"));
        assert!(block.contains("next: add regression test"));
        assert!(
            !block.contains("blocker:"),
            "brief without blocker must not emit a blocker line; got {block}",
        );
    }

    #[test]
    fn policy_v2_omits_execution_brief_block_when_none() {
        let mut context = build_above_threshold_context();
        let input = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            execution_brief: None,
            ..ContextPolicyInput::default()
        };
        apply_context_policy_v2(&mut context, &input).expect("large context should pack");
        assert!(
            brief_block_from(&context).is_none(),
            "no execution brief input => no brief block in packed context",
        );
    }

    #[test]
    fn policy_v2_execution_brief_block_is_deterministic() {
        let mut ctx_a = build_above_threshold_context();
        let mut ctx_b = build_above_threshold_context();
        let input_a = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            execution_brief: Some(sample_execution_brief()),
            ..ContextPolicyInput::default()
        };
        let input_b = input_a.clone();
        apply_context_policy_v2(&mut ctx_a, &input_a).unwrap();
        apply_context_policy_v2(&mut ctx_b, &input_b).unwrap();
        assert_eq!(brief_block_from(&ctx_a), brief_block_from(&ctx_b));
    }

    #[test]
    fn policy_v2_execution_brief_omits_missing_optional_fields() {
        let brief = ExecutionBrief {
            task_goal: "audit".to_string(),
            active_subtask: None,
            recently_completed: vec![],
            next_intended_action: None,
            current_blocker: None,
        };
        let mut context = build_above_threshold_context();
        let input = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            execution_brief: Some(brief),
            ..ContextPolicyInput::default()
        };
        apply_context_policy_v2(&mut context, &input).unwrap();
        let block = brief_block_from(&context).expect("brief block must be present");
        assert!(block.contains("goal: audit"));
        assert!(!block.contains("\nactive:"));
        assert!(!block.contains("\nrecent:"));
        assert!(!block.contains("\nnext:"));
        assert!(!block.contains("\nblocker:"));
    }

    #[test]
    fn policy_v2_execution_brief_reflects_changed_active_subtask() {
        let brief_v1 = ExecutionBrief {
            active_subtask: Some("read code".to_string()),
            ..sample_execution_brief()
        };
        let brief_v2 = ExecutionBrief {
            active_subtask: Some("write fix".to_string()),
            ..sample_execution_brief()
        };
        let mut ctx_a = build_above_threshold_context();
        let mut ctx_b = build_above_threshold_context();
        apply_context_policy_v2(
            &mut ctx_a,
            &ContextPolicyInput {
                model_name: "gpt-4".to_string(),
                execution_brief: Some(brief_v1),
                ..ContextPolicyInput::default()
            },
        )
        .unwrap();
        apply_context_policy_v2(
            &mut ctx_b,
            &ContextPolicyInput {
                model_name: "gpt-4".to_string(),
                execution_brief: Some(brief_v2),
                ..ContextPolicyInput::default()
            },
        )
        .unwrap();
        assert_ne!(
            brief_block_from(&ctx_a),
            brief_block_from(&ctx_b),
            "subtask transition must move the brief block",
        );
    }

    #[test]
    fn policy_v2_repeated_packing_does_not_duplicate_execution_brief_block() {
        let mut context = build_above_threshold_context();
        let input = ContextPolicyInput {
            model_name: "gpt-4".to_string(),
            execution_brief: Some(sample_execution_brief()),
            ..ContextPolicyInput::default()
        };
        apply_context_policy_v2(&mut context, &input).unwrap();
        let force_input = ContextPolicyInput {
            force_pack: true,
            ..input
        };
        apply_context_policy_v2(&mut context, &force_input).unwrap();
        let count = context
            .iter()
            .filter(|m| {
                m.role == MessageRole::System
                    && m.as_text()
                        .is_some_and(|t| t.starts_with("ContextPolicyV2 execution brief:"))
            })
            .count();
        assert_eq!(count, 1, "brief block must not duplicate across packs");
    }

    #[test]
    fn policy_v2_tiny_forced_pack_runs_when_execution_brief_is_present() {
        // Mirrors the memory-reference exception: without a brief a
        // tiny-context forced pack short-circuits, but with one the
        // packer must run so the brief is inlined.
        let mut context = vec![
            Message::system("System"),
            Message::user("Hi"),
            Message::assistant("Hello"),
        ];
        let input = ContextPolicyInput {
            force_pack: true,
            model_name: "gpt-4".to_string(),
            execution_brief: Some(sample_execution_brief()),
            ..ContextPolicyInput::default()
        };
        let result = apply_context_policy_v2(&mut context, &input);
        assert!(result.is_some(), "brief-bearing tiny forced pack must run");
        assert!(
            brief_block_from(&context).is_some(),
            "brief must survive the packing pass",
        );
    }

    // --- M4.2 enriched failure memory tests ---

    fn context_with_failed_tool_call(
        tool_name: &str,
        args: JsonValue,
        error_message: &str,
    ) -> Vec<Message> {
        vec![
            Message::user("go"),
            Message::assistant_with_tool_calls(
                "running".to_string(),
                vec![ToolCall {
                    id: "call_err_1".to_string(),
                    name: tool_name.to_string(),
                    arguments: args,
                }],
            ),
            Message::tool_result("call_err_1", json!({ "error": error_message }), true),
        ]
    }

    #[test]
    fn failure_class_classifies_common_errors() {
        assert_eq!(
            FailureClass::classify("No such file or directory (ENOENT)"),
            FailureClass::FileNotFound,
        );
        assert_eq!(
            FailureClass::classify("Permission denied"),
            FailureClass::PermissionDenied,
        );
        assert_eq!(
            FailureClass::classify("operation timed out after 30s"),
            FailureClass::Timeout,
        );
        assert_eq!(
            FailureClass::classify("Invalid argument: missing required field 'path'"),
            FailureClass::ValidationFailed,
        );
        assert_eq!(
            FailureClass::classify("subtask_mode_disallowed: tool not in allowed list"),
            FailureClass::NotAllowed,
        );
        assert_eq!(
            FailureClass::classify("something unrelated"),
            FailureClass::Other,
        );
    }

    #[test]
    fn failure_memory_renders_structured_block_for_single_error() {
        let context = context_with_failed_tool_call(
            "fs_read_file",
            json!({"path": "/nope.txt"}),
            "No such file: /nope.txt",
        );
        let block = build_failure_memory(&context).expect("error present");
        assert!(block.starts_with("Last error:"));
        assert!(block.contains("tool: fs_read_file"));
        assert!(block.contains("type: file_not_found"));
        assert!(block.contains("args_fp: "));
        assert!(block.contains("message: No such file: /nope.txt"));
        assert!(block.contains("retry: change args before retry"));
        assert!(!block.contains("identical call has failed"));
    }

    #[test]
    fn failure_memory_renders_retryable_verdict_for_timeouts() {
        let context = context_with_failed_tool_call(
            "search_grep",
            json!({"query": "auth"}),
            "operation timed out after 30s",
        );
        let block = build_failure_memory(&context).expect("error present");
        assert!(block.contains("type: timeout"));
        assert!(
            block.contains("retry: safe to retry (likely transient)"),
            "timeouts must be marked retryable; got {block}",
        );
    }

    #[test]
    fn failure_memory_detects_identical_repeat_call() {
        let args = json!({"path": "/missing.txt"});
        let context = vec![
            Message::user("go"),
            Message::assistant_with_tool_calls(
                "try one".to_string(),
                vec![ToolCall {
                    id: "call_a".to_string(),
                    name: "fs_read_file".to_string(),
                    arguments: args.clone(),
                }],
            ),
            Message::tool_result("call_a", json!({"error": "No such file"}), true),
            Message::assistant_with_tool_calls(
                "try again".to_string(),
                vec![ToolCall {
                    id: "call_b".to_string(),
                    name: "fs_read_file".to_string(),
                    arguments: args,
                }],
            ),
            Message::tool_result("call_b", json!({"error": "No such file"}), true),
        ];
        let block = build_failure_memory(&context).expect("error present");
        assert!(
            block.contains("identical call has failed before — do not retry unchanged"),
            "repeat should surface anti-repeat guidance; got {block}",
        );
    }

    #[test]
    fn failure_memory_does_not_flag_repeat_when_args_differ() {
        let context = vec![
            Message::user("go"),
            Message::assistant_with_tool_calls(
                "try one".to_string(),
                vec![ToolCall {
                    id: "call_a".to_string(),
                    name: "fs_read_file".to_string(),
                    arguments: json!({"path": "/one.txt"}),
                }],
            ),
            Message::tool_result("call_a", json!({"error": "No such file"}), true),
            Message::assistant_with_tool_calls(
                "try another".to_string(),
                vec![ToolCall {
                    id: "call_b".to_string(),
                    name: "fs_read_file".to_string(),
                    arguments: json!({"path": "/two.txt"}),
                }],
            ),
            Message::tool_result("call_b", json!({"error": "No such file"}), true),
        ];
        let block = build_failure_memory(&context).expect("error present");
        assert!(
            !block.contains("identical call has failed"),
            "different args must not trigger anti-repeat guidance; got {block}",
        );
    }

    #[test]
    fn failure_memory_is_emitted_as_a_bounded_block_by_packer() {
        let mut context = context_with_failed_tool_call(
            "fs_read_file",
            json!({"path": "/long/path/that/keeps/going/and/going.txt"}),
            "No such file: /long/path/that/keeps/going/and/going.txt",
        );
        let input = ContextPolicyInput {
            force_pack: true,
            model_name: "gpt-4".to_string(),
            ..ContextPolicyInput::default()
        };
        apply_context_policy_v2(&mut context, &input);
        let block = context.iter().find_map(|m| match &m.content {
            MessageContent::Text(text)
                if m.role == MessageRole::System
                    && text.starts_with("ContextPolicyV2 failure memory:") =>
            {
                Some(text.clone())
            }
            _ => None,
        });
        // Tiny forced-pack skips when no "real" block (memory ref / brief)
        // is supplied, so the failure-memory block only lands when
        // packing actually runs. Assert the budget invariant directly
        // on the formatter output instead.
        let formatted = build_failure_memory(&context_with_failed_tool_call(
            "fs_read_file",
            json!({"path": "/long/path/that/keeps/going/and/going.txt"}),
            "No such file: /long/path/that/keeps/going/and/going.txt",
        ))
        .unwrap();
        // 140 tokens ≈ 560 chars. The formatter's fields are all
        // individually short, so the full block must fit the budget.
        assert!(
            formatted.len() <= BUDGET_FAILURE_MEMORY_TOKENS * 4,
            "failure memory must stay within {}-token budget; got {} chars",
            BUDGET_FAILURE_MEMORY_TOKENS,
            formatted.len(),
        );
        // Block may or may not appear in packed context depending on
        // tiny-forced-pack short-circuit semantics — but when it does
        // appear, the prefix must match.
        if let Some(text) = block {
            assert!(text.contains("tool: fs_read_file"));
            assert!(text.contains("type: file_not_found"));
        }
    }

    #[test]
    fn arguments_fingerprint_is_stable_across_calls() {
        let args = json!({"path": "/a.txt", "mode": "read"});
        let a = arguments_fingerprint(&args);
        let b = arguments_fingerprint(&args);
        assert_eq!(a, b);
        assert_eq!(a.len(), 8);
    }

    #[test]
    fn arguments_fingerprint_distinguishes_different_args() {
        let a = arguments_fingerprint(&json!({"path": "/a.txt"}));
        let b = arguments_fingerprint(&json!({"path": "/b.txt"}));
        assert_ne!(a, b);
    }
}
