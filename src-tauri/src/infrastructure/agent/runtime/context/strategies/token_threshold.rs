//! Token-threshold compaction strategy.
//!
//! Reproduces the V1 compaction behaviour exactly: trigger when the
//! estimated token count exceeds the model-specific adaptive
//! threshold, then keep the first message (system prompt), an
//! auto-generated summary of the dropped range, and the most recent
//! [`KEEP_RECENT_MESSAGES`] turns.

use crate::core::agent::context::{CompactionDetails, ContextCompactionStrategy, ContextState};
use crate::core::llm::entities::ChatMessage as Message;

use super::super::budget::{
    adaptive_compression_threshold, estimate_context_tokens, model_context_window,
    KEEP_RECENT_MESSAGES,
};
use super::super::policy::{build_compaction_summary, classify_outcome};

/// Triggers compaction once the estimated token usage crosses the
/// model-specific `adaptive_compression_threshold`. Keeps the first
/// message (system prompt), prepends an auto-generated summary of the
/// dropped range, and retains the most recent
/// [`KEEP_RECENT_MESSAGES`] turns.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokenThresholdStrategy;

impl ContextCompactionStrategy for TokenThresholdStrategy {
    fn should_compact(&self, state: &ContextState<'_>) -> bool {
        let tokens = estimate_context_tokens(state.messages);
        let window = model_context_window(state.model_name);
        let threshold = adaptive_compression_threshold(state.model_name);
        let usage_ratio = tokens as f64 / window as f64;
        usage_ratio >= threshold
    }

    fn compact(&self, context: &mut Vec<Message>) -> Option<CompactionDetails> {
        let before_count = context.len();
        let before_tokens = estimate_context_tokens(context);

        // Secondary guard: too few messages to meaningfully compact.
        if before_count <= KEEP_RECENT_MESSAGES + 1 {
            return None;
        }

        let mut new_context = Vec::new();

        // Always keep the first message (usually the system prompt).
        if let Some(first) = context.first() {
            new_context.push(first.clone());
        }

        if let Some(summary) = build_compaction_summary(context) {
            new_context.push(summary);
        }

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
            needs_reinjection: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::core::llm::entities::MessageRole;

    fn msg(role: MessageRole, text: &str) -> Message {
        match role {
            MessageRole::System => Message::system(text.to_string()),
            MessageRole::User => Message::user(text.to_string()),
            _ => Message::assistant(text.to_string()),
        }
    }

    #[test]
    fn should_compact_is_false_when_usage_is_below_threshold() {
        let strategy = TokenThresholdStrategy;
        let messages = vec![msg(MessageRole::System, "short")];
        let state = ContextState {
            messages: &messages,
            model_name: "gpt-4o",
        };
        assert!(!strategy.should_compact(&state));
    }

    #[test]
    fn compact_is_a_noop_when_message_count_is_too_small() {
        // Even when the trigger fires, the secondary guard inside `compact`
        // declines packing when the context has fewer than
        // `KEEP_RECENT_MESSAGES + 1` messages.
        let strategy = TokenThresholdStrategy;
        let mut context = vec![msg(MessageRole::System, "system")];
        assert!(strategy.compact(&mut context).is_none());
        assert_eq!(context.len(), 1);
    }

    #[test]
    fn compact_keeps_first_message_and_recent_tail_when_triggered() {
        // Seed with many messages so the guard (`before_count >
        // KEEP_RECENT_MESSAGES + 1`) passes and the strategy actually
        // rewrites the vec.
        let strategy = TokenThresholdStrategy;
        let mut context: Vec<Message> = Vec::new();
        context.push(msg(MessageRole::System, "sys"));
        for i in 0..(KEEP_RECENT_MESSAGES + 10) {
            context.push(msg(MessageRole::User, &format!("msg {i}")));
        }

        let before = context.len();
        let details = strategy
            .compact(&mut context)
            .expect("compact should run when guard passes");

        assert!(context.len() < before, "compact should shrink the context");
        // First message stays.
        assert_eq!(context[0].as_text().unwrap_or_default(), "sys");
        // Tail is preserved.
        let last_idx = KEEP_RECENT_MESSAGES + 9;
        let expected_last = format!("msg {last_idx}");
        assert_eq!(
            context.last().and_then(|m| m.as_text()).unwrap_or_default(),
            expected_last,
        );
        assert_eq!(details.before_count, before);
        assert_eq!(details.after_count, context.len());
        assert!(details.needs_reinjection);
    }
}
