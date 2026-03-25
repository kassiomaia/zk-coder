//! Chat service implementation.
//!
//! This service implements the core ChatService trait, providing business logic
//! for chat operations while delegating persistence to the storage layer.

use std::sync::Arc;

use crate::core::chat::entities::{
    ChatDomainError, ChatMessage, Conversation, ConversationSettings, MessageFeedback,
    MessageRole, MessageStatus, Thread,
};
use crate::core::chat::repositories::{ChatRepository, MessageRepository, RepositoryResult, ThreadRepository};
use crate::core::chat::services::{ChatService, CreateThreadRequest, SendMessageRequest};
use crate::storage::SqliteChatStore;

/// Implementation of the ChatService trait.
///
/// This service orchestrates business logic between the core domain entities
/// and the storage layer. It enforces business rules such as thread resolved
/// validation while delegating persistence operations to the storage layer.
///
/// The service wraps a `SqliteChatStore` which implements the repository traits
/// from the core layer. Error conversion is handled by the storage layer's
/// `From<ChatStoreError> for ChatDomainError` implementation.
pub struct ChatServiceImpl {
    /// The underlying storage implementation
    store: Arc<SqliteChatStore>,
}

impl ChatServiceImpl {
    /// Creates a new chat service with the given storage backend.
    #[must_use]
    pub fn new(store: Arc<SqliteChatStore>) -> Self {
        Self { store }
    }

    /// Gets a reference to the underlying store.
    #[must_use]
    pub fn store(&self) -> &Arc<SqliteChatStore> {
        &self.store
    }
}

// ========================================================================
// ChatRepository Implementation
// ========================================================================

impl ChatRepository for ChatServiceImpl {
    fn create_conversation(&self, title: &str) -> RepositoryResult<Conversation> {
        self.store.create_conversation(title)
    }

    fn load_conversation(&self, id: &str) -> RepositoryResult<Conversation> {
        self.store.load_conversation(id)
    }

    fn load_all_conversations(&self) -> RepositoryResult<Vec<Conversation>> {
        self.store.load_all_conversations()
    }

    fn update_conversation_title(&self, id: &str, title: &str) -> RepositoryResult<Conversation> {
        self.store.update_conversation_title(id, title)
    }

    fn delete_conversation(&self, id: &str) -> RepositoryResult<()> {
        self.store.delete_conversation(id)
    }

    fn update_conversation_settings(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> RepositoryResult<Conversation> {
        self.store.update_conversation_settings(id, settings)
    }

    fn update_conversation_workspace(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<Conversation> {
        self.store.update_conversation_workspace(id, workspace_root)
    }
}

// ========================================================================
// ThreadRepository Implementation
// ========================================================================

impl ThreadRepository for ChatServiceImpl {
    fn create_thread(&self, conversation_id: &str, title: &str) -> RepositoryResult<Thread> {
        self.store.create_thread(conversation_id, title)
    }

    fn load_threads(&self, conversation_id: &str) -> RepositoryResult<Vec<Thread>> {
        self.store.load_threads(conversation_id)
    }

    fn load_thread(&self, id: &str) -> RepositoryResult<Thread> {
        self.store.load_thread(id)
    }

    fn update_thread_resolved(&self, id: &str, resolved: bool) -> RepositoryResult<()> {
        self.store.update_thread_resolved(id, resolved)
    }

    fn set_thread_agent_session(&self, id: &str, agent_session_id: Option<String>) -> RepositoryResult<()> {
        self.store.set_thread_agent_session(id, agent_session_id)
    }

    fn delete_thread(&self, id: &str) -> RepositoryResult<()> {
        self.store.delete_thread(id)
    }

    fn rename_thread(&self, id: &str, title: &str) -> RepositoryResult<Thread> {
        self.store.rename_thread(id, title)
    }

    fn create_thread_with_message(
        &self,
        conversation_id: &str,
        thread_title: &str,
        message_content: &str,
        message_role: MessageRole,
    ) -> RepositoryResult<(Thread, ChatMessage)> {
        self.store.create_thread_with_message(conversation_id, thread_title, message_content, message_role)
    }
}

// ========================================================================
// MessageRepository Implementation
// ========================================================================

impl MessageRepository for ChatServiceImpl {
    fn add_message(&self, message: &ChatMessage) -> RepositoryResult<()> {
        self.store.add_message(message)
    }

    fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        self.store.load_messages(conversation_id)
    }

    fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        self.store.load_thread_messages(thread_id)
    }

    fn delete_message(&self, id: &str) -> RepositoryResult<()> {
        self.store.delete_message(id)
    }

    fn update_message_status(&self, id: &str, status: MessageStatus) -> RepositoryResult<()> {
        self.store.update_message_status(id, status)
    }

    fn update_message_feedback(&self, id: &str, feedback: Option<MessageFeedback>) -> RepositoryResult<()> {
        self.store.update_message_feedback(id, feedback)
    }
}

// ========================================================================
// ChatService Implementation
// ========================================================================

impl ChatService for ChatServiceImpl {
    // ------------------------------------------------------------------------
    // Conversation Operations (delegated to ChatRepository)
    // ------------------------------------------------------------------------

    fn create_conversation(&self, title: &str) -> RepositoryResult<Conversation> {
        ChatRepository::create_conversation(self, title)
    }

    fn load_conversation(&self, id: &str) -> RepositoryResult<Conversation> {
        ChatRepository::load_conversation(self, id)
    }

    fn load_all_conversations(&self) -> RepositoryResult<Vec<Conversation>> {
        ChatRepository::load_all_conversations(self)
    }

    fn rename_conversation(&self, id: &str, title: &str) -> RepositoryResult<Conversation> {
        ChatRepository::update_conversation_title(self, id, title)
    }

    fn delete_conversation(&self, id: &str) -> RepositoryResult<()> {
        ChatRepository::delete_conversation(self, id)
    }

    fn update_conversation_settings(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> RepositoryResult<Conversation> {
        ChatRepository::update_conversation_settings(self, id, settings)
    }

    fn set_conversation_workspace(&self, id: &str, workspace_root: &str) -> RepositoryResult<Conversation> {
        ChatRepository::update_conversation_workspace(self, id, workspace_root)
    }

    // ------------------------------------------------------------------------
    // Thread Operations (delegated to ThreadRepository)
    // ------------------------------------------------------------------------

    fn create_thread(&self, request: CreateThreadRequest) -> RepositoryResult<Thread> {
        ThreadRepository::create_thread(self, &request.conversation_id, &request.title)
    }

    fn load_threads(&self, conversation_id: &str) -> RepositoryResult<Vec<Thread>> {
        ThreadRepository::load_threads(self, conversation_id)
    }

    fn load_thread(&self, id: &str) -> RepositoryResult<Thread> {
        ThreadRepository::load_thread(self, id)
    }

    fn resolve_thread(&self, id: &str, resolved: bool) -> RepositoryResult<()> {
        ThreadRepository::update_thread_resolved(self, id, resolved)
    }

    fn delete_thread(&self, id: &str) -> RepositoryResult<()> {
        ThreadRepository::delete_thread(self, id)
    }

    fn rename_thread(&self, id: &str, title: &str) -> RepositoryResult<Thread> {
        ThreadRepository::rename_thread(self, id, title)
    }

    fn create_thread_with_message(
        &self,
        conversation_id: &str,
        thread_title: &str,
        content: &str,
        role: MessageRole,
    ) -> RepositoryResult<(Thread, ChatMessage)> {
        ThreadRepository::create_thread_with_message(self, conversation_id, thread_title, content, role)
    }

    fn set_thread_agent_session(&self, id: &str, agent_session_id: Option<String>) -> RepositoryResult<()> {
        ThreadRepository::set_thread_agent_session(self, id, agent_session_id)
    }

    // ------------------------------------------------------------------------
    // Message Operations (with business logic)
    // ------------------------------------------------------------------------

    fn send_message(&self, request: SendMessageRequest) -> RepositoryResult<ChatMessage> {
        // Business rule: Check if thread is resolved before accepting new messages
        let thread = ThreadRepository::load_thread(self, &request.thread_id)?;

        if thread.is_resolved() {
            return Err(ChatDomainError::ThreadResolved {
                thread_title: thread.title.clone(),
            });
        }

        // Create the message
        let message = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: request.conversation_id.clone(),
            thread_id: Some(request.thread_id.clone()),
            role: request.role,
            content: request.content.clone(),
            timestamp: chrono::Utc::now(),
            status: MessageStatus::Sent,
            metadata: None,
        };

        // Add to storage
        MessageRepository::add_message(self, &message)?;

        Ok(message)
    }

    fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        MessageRepository::load_messages(self, conversation_id)
    }

    fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        MessageRepository::load_thread_messages(self, thread_id)
    }

    fn delete_message(&self, id: &str) -> RepositoryResult<()> {
        MessageRepository::delete_message(self, id)
    }

    fn update_message_status(&self, id: &str, status: MessageStatus) -> RepositoryResult<()> {
        MessageRepository::update_message_status(self, id, status)
    }

    fn update_message_feedback(&self, id: &str, feedback: Option<MessageFeedback>) -> RepositoryResult<()> {
        MessageRepository::update_message_feedback(self, id, feedback)
    }
}

// Since ChatServiceImpl implements all three repository traits, the blanket
// implementation from core/chat/repositories.rs will automatically implement
// ChatStore for it, so we don't need an explicit impl here.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::services::ChatService;

    fn create_test_service() -> ChatServiceImpl {
        let store = Arc::new(SqliteChatStore::in_memory().expect("Failed to create store"));
        ChatServiceImpl::new(store)
    }

    #[test]
    fn test_create_conversation() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test Conversation")
            .expect("Failed to create");
        assert_eq!(conv.title, "Test Conversation");
    }

    #[test]
    fn test_load_conversation() {
        let service = create_test_service();
        let created = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");
        let loaded = ChatService::load_conversation(&service, &created.id)
            .expect("Failed to load");
        assert_eq!(loaded.id, created.id);
    }

    #[test]
    fn test_create_thread() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread = ChatService::create_thread(&service, request)
            .expect("Failed to create thread");
        assert_eq!(thread.conversation_id, conv.id);
        assert_eq!(thread.title, "Test Thread");
    }

    #[test]
    fn test_send_message_to_open_thread() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread = ChatService::create_thread(&service, request)
            .expect("Failed to create thread");

        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Hello world".to_string(),
            role: MessageRole::User,
        };

        let message = ChatService::send_message(&service, msg_request)
            .expect("Failed to send message");
        assert_eq!(message.content, "Hello world");
        assert_eq!(message.thread_id, Some(thread.id));
    }

    #[test]
    fn test_send_message_to_resolved_thread_fails() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread = ChatService::create_thread(&service, request)
            .expect("Failed to create thread");

        // Resolve the thread
        ChatService::resolve_thread(&service, &thread.id, true)
            .expect("Failed to resolve thread");

        // Try to send a message
        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Hello world".to_string(),
            role: MessageRole::User,
        };

        let result = ChatService::send_message(&service, msg_request);
        assert!(matches!(result, Err(ChatDomainError::ThreadResolved { .. })));
    }

    #[test]
    fn test_create_thread_with_message() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");

        let (thread, message) = ChatService::create_thread_with_message(
            &service,
            &conv.id,
            "Thread Title",
            "Hello",
            MessageRole::User,
        )
        .expect("Failed to create thread with message");

        assert_eq!(thread.title, "Thread Title");
        assert_eq!(message.content, "Hello");
        assert_eq!(message.thread_id, Some(thread.id));
    }

    #[test]
    fn test_update_conversation_settings() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");

        let settings = ConversationSettings {
            model: "custom-model".to_string(),
            temperature: 0.9,
            max_tokens: 8192,
            system_prompt: Some("Custom prompt".to_string()),
            workspace_root: None,
        };

        let updated = ChatService::update_conversation_settings(&service, &conv.id, &settings)
            .expect("Failed to update settings");

        assert_eq!(updated.settings.model, "custom-model");
        assert_eq!(updated.settings.temperature, 0.9);
        assert_eq!(updated.settings.system_prompt, Some("Custom prompt".to_string()));
    }

    #[test]
    fn test_rename_thread() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Old Title".to_string(),
        };
        let thread = ChatService::create_thread(&service, request)
            .expect("Failed to create thread");

        let renamed = ChatService::rename_thread(&service, &thread.id, "New Title")
            .expect("Failed to rename thread");

        assert_eq!(renamed.title, "New Title");
    }

    #[test]
    fn test_update_message_status() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread = ChatService::create_thread(&service, request)
            .expect("Failed to create thread");

        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Hello".to_string(),
            role: MessageRole::User,
        };
        let message = ChatService::send_message(&service, msg_request)
            .expect("Failed to send message");

        ChatService::update_message_status(&service, &message.id, MessageStatus::Delivered)
            .expect("Failed to update message status");

        let messages = ChatService::load_thread_messages(&service, &thread.id)
            .expect("Failed to load messages");

        assert_eq!(messages[0].status, MessageStatus::Delivered);
    }

    #[test]
    fn test_update_message_feedback() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread = ChatService::create_thread(&service, request)
            .expect("Failed to create thread");

        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Hello".to_string(),
            role: MessageRole::Assistant,
        };
        let message = ChatService::send_message(&service, msg_request)
            .expect("Failed to send message");

        ChatService::update_message_feedback(&service, &message.id, Some(MessageFeedback::Like))
            .expect("Failed to update message feedback");

        let messages = ChatService::load_thread_messages(&service, &thread.id)
            .expect("Failed to load messages");

        assert_eq!(
            messages[0].metadata.as_ref().and_then(|m| m.feedback),
            Some(MessageFeedback::Like)
        );
    }

    #[test]
    fn test_delete_conversation() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");

        ChatService::delete_conversation(&service, &conv.id)
            .expect("Failed to delete conversation");

        let result = ChatService::load_conversation(&service, &conv.id);
        assert!(matches!(result, Err(ChatDomainError::ConversationNotFound { .. })));
    }

    #[test]
    fn test_delete_thread() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test")
            .expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread = ChatService::create_thread(&service, request)
            .expect("Failed to create thread");

        ChatService::delete_thread(&service, &thread.id)
            .expect("Failed to delete thread");

        let result = ChatService::load_thread(&service, &thread.id);
        assert!(matches!(result, Err(ChatDomainError::ThreadNotFound { .. })));
    }
}
