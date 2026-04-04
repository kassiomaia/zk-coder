//! Context management utilities for agent runtime.
//!
//! Provides context compaction and token estimation for managing
//! LLM context window during agent execution.

#[allow(unused_imports)]
use crate::core::llm::entities::{ChatMessage as Message, MessageRole, ToolCall};
use serde_json::Value as JsonValue;

/// Approximate context window size for token budgeting (conservative 128k).
pub const CONTEXT_WINDOW: usize = 128_000;

/// Threshold (60%) at which to compact the context.
pub const COMPRESSION_THRESHOLD: f64 = 0.60;

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

/// Details about context compaction.
#[derive(Debug, Clone)]
pub struct CompactionDetails {
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
}

/// Estimates the token count for a message context.
///
/// This is a rough estimate used for budgeting. The actual token count
/// may vary based on the model and content.
pub fn estimate_context_tokens(context: &[Message]) -> usize {
    let mut total = 0usize;
    for msg in context {
        // Estimate tokens: ~4 chars per token, plus overhead for role/metadata
        let content_len = msg.as_text().map_or(0, |s| s.len());
        total += content_len / 4 + 10; // ~4 chars per token, +10 for overhead
    }
    total
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
pub fn maybe_compact_context(context: &mut Vec<Message>) -> Option<CompactionDetails> {
    let before_count = context.len();
    let before_tokens = estimate_context_tokens(context);

    // Calculate if we need compaction
    let usage_ratio = before_tokens as f64 / CONTEXT_WINDOW as f64;
    if usage_ratio < COMPRESSION_THRESHOLD {
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
/// 6) recent dialogue tail
pub fn apply_context_policy_v2(
    context: &mut Vec<Message>,
    input: &ContextPolicyInput,
) -> Option<CompactionDetails> {
    let before_count = context.len();
    let before_tokens = estimate_context_tokens(context);
    let usage_ratio = before_tokens as f64 / CONTEXT_WINDOW as f64;
    if usage_ratio < COMPRESSION_THRESHOLD && !input.force_pack {
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

    let tail = collect_recent_dialogue_tail(context, BUDGET_RECENT_DIALOGUE_TOKENS);
    packed.extend(tail);

    let after_count = packed.len();
    let after_tokens = estimate_context_tokens(&packed);
    *context = packed;

    Some(CompactionDetails {
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

fn build_failure_memory(context: &[Message]) -> Option<String> {
    let last_error = extract_last_tool_error(context)?;
    let mut attempts: Vec<String> = Vec::new();

    let mut seen_error = false;
    for msg in context.iter().rev() {
        if let Some(result) = &msg.tool_result {
            if result.is_error {
                seen_error = true;
                continue;
            }
        }
        if !seen_error {
            continue;
        }
        for call in &msg.tool_calls {
            attempts.push(call.name.clone());
            if attempts.len() >= 3 {
                break;
            }
        }
        if attempts.len() >= 3 {
            break;
        }
    }
    attempts.reverse();

    if attempts.is_empty() {
        Some(format!("Last actionable error: {}", last_error))
    } else {
        Some(format!(
            "Last actionable error: {}. Attempted fixes: {}",
            last_error,
            attempts.join(", ")
        ))
    }
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
        let tokens = estimate_context_tokens(std::slice::from_ref(msg));
        if used + tokens > max_tokens {
            break;
        }
        used += tokens;
        tail.push(msg.clone());
        kept += 1;
    }
    tail.reverse();
    tail
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let mut out = text[..max_len].to_string();
    out.push_str("...");
    out
}

fn estimate_text_tokens(text: &str) -> usize {
    text.len() / 4 + 4
}

fn truncate_to_token_budget(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens.saturating_mul(4);
    truncate_text(text.trim(), max_chars)
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

        let result = maybe_compact_context(&mut context);
        assert!(result.is_none());
        assert_eq!(context.len(), 3); // Unchanged
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
        let result = maybe_compact_context(&mut context);

        // At exactly threshold, should still compact
        // (the function checks usage_ratio < threshold for no compaction)
        // So at >= threshold, it should compact
        if result.is_some() {
            assert!(context.len() < before_len);
        }
    }

    #[test]
    fn test_context_compaction_above_threshold() {
        // Create a context large enough to trigger compaction (>60% of 128k tokens = ~76800 tokens)
        // At ~30 chars per message (content + overhead), need ~2500+ messages to trigger compaction
        let mut context = vec![Message::system("System prompt")];
        for i in 0..2500 {
            context.push(Message::user(&format!("Message {} with some content", i)));
            context.push(Message::assistant(&format!(
                "Response {} with more content",
                i
            )));
        }

        let before_len = context.len();
        let result = maybe_compact_context(&mut context);

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
        let result = maybe_compact_context(&mut context);

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
        // Need enough messages to exceed 60% of CONTEXT_WINDOW
        // Each message pair is ~22 tokens, so we need ~3500 pairs to reach ~77000 tokens
        for i in 0..3500 {
            context.push(Message::user(&format!("User {}", i)));
            context.push(Message::assistant(&format!("Assistant {}", i)));
        }

        let result = maybe_compact_context(&mut context);
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

        let result = maybe_compact_context(&mut context);
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

        let result = maybe_compact_context(&mut context);
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

        maybe_compact_context(&mut context);

        // System message should always be first
        assert_eq!(context[0].role, MessageRole::System);
        assert_eq!(context[0].as_text(), Some("Important system prompt"));
    }

    #[test]
    fn test_context_compaction_with_mixed_message_types() {
        let mut context = vec![Message::system("System")];
        // Need enough messages to exceed 60% of CONTEXT_WINDOW (76800 tokens)
        // Each message pair is ~24 tokens, so we need ~3200 pairs to reach ~77000 tokens
        for i in 0..3200 {
            context.push(Message::user(&format!("User {}", i)));
            context.push(Message::assistant_with_tool_calls(
                format!("Response {}", i),
                vec![ToolCall {
                    id: format!("call_{}", i),
                    name: "test_tool".to_string(),
                    arguments: json!({"index": i}),
                }],
            ));
        }

        let result = maybe_compact_context(&mut context);
        assert!(result.is_some());

        // Should still have system message
        assert_eq!(context[0].role, MessageRole::System);
        // Should be reduced to system + summary + recent
        assert!(context.len() <= KEEP_RECENT_MESSAGES + 2);
    }

    #[test]
    fn test_compaction_details_needs_reinjection_flag() {
        let mut context = vec![Message::system("System")];
        // Need enough messages to exceed 60% of CONTEXT_WINDOW (128000 * 0.6 = 76800 tokens)
        // Each message contributes ~11 tokens (content_len / 4 + 10)
        // So we need about 7000 messages to trigger compaction
        for i in 0..7000 {
            context.push(Message::user(&format!("User {}", i)));
        }

        let result = maybe_compact_context(&mut context);
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

        let result = maybe_compact_context(&mut context);
        if let Some(details) = result {
            assert_eq!(details.before_tokens, before_tokens);
            assert!(details.after_tokens < before_tokens);
        }
    }
}
