//! Shared test fixtures for NLP module tests.

use std::sync::Arc;

use futures::future::BoxFuture;

use super::context::{ContextMessage, ConversationContext};
use super::llm_gateway::{NlpLlmError, NlpLlmGateway};

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

/// Creates a gateway fixture that always returns the provided response.
#[must_use]
pub fn fixed_gateway(response: impl Into<String>) -> Arc<dyn NlpLlmGateway + Send + Sync> {
    Arc::new(FixedGateway {
        response: response.into(),
    })
}

/// Creates a gateway fixture that always fails with the provided message.
#[must_use]
pub fn failing_gateway(message: impl Into<String>) -> Arc<dyn NlpLlmGateway + Send + Sync> {
    Arc::new(FailingGateway {
        message: message.into(),
    })
}

struct FixedGateway {
    response: String,
}

impl NlpLlmGateway for FixedGateway {
    fn complete<'a>(
        &'a self,
        _system: &'a str,
        _user: &'a str,
    ) -> BoxFuture<'a, Result<String, NlpLlmError>> {
        let response = self.response.clone();
        Box::pin(async move { Ok(response) })
    }
}

struct FailingGateway {
    message: String,
}

impl NlpLlmGateway for FailingGateway {
    fn complete<'a>(
        &'a self,
        _system: &'a str,
        _user: &'a str,
    ) -> BoxFuture<'a, Result<String, NlpLlmError>> {
        let message = self.message.clone();
        Box::pin(async move { Err(NlpLlmError::Store(message)) })
    }
}
