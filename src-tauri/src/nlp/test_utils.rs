//! Shared test fixtures for NLP module tests.

use super::context::{ContextMessage, ConversationContext};

/// Creates an empty conversation context fixture.
#[must_use]
pub fn empty_context() -> ConversationContext {
    ConversationContext::new()
}

/// Creates a conversation context fixture with user messages.
#[must_use]
pub fn context_with_user_messages(messages: &[&str]) -> ConversationContext {
    let recent_messages = messages.iter().map(|msg| ContextMessage::user(msg)).collect();

    ConversationContext {
        recent_messages,
        ..ConversationContext::new()
    }
}
