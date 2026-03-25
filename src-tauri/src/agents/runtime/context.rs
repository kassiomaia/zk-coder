//! Context management utilities for agent runtime.
//!
//! Provides context compaction and token estimation for managing
//! LLM context window during agent execution.

use crate::llm::Message;

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
    use crate::llm::{Message, MessageContent, MessageRole};

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
    fn test_context_compaction_above_threshold() {
        // Create a context large enough to trigger compaction (>60% of 128k tokens = ~76800 tokens)
        // At ~30 chars per message (content + overhead), need ~2500+ messages to trigger compaction
        let mut context = vec![Message::system("System prompt")];
        for i in 0..2500 {
            context.push(Message::user(&format!("Message {} with some content", i)));
            context.push(Message::assistant(&format!("Response {} with more content", i)));
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
            context.push(Message::user(format!("User {} message with long content: {}", i, long_content)));
            context.push(Message::assistant(format!("Assistant {} response with long content: {}", i, long_content)));
        }

        let before_len = context.len();
        let result = maybe_compact_context(&mut context);

        // Verify compaction occurred
        assert!(result.is_some(), "Compaction should occur with enough content");
        assert!(context.len() < before_len, "Context should be smaller after compaction");

        // First message should be preserved
        assert_eq!(context[0].as_text(), Some("System"));

        // After compaction, we should have first message + recent messages
        assert!(context.len() <= KEEP_RECENT_MESSAGES + 1);
    }
}
