//! Provides the implementation of the `ChatService`.
//!
//! This service implements the core `ChatService` trait, offering business logic
//! for chat operations while delegating persistence to the underlying storage layer.

mod operations;
mod repositories;

use std::sync::Arc;

use crate::core::chat::repositories::ChatRepositorySet;

/// The concrete implementation of the `ChatService` trait.
///
/// This service is responsible for orchestrating the business logic between the
/// core domain entities and the storage layer. It enforces business rules, such
/// as validating that a thread is not resolved before adding a message, while
/// delegating persistence operations to a repository adapter.
pub struct ChatServiceImpl {
    /// The underlying repository implementation that handles data persistence.
    repository: Arc<dyn ChatRepositorySet>,
}

impl ChatServiceImpl {
    /// Creates a new `ChatServiceImpl` with the given repository backend.
    #[must_use]
    pub fn new(repository: Arc<dyn ChatRepositorySet>) -> Self {
        Self { repository }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::services::{ChatService, CreateThreadRequest, SendMessageRequest};
    use crate::core::chat::{ConversationSettings, MessageFeedback, MessageRole, MessageStatus};
    use crate::core::ChatDomainError;
    use crate::infrastructure::sqlite::repositories::chat::SqliteChatRepository;

    fn create_test_service() -> ChatServiceImpl {
        let repository =
            Arc::new(SqliteChatRepository::in_memory().expect("Failed to create repository"));
        ChatServiceImpl::new(repository)
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
        let created = ChatService::create_conversation(&service, "Test").expect("Failed to create");
        let loaded = ChatService::load_conversation(&service, &created.id).expect("Failed to load");
        assert_eq!(loaded.id, created.id);
    }

    #[test]
    fn test_create_thread() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread =
            ChatService::create_thread(&service, request).expect("Failed to create thread");
        assert_eq!(thread.conversation_id, conv.id);
        assert_eq!(thread.title, "Test Thread");
    }

    #[test]
    fn test_send_message_to_open_thread() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread =
            ChatService::create_thread(&service, request).expect("Failed to create thread");

        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Hello world".to_string(),
            role: MessageRole::User,
        };

        let message =
            ChatService::send_message(&service, msg_request).expect("Failed to send message");
        assert_eq!(message.content, "Hello world");
        assert_eq!(message.thread_id, Some(thread.id));
    }

    #[test]
    fn test_send_message_to_resolved_thread_fails() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread =
            ChatService::create_thread(&service, request).expect("Failed to create thread");

        // Resolve the thread
        ChatService::resolve_thread(&service, &thread.id, true).expect("Failed to resolve thread");

        // Try to send a message
        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Hello world".to_string(),
            role: MessageRole::User,
        };

        let result = ChatService::send_message(&service, msg_request);
        assert!(matches!(
            result,
            Err(ChatDomainError::ThreadResolved { .. })
        ));
    }

    #[test]
    fn test_create_thread_with_message() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");

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
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");

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
        assert_eq!(
            updated.settings.system_prompt,
            Some("Custom prompt".to_string())
        );
    }

    #[test]
    fn test_rename_thread() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Old Title".to_string(),
        };
        let thread =
            ChatService::create_thread(&service, request).expect("Failed to create thread");

        let renamed = ChatService::rename_thread(&service, &thread.id, "New Title")
            .expect("Failed to rename thread");

        assert_eq!(renamed.title, "New Title");
    }

    #[test]
    fn test_update_message_status() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread =
            ChatService::create_thread(&service, request).expect("Failed to create thread");

        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Hello".to_string(),
            role: MessageRole::User,
        };
        let message =
            ChatService::send_message(&service, msg_request).expect("Failed to send message");

        ChatService::update_message_status(&service, &message.id, MessageStatus::Delivered)
            .expect("Failed to update message status");

        let messages = ChatService::load_thread_messages(&service, &thread.id)
            .expect("Failed to load messages");

        assert_eq!(messages[0].status, MessageStatus::Delivered);
    }

    #[test]
    fn test_update_message_feedback() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread =
            ChatService::create_thread(&service, request).expect("Failed to create thread");

        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Hello".to_string(),
            role: MessageRole::Assistant,
        };
        let message =
            ChatService::send_message(&service, msg_request).expect("Failed to send message");

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
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");

        ChatService::delete_conversation(&service, &conv.id)
            .expect("Failed to delete conversation");

        let result = ChatService::load_conversation(&service, &conv.id);
        assert!(matches!(
            result,
            Err(ChatDomainError::ConversationNotFound { .. })
        ));
    }

    #[test]
    fn test_delete_thread() {
        let service = create_test_service();
        let conv = ChatService::create_conversation(&service, "Test").expect("Failed to create");
        let request = CreateThreadRequest {
            conversation_id: conv.id.clone(),
            title: "Test Thread".to_string(),
        };
        let thread =
            ChatService::create_thread(&service, request).expect("Failed to create thread");

        ChatService::delete_thread(&service, &thread.id).expect("Failed to delete thread");

        let result = ChatService::load_thread(&service, &thread.id);
        assert!(matches!(
            result,
            Err(ChatDomainError::ThreadNotFound { .. })
        ));
    }

    // ========================================================================
    // Additional Comprehensive Tests
    // ========================================================================

    #[test]
    fn test_load_all_conversations_empty() {
        let service = create_test_service();

        let result = ChatService::load_all_conversations(&service);

        assert!(result.is_ok());
        let conversations = result.unwrap();
        assert!(conversations.is_empty());
    }

    #[test]
    fn test_load_all_conversations_multiple() {
        let service = create_test_service();

        ChatService::create_conversation(&service, "First").expect("Failed to create first");
        ChatService::create_conversation(&service, "Second").expect("Failed to create second");

        let result = ChatService::load_all_conversations(&service);

        assert!(result.is_ok());
        let conversations = result.unwrap();
        assert_eq!(conversations.len(), 2);
    }

    #[test]
    fn test_rename_conversation() {
        let service = create_test_service();

        let conv =
            ChatService::create_conversation(&service, "Original Title").expect("Failed to create");

        let renamed = ChatService::rename_conversation(&service, &conv.id, "New Title")
            .expect("Failed to rename");

        assert_eq!(renamed.id, conv.id);
        assert_eq!(renamed.title, "New Title");

        // Verify persistence
        let loaded = ChatService::load_conversation(&service, &conv.id).expect("Failed to load");
        assert_eq!(loaded.title, "New Title");
    }

    #[test]
    fn test_set_conversation_workspace() {
        let service = create_test_service();

        let conv =
            ChatService::create_conversation(&service, "Workspace Test").expect("Failed to create");

        let updated =
            ChatService::set_conversation_workspace(&service, &conv.id, "/test/workspace")
                .expect("Failed to set workspace");

        assert_eq!(
            updated.settings.workspace_root,
            Some("/test/workspace".to_string())
        );
    }

    #[test]
    fn test_load_threads_for_conversation() {
        let service = create_test_service();

        let conv =
            ChatService::create_conversation(&service, "Multi-thread").expect("Failed to create");

        ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread 1".to_string(),
            },
        )
        .expect("Failed to create thread 1");

        ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread 2".to_string(),
            },
        )
        .expect("Failed to create thread 2");

        let threads =
            ChatService::load_threads(&service, &conv.id).expect("Failed to load threads");

        assert_eq!(threads.len(), 2);
    }

    #[test]
    fn test_thread_resolution_toggle() {
        let service = create_test_service();

        let conv = ChatService::create_conversation(&service, "Resolution Test")
            .expect("Failed to create");

        let thread = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Resolvable".to_string(),
            },
        )
        .expect("Failed to create thread");

        // Thread should start unresolved
        assert!(!thread.is_resolved());

        // Resolve the thread
        ChatService::resolve_thread(&service, &thread.id, true).expect("Failed to resolve thread");

        let resolved = ChatService::load_thread(&service, &thread.id).expect("Failed to load");
        assert!(resolved.is_resolved());

        // Reopen the thread
        ChatService::resolve_thread(&service, &thread.id, false).expect("Failed to reopen thread");

        let reopened = ChatService::load_thread(&service, &thread.id).expect("Failed to load");
        assert!(!reopened.is_resolved());
    }

    #[test]
    fn test_set_thread_agent_session() {
        let service = create_test_service();

        let conv = ChatService::create_conversation(&service, "Agent Session Test")
            .expect("Failed to create");

        let thread = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "With Agent".to_string(),
            },
        )
        .expect("Failed to create thread");

        ChatService::set_thread_agent_session(
            &service,
            &thread.id,
            Some("agent-session-123".to_string()),
        )
        .expect("Failed to set agent session");

        let updated = ChatService::load_thread(&service, &thread.id).expect("Failed to load");
        assert_eq!(
            updated.metadata.agent_session_id,
            Some("agent-session-123".to_string())
        );
    }

    #[test]
    fn test_delete_message() {
        let service = create_test_service();

        let conv = ChatService::create_conversation(&service, "Message Deletion")
            .expect("Failed to create");

        let thread = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread".to_string(),
            },
        )
        .expect("Failed to create thread");

        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "To be deleted".to_string(),
            role: MessageRole::User,
        };
        let message =
            ChatService::send_message(&service, msg_request).expect("Failed to send message");

        ChatService::delete_message(&service, &message.id).expect("Failed to delete message");

        let messages = ChatService::load_thread_messages(&service, &thread.id)
            .expect("Failed to load messages");
        assert!(messages.is_empty());
    }

    #[test]
    fn test_messages_with_different_roles() {
        let service = create_test_service();

        let conv =
            ChatService::create_conversation(&service, "Roles Test").expect("Failed to create");

        let thread = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread".to_string(),
            },
        )
        .expect("Failed to create thread");

        // Send user message
        let user_msg = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "User question".to_string(),
            role: MessageRole::User,
        };
        ChatService::send_message(&service, user_msg).expect("Failed to send user message");

        // Send assistant message
        let asst_msg = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Assistant response".to_string(),
            role: MessageRole::Assistant,
        };
        ChatService::send_message(&service, asst_msg).expect("Failed to send assistant message");

        let messages = ChatService::load_thread_messages(&service, &thread.id)
            .expect("Failed to load messages");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_conversation_settings_defaults() {
        let service = create_test_service();

        let conv =
            ChatService::create_conversation(&service, "Defaults Test").expect("Failed to create");

        // Check default values from ConversationSettings::default()
        assert!(!conv.settings.model.is_empty());
        assert_eq!(conv.settings.temperature, 0.7);
        assert_eq!(conv.settings.max_tokens, 4096);
        assert!(conv.settings.system_prompt.is_none());
        assert!(conv.settings.workspace_root.is_none());
    }

    #[test]
    fn test_message_status_transitions() {
        let service = create_test_service();

        let conv =
            ChatService::create_conversation(&service, "Status Test").expect("Failed to create");

        let thread = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread".to_string(),
            },
        )
        .expect("Failed to create thread");

        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Test message".to_string(),
            role: MessageRole::User,
        };
        let message =
            ChatService::send_message(&service, msg_request).expect("Failed to send message");

        // Initial status should be Sent
        assert_eq!(message.status, MessageStatus::Sent);

        // Transition to Delivered
        ChatService::update_message_status(&service, &message.id, MessageStatus::Delivered)
            .expect("Failed to update to Delivered");

        let messages =
            ChatService::load_thread_messages(&service, &thread.id).expect("Failed to load");
        assert_eq!(messages[0].status, MessageStatus::Delivered);
    }

    #[test]
    fn test_message_feedback_like_and_dislike() {
        let service = create_test_service();

        let conv =
            ChatService::create_conversation(&service, "Feedback Test").expect("Failed to create");

        let thread = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread".to_string(),
            },
        )
        .expect("Failed to create thread");

        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Assistant response".to_string(),
            role: MessageRole::Assistant,
        };
        let message =
            ChatService::send_message(&service, msg_request).expect("Failed to send message");

        // Like the message
        ChatService::update_message_feedback(&service, &message.id, Some(MessageFeedback::Like))
            .expect("Failed to set Like");

        let messages =
            ChatService::load_thread_messages(&service, &thread.id).expect("Failed to load");
        assert_eq!(
            messages[0].metadata.as_ref().and_then(|m| m.feedback),
            Some(MessageFeedback::Like)
        );

        // Change to Dislike
        ChatService::update_message_feedback(&service, &message.id, Some(MessageFeedback::Dislike))
            .expect("Failed to set Dislike");

        let messages =
            ChatService::load_thread_messages(&service, &thread.id).expect("Failed to load");
        assert_eq!(
            messages[0].metadata.as_ref().and_then(|m| m.feedback),
            Some(MessageFeedback::Dislike)
        );

        // Clear feedback
        ChatService::update_message_feedback(&service, &message.id, None)
            .expect("Failed to clear feedback");

        let messages =
            ChatService::load_thread_messages(&service, &thread.id).expect("Failed to load");
        assert_eq!(messages[0].metadata.as_ref().and_then(|m| m.feedback), None);
    }

    #[test]
    fn test_send_message_after_reopening_thread() {
        let service = create_test_service();

        let conv =
            ChatService::create_conversation(&service, "Reopen Test").expect("Failed to create");

        let thread = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread".to_string(),
            },
        )
        .expect("Failed to create thread");

        // Resolve the thread
        ChatService::resolve_thread(&service, &thread.id, true).expect("Failed to resolve");

        // Sending should fail
        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Test".to_string(),
            role: MessageRole::User,
        };
        let result = ChatService::send_message(&service, msg_request);
        assert!(matches!(
            result,
            Err(ChatDomainError::ThreadResolved { .. })
        ));

        // Reopen the thread
        ChatService::resolve_thread(&service, &thread.id, false).expect("Failed to reopen");

        // Sending should now succeed
        let msg_request = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread.id.clone(),
            content: "Test".to_string(),
            role: MessageRole::User,
        };
        let result = ChatService::send_message(&service, msg_request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_conversation_not_found() {
        let service = create_test_service();

        let result = ChatService::load_conversation(&service, "nonexistent-id");
        assert!(matches!(
            result,
            Err(ChatDomainError::ConversationNotFound { .. })
        ));
    }

    #[test]
    fn test_load_thread_not_found() {
        let service = create_test_service();

        let result = ChatService::load_thread(&service, "nonexistent-thread");
        assert!(matches!(
            result,
            Err(ChatDomainError::ThreadNotFound { .. })
        ));
    }

    #[test]
    fn test_delete_message_not_found() {
        let service = create_test_service();

        let result = ChatService::delete_message(&service, "nonexistent-message");
        assert!(matches!(
            result,
            Err(ChatDomainError::MessageNotFound { .. })
        ));
    }

    #[test]
    fn test_full_conversation_lifecycle() {
        let service = create_test_service();

        // Create conversation
        let conv = ChatService::create_conversation(&service, "Lifecycle Test")
            .expect("Failed to create conversation");

        // Update settings
        let settings = ConversationSettings {
            model: "custom-model".to_string(),
            temperature: 0.8,
            max_tokens: 2048,
            system_prompt: Some("Custom prompt".to_string()),
            workspace_root: Some("/workspace".to_string()),
        };
        ChatService::update_conversation_settings(&service, &conv.id, &settings)
            .expect("Failed to update settings");

        // Create thread with message
        let (thread, message) = ChatService::create_thread_with_message(
            &service,
            &conv.id,
            "Initial Thread",
            "First message",
            MessageRole::User,
        )
        .expect("Failed to create thread with message");

        // Add more messages
        for i in 1..=3 {
            let msg_request = SendMessageRequest {
                conversation_id: conv.id.clone(),
                thread_id: thread.id.clone(),
                content: format!("Message {}", i),
                role: MessageRole::Assistant,
            };
            ChatService::send_message(&service, msg_request).expect("Failed to send message");
        }

        // Verify all messages
        let messages = ChatService::load_thread_messages(&service, &thread.id)
            .expect("Failed to load messages");
        assert_eq!(messages.len(), 4); // 1 initial + 3 additional

        // Add feedback to assistant message
        ChatService::update_message_feedback(
            &service,
            &messages[1].id,
            Some(MessageFeedback::Like),
        )
        .expect("Failed to add feedback");

        // Verify settings persist
        let reloaded =
            ChatService::load_conversation(&service, &conv.id).expect("Failed to reload");
        assert_eq!(reloaded.settings.model, "custom-model");
        assert_eq!(
            reloaded.settings.workspace_root,
            Some("/workspace".to_string())
        );

        // Delete thread
        ChatService::delete_thread(&service, &thread.id).expect("Failed to delete thread");

        // Delete conversation
        ChatService::delete_conversation(&service, &conv.id)
            .expect("Failed to delete conversation");

        // Verify deletion
        let result = ChatService::load_conversation(&service, &conv.id);
        assert!(matches!(
            result,
            Err(ChatDomainError::ConversationNotFound { .. })
        ));
    }

    #[test]
    fn test_multiple_threads_per_conversation() {
        let service = create_test_service();

        let conv = ChatService::create_conversation(&service, "Multi-thread Test")
            .expect("Failed to create conversation");

        // Create multiple threads
        let thread1 = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread 1".to_string(),
            },
        )
        .expect("Failed to create thread 1");

        let thread2 = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread 2".to_string(),
            },
        )
        .expect("Failed to create thread 2");

        let thread3 = ChatService::create_thread(
            &service,
            CreateThreadRequest {
                conversation_id: conv.id.clone(),
                title: "Thread 3".to_string(),
            },
        )
        .expect("Failed to create thread 3");

        // Add messages to different threads
        let msg1 = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread1.id.clone(),
            content: "Message in thread 1".to_string(),
            role: MessageRole::User,
        };
        ChatService::send_message(&service, msg1).expect("Failed to send to thread 1");

        let msg2 = SendMessageRequest {
            conversation_id: conv.id.clone(),
            thread_id: thread2.id.clone(),
            content: "Message in thread 2".to_string(),
            role: MessageRole::User,
        };
        ChatService::send_message(&service, msg2).expect("Failed to send to thread 2");

        // Verify messages are isolated to threads
        let thread1_msgs = ChatService::load_thread_messages(&service, &thread1.id)
            .expect("Failed to load thread 1 messages");
        assert_eq!(thread1_msgs.len(), 1);

        let thread2_msgs = ChatService::load_thread_messages(&service, &thread2.id)
            .expect("Failed to load thread 2 messages");
        assert_eq!(thread2_msgs.len(), 1);

        let thread3_msgs = ChatService::load_thread_messages(&service, &thread3.id)
            .expect("Failed to load thread 3 messages");
        assert_eq!(thread3_msgs.len(), 0);

        // All threads should be listed
        let threads =
            ChatService::load_threads(&service, &conv.id).expect("Failed to load threads");
        assert_eq!(threads.len(), 3);
    }
}
