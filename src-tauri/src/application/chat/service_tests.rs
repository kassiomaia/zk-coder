// Chat service tests.
//
// Comprehensive tests for the ChatServiceImpl covering:
// - Conversation management
// - Thread operations
// - Message handling
// - Workspace association
// - State transitions and error paths

use std::sync::Arc;

use crate::application::chat::ChatServiceImpl;
use crate::core::chat::entities::{
    ChatDomainError, ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole, MessageStatus,
    Thread,
};
use crate::core::chat::repositories::RepositoryResult;
use crate::core::chat::services::{ChatService, CreateThreadRequest, SendMessageRequest};
use crate::infrastructure::sqlite::repositories::chat::SqliteChatRepository;

// =========================================================================
// Test Fixtures
// =========================================================================

/// Creates a test service with an in-memory store.
fn create_test_service() -> ChatServiceImpl {
    let store = Arc::new(
        SqliteChatRepository::in_memory().expect("Failed to create in-memory repository"),
    );
    ChatServiceImpl::new(store)
}

/// Creates a test conversation.
fn create_test_conversation(service: &ChatServiceImpl, title: &str) -> RepositoryResult<Conversation> {
    ChatService::create_conversation(service, title)
}

/// Creates a test thread for a conversation.
fn create_test_thread(
    service: &ChatServiceImpl,
    conversation_id: String,
    title: &str,
) -> RepositoryResult<Thread> {
    let request = CreateThreadRequest {
        conversation_id,
        title: title.to_string(),
    };
    ChatService::create_thread(service, request)
}

/// Sends a test message.
fn send_test_message(
    service: &ChatServiceImpl,
    conversation_id: String,
    thread_id: String,
    content: &str,
    role: MessageRole,
) -> RepositoryResult<ChatMessage> {
    let request = SendMessageRequest {
        conversation_id,
        thread_id,
        content: content.to_string(),
        role,
    };
    ChatService::send_message(service, request)
}

// =========================================================================
// Conversation Management Tests
// =========================================================================

#[test]
fn test_create_conversation_with_title() {
    let service = create_test_service();

    let result = create_test_conversation(&service, "Test Conversation");

    assert!(result.is_ok());
    let conv = result.unwrap();
    assert_eq!(conv.title, "Test Conversation");
    assert!(!conv.id.is_empty());
    assert!(conv.created_at <= chrono::Utc::now());
}

#[test]
fn test_create_conversation_empty_title_is_allowed() {
    let service = create_test_service();

    let result = create_test_conversation(&service, "");

    assert!(result.is_ok());
    let conv = result.unwrap();
    assert_eq!(conv.title, "");
}

#[test]
fn test_load_conversation_success() {
    let service = create_test_service();

    let created = create_test_conversation(&service, "Load Test").expect("Failed to create");
    let loaded = ChatService::load_conversation(&service, &created.id).expect("Failed to load");

    assert_eq!(loaded.id, created.id);
    assert_eq!(loaded.title, "Load Test");
}

#[test]
fn test_load_conversation_not_found() {
    let service = create_test_service();

    let result = ChatService::load_conversation(&service, "nonexistent-id");

    assert!(matches!(result, Err(ChatDomainError::ConversationNotFound { .. })));
    if let Err(ChatDomainError::ConversationNotFound { id }) = result {
        assert_eq!(id, "nonexistent-id");
    }
}

#[test]
fn test_load_all_conversations_initially_empty() {
    let service = create_test_service();

    let result = ChatService::load_all_conversations(&service);

    assert!(result.is_ok());
    let conversations = result.unwrap();
    assert!(conversations.is_empty());
}

#[test]
fn test_load_all_conversations_returns_multiple() {
    let service = create_test_service();

    create_test_conversation(&service, "First").expect("Failed to create first");
    create_test_conversation(&service, "Second").expect("Failed to create second");
    create_test_conversation(&service, "Third").expect("Failed to create third");

    let result = ChatService::load_all_conversations(&service);

    assert!(result.is_ok());
    let conversations = result.unwrap();
    assert_eq!(conversations.len(), 3);
}

#[test]
fn test_rename_conversation_success() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Original Title").expect("Failed to create");
    let renamed = ChatService::rename_conversation(&service, &conv.id, "New Title")
        .expect("Failed to rename");

    assert_eq!(renamed.id, conv.id);
    assert_eq!(renamed.title, "New Title");

    // Verify persistence
    let loaded = ChatService::load_conversation(&service, &conv.id).expect("Failed to reload");
    assert_eq!(loaded.title, "New Title");
}

#[test]
fn test_rename_conversation_not_found() {
    let service = create_test_service();

    let result = ChatService::rename_conversation(&service, "nonexistent-id", "New Title");

    assert!(matches!(result, Err(ChatDomainError::ConversationNotFound { .. })));
}

#[test]
fn test_delete_conversation_success() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "To Delete").expect("Failed to create");

    let result = ChatService::delete_conversation(&service, &conv.id);

    assert!(result.is_ok());

    // Verify deletion
    let result = ChatService::load_conversation(&service, &conv.id);
    assert!(matches!(result, Err(ChatDomainError::ConversationNotFound { .. })));
}

#[test]
fn test_delete_conversation_not_found() {
    let service = create_test_service();

    let result = ChatService::delete_conversation(&service, "nonexistent-id");

    assert!(matches!(result, Err(ChatDomainError::ConversationNotFound { .. })));
}

#[test]
fn test_update_conversation_settings() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Settings Test").expect("Failed to create");

    let settings = ConversationSettings {
        model: "custom-model".to_string(),
        temperature: 0.9,
        max_tokens: 8192,
        system_prompt: Some("Custom prompt".to_string()),
        workspace_root: Some("/workspace".to_string()),
    };

    let updated = ChatService::update_conversation_settings(&service, &conv.id, &settings)
        .expect("Failed to update settings");

    assert_eq!(updated.settings.model, "custom-model");
    assert_eq!(updated.settings.temperature, 0.9);
    assert_eq!(updated.settings.max_tokens, 8192);
    assert_eq!(updated.settings.system_prompt, Some("Custom prompt".to_string()));
    assert_eq!(updated.settings.workspace_root, Some("/workspace".to_string()));
}

#[test]
fn test_update_conversation_settings_partial() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Partial Update").expect("Failed to create");

    let settings = ConversationSettings {
        model: "new-model".to_string(),
        temperature: 0.5,
        max_tokens: 2048,
        system_prompt: None,
        workspace_root: None,
    };

    let updated = ChatService::update_conversation_settings(&service, &conv.id, &settings)
        .expect("Failed to update");

    assert_eq!(updated.settings.model, "new-model");
    assert_eq!(updated.settings.temperature, 0.5);
    assert_eq!(updated.settings.max_tokens, 2048);
}

#[test]
fn test_set_conversation_workspace() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Workspace Test").expect("Failed to create");

    let updated = ChatService::set_conversation_workspace(&service, &conv.id, "/test/workspace")
        .expect("Failed to set workspace");

    assert_eq!(updated.settings.workspace_root, Some("/test/workspace".to_string()));

    // Verify persistence
    let reloaded = ChatService::load_conversation(&service, &conv.id).expect("Failed to reload");
    assert_eq!(reloaded.settings.workspace_root, Some("/test/workspace".to_string()));
}

#[test]
fn test_conversation_settings_defaults() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Defaults").expect("Failed to create");

    // Check default values from ConversationSettings::default()
    assert!(!conv.settings.model.is_empty());
    assert_eq!(conv.settings.temperature, 0.7);
    assert_eq!(conv.settings.max_tokens, 4096);
    assert!(conv.settings.system_prompt.is_none());
    assert!(conv.settings.workspace_root.is_none());
}

// =========================================================================
// Thread Operations Tests
// =========================================================================

#[test]
fn test_create_thread_success() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Thread Test").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "New Thread")
        .expect("Failed to create thread");

    assert_eq!(thread.conversation_id, conv.id);
    assert_eq!(thread.title, "New Thread");
    assert!(!thread.id.is_empty());
    assert!(!thread.is_resolved());
}

#[test]
fn test_create_thread_empty_title() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Empty Title").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id, "")
        .expect("Failed to create thread");

    assert_eq!(thread.title, "");
}

#[test]
fn test_load_thread_success() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Load Thread").expect("Failed to create");
    let created = create_test_thread(&service, conv.id.clone(), "Thread Title")
        .expect("Failed to create thread");

    let loaded = ChatService::load_thread(&service, &created.id).expect("Failed to load thread");

    assert_eq!(loaded.id, created.id);
    assert_eq!(loaded.title, "Thread Title");
}

#[test]
fn test_load_thread_not_found() {
    let service = create_test_service();

    let result = ChatService::load_thread(&service, "nonexistent-thread");

    assert!(matches!(result, Err(ChatDomainError::ThreadNotFound { .. })));
}

#[test]
fn test_load_threads_for_conversation() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Multi-thread").expect("Failed to create");

    create_test_thread(&service, conv.id.clone(), "Thread 1").expect("Failed to create thread 1");
    create_test_thread(&service, conv.id.clone(), "Thread 2").expect("Failed to create thread 2");
    create_test_thread(&service, conv.id.clone(), "Thread 3").expect("Failed to create thread 3");

    let threads = ChatService::load_threads(&service, &conv.id).expect("Failed to load threads");

    assert_eq!(threads.len(), 3);
}

#[test]
fn test_load_threads_empty_for_new_conversation() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "No Threads").expect("Failed to create");

    let threads = ChatService::load_threads(&service, &conv.id).expect("Failed to load");

    assert!(threads.is_empty());
}

#[test]
fn test_resolve_thread_toggle() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Resolve Test").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Resolvable")
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
fn test_rename_thread() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Rename Thread").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Old Title")
        .expect("Failed to create thread");

    let renamed = ChatService::rename_thread(&service, &thread.id, "New Title")
        .expect("Failed to rename thread");

    assert_eq!(renamed.id, thread.id);
    assert_eq!(renamed.title, "New Title");

    // Verify persistence
    let loaded = ChatService::load_thread(&service, &thread.id).expect("Failed to load");
    assert_eq!(loaded.title, "New Title");
}

#[test]
fn test_rename_thread_not_found() {
    let service = create_test_service();

    let result = ChatService::rename_thread(&service, "nonexistent-thread", "New Title");

    assert!(matches!(result, Err(ChatDomainError::ThreadNotFound { .. })));
}

#[test]
fn test_delete_thread() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Delete Thread").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "To Delete")
        .expect("Failed to create thread");

    ChatService::delete_thread(&service, &thread.id).expect("Failed to delete thread");

    // Verify deletion
    let result = ChatService::load_thread(&service, &thread.id);
    assert!(matches!(result, Err(ChatDomainError::ThreadNotFound { .. })));
}

#[test]
fn test_delete_thread_not_found() {
    let service = create_test_service();

    let result = ChatService::delete_thread(&service, "nonexistent-thread");

    assert!(matches!(result, Err(ChatDomainError::ThreadNotFound { .. })));
}

#[test]
fn test_set_thread_agent_session() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Agent Session").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "With Agent")
        .expect("Failed to create thread");

    ChatService::set_thread_agent_session(&service, &thread.id, Some("agent-session-123".to_string()))
        .expect("Failed to set agent session");

    let updated = ChatService::load_thread(&service, &thread.id).expect("Failed to load");
    assert_eq!(updated.metadata.agent_session_id, Some("agent-session-123".to_string()));

    // Clear the agent session
    ChatService::set_thread_agent_session(&service, &thread.id, None)
        .expect("Failed to clear agent session");

    let cleared = ChatService::load_thread(&service, &thread.id).expect("Failed to load");
    assert!(cleared.metadata.agent_session_id.is_none());
}

#[test]
fn test_create_thread_with_message() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Thread With Message").expect("Failed to create");

    let (thread, message) = ChatService::create_thread_with_message(
        &service,
        &conv.id,
        "Thread Title",
        "Hello, World!",
        MessageRole::User,
    )
    .expect("Failed to create thread with message");

    assert_eq!(thread.title, "Thread Title");
    assert_eq!(message.content, "Hello, World!");
    assert_eq!(message.thread_id, Some(thread.id));
    assert_eq!(message.role, MessageRole::User);
    assert_eq!(message.status, MessageStatus::Sent);
}

// =========================================================================
// Message Handling Tests
// =========================================================================

#[test]
fn test_send_message_to_open_thread() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Message Test").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Open Thread")
        .expect("Failed to create thread");

    let message = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "Hello, World!",
        MessageRole::User,
    )
    .expect("Failed to send message");

    assert_eq!(message.content, "Hello, World!");
    assert_eq!(message.thread_id, Some(thread.id));
    assert_eq!(message.role, MessageRole::User);
    assert_eq!(message.status, MessageStatus::Sent);
}

#[test]
fn test_send_message_to_resolved_thread_fails() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Resolved Thread").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Thread")
        .expect("Failed to create thread");

    // Resolve the thread
    ChatService::resolve_thread(&service, &thread.id, true).expect("Failed to resolve");

    // Try to send a message
    let result = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "Should fail",
        MessageRole::User,
    );

    assert!(matches!(result, Err(ChatDomainError::ThreadResolved { .. })));
    if let Err(ChatDomainError::ThreadResolved { thread_title }) = result {
        assert_eq!(thread_title, "Thread");
    }
}

#[test]
fn test_send_message_after_reopening_thread() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Reopen Thread").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Thread")
        .expect("Failed to create thread");

    // Resolve the thread
    ChatService::resolve_thread(&service, &thread.id, true).expect("Failed to resolve");

    // Sending should fail
    let result = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "Should fail",
        MessageRole::User,
    );
    assert!(matches!(result, Err(ChatDomainError::ThreadResolved { .. })));

    // Reopen the thread
    ChatService::resolve_thread(&service, &thread.id, false).expect("Failed to reopen");

    // Sending should now succeed
    let result = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "Should succeed",
        MessageRole::User,
    );
    assert!(result.is_ok());
}

#[test]
fn test_send_messages_with_different_roles() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Roles Test").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Thread")
        .expect("Failed to create thread");

    // Send user message
    let user_msg = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "User question",
        MessageRole::User,
    )
    .expect("Failed to send user message");

    // Send assistant message
    let asst_msg = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "Assistant response",
        MessageRole::Assistant,
    )
    .expect("Failed to send assistant message");

    // Send system message
    let sys_msg = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "System notification",
        MessageRole::System,
    )
    .expect("Failed to send system message");

    assert_eq!(user_msg.role, MessageRole::User);
    assert_eq!(asst_msg.role, MessageRole::Assistant);
    assert_eq!(sys_msg.role, MessageRole::System);
}

#[test]
fn test_load_thread_messages() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Load Messages").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Thread")
        .expect("Failed to create thread");

    send_test_message(&service, conv.id.clone(), thread.id.clone(), "First", MessageRole::User)
        .expect("Failed to send first");
    send_test_message(&service, conv.id.clone(), thread.id.clone(), "Second", MessageRole::Assistant)
        .expect("Failed to send second");
    send_test_message(&service, conv.id.clone(), thread.id.clone(), "Third", MessageRole::User)
        .expect("Failed to send third");

    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].content, "First");
    assert_eq!(messages[1].content, "Second");
    assert_eq!(messages[2].content, "Third");
}

#[test]
fn test_load_thread_messages_empty() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Empty Thread").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Empty")
        .expect("Failed to create thread");

    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");

    assert!(messages.is_empty());
}

#[test]
fn test_load_conversation_messages() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Conv Messages").expect("Failed to create");
    let thread1 = create_test_thread(&service, conv.id.clone(), "Thread 1")
        .expect("Failed to create thread 1");
    let thread2 = create_test_thread(&service, conv.id.clone(), "Thread 2")
        .expect("Failed to create thread 2");

    send_test_message(&service, conv.id.clone(), thread1.id.clone(), "Msg 1", MessageRole::User)
        .expect("Failed to send to thread 1");
    send_test_message(&service, conv.id.clone(), thread2.id.clone(), "Msg 2", MessageRole::User)
        .expect("Failed to send to thread 2");

    let messages = ChatService::load_messages(&service, &conv.id).expect("Failed to load");

    assert_eq!(messages.len(), 2);
}

#[test]
fn test_delete_message() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Delete Message").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Thread")
        .expect("Failed to create thread");

    let message = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "To be deleted",
        MessageRole::User,
    )
    .expect("Failed to send message");

    ChatService::delete_message(&service, &message.id).expect("Failed to delete message");

    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");
    assert!(messages.is_empty());
}

#[test]
fn test_delete_message_not_found() {
    let service = create_test_service();

    let result = ChatService::delete_message(&service, "nonexistent-message");

    assert!(matches!(result, Err(ChatDomainError::MessageNotFound { .. })));
}

#[test]
fn test_update_message_status() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Status Update").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Thread")
        .expect("Failed to create thread");

    let message = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "Test message",
        MessageRole::User,
    )
    .expect("Failed to send message");

    // Initial status should be Sent
    assert_eq!(message.status, MessageStatus::Sent);

    // Update to Delivered
    ChatService::update_message_status(&service, &message.id, MessageStatus::Delivered)
        .expect("Failed to update status");

    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");
    assert_eq!(messages[0].status, MessageStatus::Delivered);

    // Update to Complete
    ChatService::update_message_status(&service, &message.id, MessageStatus::Sent)
        .expect("Failed to update to Complete");

    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");
    assert_eq!(messages[0].status, MessageStatus::Sent);
}

#[test]
fn test_update_message_feedback() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Feedback Test").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Thread")
        .expect("Failed to create thread");

    let message = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "Assistant response",
        MessageRole::Assistant,
    )
    .expect("Failed to send message");

    // Like the message
    ChatService::update_message_feedback(&service, &message.id, Some(MessageFeedback::Like))
        .expect("Failed to set Like");

    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");
    assert_eq!(
        messages[0].metadata.as_ref().and_then(|m| m.feedback),
        Some(MessageFeedback::Like)
    );

    // Change to Dislike
    ChatService::update_message_feedback(&service, &message.id, Some(MessageFeedback::Dislike))
        .expect("Failed to set Dislike");

    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");
    assert_eq!(
        messages[0].metadata.as_ref().and_then(|m| m.feedback),
        Some(MessageFeedback::Dislike)
    );

    // Clear feedback
    ChatService::update_message_feedback(&service, &message.id, None)
        .expect("Failed to clear feedback");

    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");
    assert_eq!(messages[0].metadata.as_ref().and_then(|m| m.feedback), None);
}

#[test]
fn test_message_status_transitions() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Transitions").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Thread")
        .expect("Failed to create thread");

    let message = send_test_message(
        &service,
        conv.id.clone(),
        thread.id.clone(),
        "Test",
        MessageRole::User,
    )
    .expect("Failed to send message");

    // Test all status transitions
    let statuses = vec![
        MessageStatus::Sending,
        MessageStatus::Sent,
        MessageStatus::Delivered,
        MessageStatus::Sent,
    ];

    for status in statuses {
        ChatService::update_message_status(&service, &message.id, status.clone())
            .expect("Failed to update status");

        let messages = ChatService::load_thread_messages(&service, &thread.id)
            .expect("Failed to load");
        assert_eq!(messages[0].status, status);
    }
}

// =========================================================================
// Workspace Association Tests
// =========================================================================

#[test]
fn test_conversation_workspace_persistence() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Workspace Persist").expect("Failed to create");

    // Set workspace
    let updated = ChatService::set_conversation_workspace(&service, &conv.id, "/project/root")
        .expect("Failed to set workspace");

    assert_eq!(updated.settings.workspace_root, Some("/project/root".to_string()));

    // Verify across reloads
    let reloaded = ChatService::load_conversation(&service, &conv.id)
        .expect("Failed to reload");
    assert_eq!(reloaded.settings.workspace_root, Some("/project/root".to_string()));
}

#[test]
fn test_conversation_workspace_update_via_settings() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Workspace Via Settings")
        .expect("Failed to create");

    let settings = ConversationSettings {
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_tokens: 4096,
        system_prompt: None,
        workspace_root: Some("/another/path".to_string()),
    };

    let updated = ChatService::update_conversation_settings(&service, &conv.id, &settings)
        .expect("Failed to update settings");

    assert_eq!(updated.settings.workspace_root, Some("/another/path".to_string()));
}

#[test]
fn test_conversation_workspace_to_none() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Clear Workspace").expect("Failed to create");

    // Set workspace
    ChatService::set_conversation_workspace(&service, &conv.id, "/initial/path")
        .expect("Failed to set initial workspace");

    // Clear it via settings
    let settings = ConversationSettings {
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_tokens: 4096,
        system_prompt: None,
        workspace_root: None,
    };

    let updated = ChatService::update_conversation_settings(&service, &conv.id, &settings)
        .expect("Failed to clear workspace");

    assert!(updated.settings.workspace_root.is_none());
}

// =========================================================================
// Integration Tests
// =========================================================================

#[test]
fn test_full_conversation_lifecycle() {
    let service = create_test_service();

    // Create conversation
    let conv = create_test_conversation(&service, "Full Lifecycle").expect("Failed to create");

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
        send_test_message(
            &service,
            conv.id.clone(),
            thread.id.clone(),
            &format!("Message {}", i),
            MessageRole::Assistant,
        )
        .expect("Failed to send message");
    }

    // Verify all messages
    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");
    assert_eq!(messages.len(), 4); // 1 initial + 3 additional

    // Add feedback
    ChatService::update_message_feedback(&service, &messages[1].id, Some(MessageFeedback::Like))
        .expect("Failed to add feedback");

    // Verify settings persist
    let reloaded = ChatService::load_conversation(&service, &conv.id)
        .expect("Failed to reload");
    assert_eq!(reloaded.settings.model, "custom-model");
    assert_eq!(reloaded.settings.workspace_root, Some("/workspace".to_string()));

    // Delete thread
    ChatService::delete_thread(&service, &thread.id).expect("Failed to delete thread");

    // Delete conversation
    ChatService::delete_conversation(&service, &conv.id).expect("Failed to delete");

    // Verify deletion
    let result = ChatService::load_conversation(&service, &conv.id);
    assert!(matches!(result, Err(ChatDomainError::ConversationNotFound { .. })));
}

#[test]
fn test_multiple_threads_per_conversation() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Multi-thread Conv").expect("Failed to create");

    // Create multiple threads
    let thread1 = create_test_thread(&service, conv.id.clone(), "Thread 1")
        .expect("Failed to create thread 1");
    let thread2 = create_test_thread(&service, conv.id.clone(), "Thread 2")
        .expect("Failed to create thread 2");
    let thread3 = create_test_thread(&service, conv.id.clone(), "Thread 3")
        .expect("Failed to create thread 3");

    // Add messages to different threads
    send_test_message(
        &service,
        conv.id.clone(),
        thread1.id.clone(),
        "Message in thread 1",
        MessageRole::User,
    )
    .expect("Failed to send to thread 1");

    send_test_message(
        &service,
        conv.id.clone(),
        thread2.id.clone(),
        "Message in thread 2",
        MessageRole::User,
    )
    .expect("Failed to send to thread 2");

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
    let threads = ChatService::load_threads(&service, &conv.id).expect("Failed to load threads");
    assert_eq!(threads.len(), 3);
}

#[test]
fn test_conversation_isolation() {
    let service = create_test_service();

    // Create multiple conversations
    let conv1 = create_test_conversation(&service, "Conversation 1").expect("Failed to create");
    let conv2 = create_test_conversation(&service, "Conversation 2").expect("Failed to create");

    // Add threads to each
    let thread1 = create_test_thread(&service, conv1.id.clone(), "Thread 1")
        .expect("Failed to create thread in conv 1");
    let thread2 = create_test_thread(&service, conv2.id.clone(), "Thread 2")
        .expect("Failed to create thread in conv 2");

    // Verify isolation
    let conv1_threads = ChatService::load_threads(&service, &conv1.id)
        .expect("Failed to load conv 1 threads");
    let conv2_threads = ChatService::load_threads(&service, &conv2.id)
        .expect("Failed to load conv 2 threads");

    assert_eq!(conv1_threads.len(), 1);
    assert_eq!(conv2_threads.len(), 1);
    assert_eq!(conv1_threads[0].id, thread1.id);
    assert_eq!(conv2_threads[0].id, thread2.id);
}

#[test]
fn test_message_order_preserved() {
    let service = create_test_service();

    let conv = create_test_conversation(&service, "Order Test").expect("Failed to create");
    let thread = create_test_thread(&service, conv.id.clone(), "Thread")
        .expect("Failed to create thread");

    // Send multiple messages rapidly
    for i in 1..=10 {
        send_test_message(
            &service,
            conv.id.clone(),
            thread.id.clone(),
            &format!("Message {}", i),
            MessageRole::User,
        )
        .expect("Failed to send message");
    }

    let messages = ChatService::load_thread_messages(&service, &thread.id)
        .expect("Failed to load messages");

    assert_eq!(messages.len(), 10);

    // Verify order is preserved
    for (i, msg) in messages.iter().enumerate() {
        assert_eq!(msg.content, format!("Message {}", i + 1));
    }
}
