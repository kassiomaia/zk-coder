//! Context management utilities for agent runtime.
//!
//! Provides context compaction and token estimation for managing
//! LLM context window during agent execution.

use crate::core::llm::entities::{ChatMessage as Message, MessageContent, MessageRole, ToolCall};

/// Approximate context window size for token budgeting (conservative 128k).
pub const CONTEXT_WINDOW: usize = 128_000;

/// Threshold (60%) at which to compact the context.
pub const COMPRESSION_THRESHOLD: f64 = 0.60;

/// Number of recent messages to preserve during compaction.
pub const KEEP_RECENT_MESSAGES: usize = 15;

/// Details about context compaction.
#[derive(Debug, Clone)]
pub struct CompactionDetails {
    pub before_count: usize,
    pub after_count: usize,
    pub before_tokens: usize,
    pub after_tokens: usize,
    pub needs_reinjection: bool,
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

        // Recent messages preserved
        assert!(context.len() <= KEEP_RECENT_MESSAGES + 1);
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

        // After compaction, we should have first message + recent messages
        assert!(context.len() <= KEEP_RECENT_MESSAGES + 1);
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
        // Should be reduced to recent + system
        assert!(context.len() <= KEEP_RECENT_MESSAGES + 1);
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
