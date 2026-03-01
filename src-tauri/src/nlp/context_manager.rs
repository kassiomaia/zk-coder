//! Context manager for conversation lifecycle.
//!
//! This module provides context management including compression,
//! token estimation, and message history.

use std::sync::Arc;

use chrono::Utc;
use thiserror::Error;

use super::context::{ContextMessage, ContextMetadata, ContextWindow, ConversationContext};
use crate::ai::{ClaudeClient, ClaudeError};
use crate::storage::{ContextStore, StoreError};

/// Maximum tokens before compression
const COMPRESSION_THRESHOLD: usize = 50_000;
/// Maximum context tokens
const MAX_TOKENS: usize = 100_000;
/// Number of recent messages to always keep
const KEEP_RECENT_COUNT: usize = 20;

/// Context manager handling lifecycle and compression.
pub struct ContextManager {
    /// Storage backend
    store: ContextStore,
    /// Claude client for compression
    claude: Arc<ClaudeClient>,
}

impl ContextManager {
    /// Creates a new context manager.
    #[must_use]
    pub fn new(store: ContextStore, claude: Arc<ClaudeClient>) -> Self {
        Self { store, claude }
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
    async fn compress(
        &self,
        mut window: ContextWindow,
    ) -> Result<ContextWindow, ContextError> {
        let message_count = window.messages.len();
        if message_count <= KEEP_RECENT_COUNT {
            return Ok(window);
        }

        // Messages to compress
        let to_compress_count = message_count / 2;
        let to_compress: Vec<_> = window.messages.drain(0..to_compress_count).collect();

        // Generate summary
        let summary = self.generate_summary(&to_compress).await?;
        window.metadata.summary = Some(summary);

        Ok(window)
    }

    async fn generate_summary(
        &self,
        messages: &[ContextMessage],
    ) -> Result<String, ContextError> {
        let content: String = messages
            .iter()
            .map(|m| format!("{}: {}", m.role.as_str(), m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Summarize this conversation concisely, preserving key technical details:\n\n{content}"
        );

        self.claude
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
    /// Store error
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    /// AI error
    #[error("AI error: {0}")]
    Ai(#[from] ClaudeError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> ContextManager {
        let store = ContextStore::in_memory().expect("Failed to create store");
        let claude = Arc::new(ClaudeClient::new("test-key"));
        ContextManager::new(store, claude)
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
}
