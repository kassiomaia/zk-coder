//! Context manager for conversation lifecycle.
//!
//! This module provides context management including compression,
//! token estimation, and message history.

use std::sync::Arc;

use chrono::Utc;
use log::warn;
use thiserror::Error;

use crate::application::nlp::gateway::{NlpLlmError, NlpLlmGateway};
use crate::core::nlp::entities::{
    ContextMessage, ContextMetadata, ContextWindow, ConversationContext,
};
use crate::core::nlp::repositories::{ContextWindowRepository, NlpDomainError};

/// Maximum tokens before compression
const COMPRESSION_THRESHOLD: usize = 50_000;
/// Maximum context tokens
const MAX_TOKENS: usize = 100_000;
/// Number of recent messages to always keep
const KEEP_RECENT_COUNT: usize = 20;

/// Context manager handling lifecycle and compression.
pub struct ContextManager {
    /// Storage backend
    store: Arc<dyn ContextWindowRepository>,
    /// LLM gateway for summarization/compression
    gateway: Arc<dyn NlpLlmGateway + Send + Sync>,
}

impl ContextManager {
    /// Creates a new context manager with an abstract NLP LLM gateway.
    #[must_use]
    pub fn new(
        store: Arc<dyn ContextWindowRepository>,
        gateway: Arc<dyn NlpLlmGateway + Send + Sync>,
    ) -> Self {
        Self { store, gateway }
    }

    /// Adds a message to a conversation context.
    ///
    /// # Errors
    ///
    /// Returns an error if storage or compression fails.
    pub async fn add_message(
        &self,
        conversation_id: &str,
        message: ContextMessage,
    ) -> Result<(), ContextError> {
        // Ensure context exists
        if !self.store.exists(conversation_id)? {
            self.store.create(conversation_id)?;
        }

        let mut window = self.store.load(conversation_id)?;
        window.messages.push(message);
        window.updated_at = Utc::now();

        // Check for compression
        if self.estimate_tokens(&window) > COMPRESSION_THRESHOLD {
            window = self.compress(window).await?;
        }

        self.store.save(&window)?;
        Ok(())
    }

    /// Gets context for a conversation.
    ///
    /// # Errors
    ///
    /// Returns an error if loading fails.
    pub async fn get_context(
        &self,
        conversation_id: &str,
    ) -> Result<ConversationContext, ContextError> {
        // Return empty context if not found
        if !self.store.exists(conversation_id)? {
            return Ok(ConversationContext::new());
        }

        let window = self.store.load(conversation_id)?;

        // Get recent messages
        let recent_messages: Vec<ContextMessage> = window
            .messages
            .iter()
            .rev()
            .take(KEEP_RECENT_COUNT)
            .rev()
            .cloned()
            .collect();

        Ok(ConversationContext {
            recent_messages,
            metadata: window.metadata.clone(),
            summary: window.metadata.summary.clone(),
        })
    }

    /// Updates context metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if loading or saving fails.
    pub async fn update_metadata(
        &self,
        conversation_id: &str,
        metadata: ContextMetadata,
    ) -> Result<(), ContextError> {
        if !self.store.exists(conversation_id)? {
            self.store.create(conversation_id)?;
        }

        let mut window = self.store.load(conversation_id)?;
        window.metadata = metadata;
        window.updated_at = Utc::now();
        self.store.save(&window)?;
        Ok(())
    }

    /// Gets the full context window.
    ///
    /// # Errors
    ///
    /// Returns an error if loading fails.
    pub fn get_window(&self, conversation_id: &str) -> Result<ContextWindow, ContextError> {
        self.store.load(conversation_id).map_err(ContextError::from)
    }

    /// Clears a conversation context.
    ///
    /// # Errors
    ///
    /// Returns an error if deletion fails.
    pub fn clear(&self, conversation_id: &str) -> Result<(), ContextError> {
        self.store
            .delete(conversation_id)
            .map_err(ContextError::from)
    }

    /// Estimates token count for a context window.
    fn estimate_tokens(&self, window: &ContextWindow) -> usize {
        window.estimate_tokens()
    }

    /// Compresses a context window by summarizing older messages.
    async fn compress(&self, mut window: ContextWindow) -> Result<ContextWindow, ContextError> {
        let message_count = window.messages.len();
        if message_count <= KEEP_RECENT_COUNT {
            return Ok(window);
        }

        // Messages to compress
        let to_compress_count = message_count / 2;
        let to_compress: Vec<_> = window.messages.drain(0..to_compress_count).collect();

        // Generate summary with safe fallback: if summarization fails, restore messages
        // and continue without compressed summary.
        match self.generate_summary(&to_compress).await {
            Ok(summary) => {
                window.metadata.summary = Some(summary);
            }
            Err(error) => {
                warn!("Context compression summary failed, preserving full context: {error}");
                let mut restored = to_compress;
                restored.extend(window.messages);
                window.messages = restored;
            }
        }

        Ok(window)
    }

    async fn generate_summary(&self, messages: &[ContextMessage]) -> Result<String, ContextError> {
        let content: String = messages
            .iter()
            .map(|m| format!("{}: {}", m.role.as_str(), m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Summarize this conversation concisely, preserving key technical details:\n\n{content}"
        );

        self.gateway
            .complete(
                "You are a conversation summarizer. Provide concise technical summaries.",
                &prompt,
            )
            .await
            .map_err(ContextError::from)
    }
}

/// Errors from context management.
#[derive(Debug, Error)]
pub enum ContextError {
    /// Domain repository error
    #[error("Repository error: {0}")]
    Repository(#[from] NlpDomainError),
    /// AI error
    #[error("AI error: {0}")]
    Ai(#[from] NlpLlmError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::nlp::test_utils::{failing_gateway, fixed_gateway};
    use crate::infrastructure::sqlite::repositories::nlp_context::SqliteContextWindowRepository;

    fn create_test_manager() -> ContextManager {
        let store =
            Arc::new(SqliteContextWindowRepository::in_memory().expect("Failed to create store"))
                as Arc<dyn ContextWindowRepository>;
        ContextManager::new(store, fixed_gateway("summary"))
    }

    fn create_failing_compression_manager() -> ContextManager {
        let store =
            Arc::new(SqliteContextWindowRepository::in_memory().expect("Failed to create store"))
                as Arc<dyn ContextWindowRepository>;
        ContextManager::new(store, failing_gateway("summary unavailable"))
    }

    #[test]
    fn test_estimate_tokens() {
        let manager = create_test_manager();
        let mut window = ContextWindow::new("conv-123");
        window.messages.push(ContextMessage::user("Hello world"));

        let tokens = manager.estimate_tokens(&window);
        assert!(tokens > 0);
    }

    #[tokio::test]
    async fn test_get_context_empty() {
        let manager = create_test_manager();

        let context = manager
            .get_context("nonexistent")
            .await
            .expect("Should return empty context");
        assert!(context.recent_messages.is_empty());
    }

    #[tokio::test]
    async fn test_add_message() {
        let manager = create_test_manager();

        manager
            .add_message("conv-456", ContextMessage::user("Hello"))
            .await
            .expect("Should add message");

        let context = manager
            .get_context("conv-456")
            .await
            .expect("Should get context");
        assert_eq!(context.recent_messages.len(), 1);
        assert_eq!(context.recent_messages[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_update_metadata() {
        let manager = create_test_manager();

        let metadata = ContextMetadata::default();
        manager
            .update_metadata("conv-789", metadata)
            .await
            .expect("Should update metadata");

        let context = manager
            .get_context("conv-789")
            .await
            .expect("Should get context");
        assert!(context.metadata.summary.is_none());
    }

    #[tokio::test]
    async fn test_get_context_returns_last_n_messages() {
        let manager = create_test_manager();

        for idx in 0..25 {
            manager
                .add_message("conv-recent", ContextMessage::user(&format!("msg-{idx}")))
                .await
                .expect("Should add message");
        }

        let context = manager
            .get_context("conv-recent")
            .await
            .expect("Should get context");

        assert_eq!(context.recent_messages.len(), 20);
        assert_eq!(context.recent_messages[0].content, "msg-5");
        assert_eq!(context.recent_messages[19].content, "msg-24");
    }

    #[test]
    fn test_clear_nonexistent_context_is_safe() {
        let manager = create_test_manager();
        let result = manager.clear("missing-conversation");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_compression_failure_preserves_messages() {
        let manager = create_failing_compression_manager();

        let large_message = "x".repeat(10_000);
        for idx in 0..21 {
            manager
                .add_message(
                    "conv-compress-fail",
                    ContextMessage::user(&format!("{idx}-{large_message}")),
                )
                .await
                .expect("Should keep context even when summarization fails");
        }

        let window = manager
            .get_window("conv-compress-fail")
            .expect("Should load context window");
        assert_eq!(window.messages.len(), 21);
        assert!(window.metadata.summary.is_none());
    }
}
