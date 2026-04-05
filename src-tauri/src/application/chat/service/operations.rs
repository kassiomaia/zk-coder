use crate::core::chat::entities::{
    ChatDomainError, ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole,
    MessageStatus, Thread,
};
use crate::core::chat::repositories::{
    ChatRepository, MessageRepository, RepositoryResult, ThreadRepository,
};
use crate::core::chat::services::{ChatService, CreateThreadRequest, SendMessageRequest};

use super::ChatServiceImpl;

impl ChatService for ChatServiceImpl {
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

    fn set_conversation_workspace(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<Conversation> {
        ChatRepository::update_conversation_workspace(self, id, workspace_root)
    }

    fn archive_conversation(&self, id: &str, archived: bool) -> RepositoryResult<()> {
        ChatRepository::update_conversation_archived(self, id, archived)
    }

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

    fn archive_thread(&self, id: &str, archived: bool) -> RepositoryResult<()> {
        ThreadRepository::update_thread_archived(self, id, archived)
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
        ThreadRepository::create_thread_with_message(
            self,
            conversation_id,
            thread_title,
            content,
            role,
        )
    }

    fn set_thread_agent_session(
        &self,
        id: &str,
        agent_session_id: Option<String>,
    ) -> RepositoryResult<()> {
        ThreadRepository::set_thread_agent_session(self, id, agent_session_id)
    }

    fn set_yolo_mode(&self, id: &str, yolo_mode: bool) -> RepositoryResult<()> {
        ThreadRepository::update_thread_yolo_mode(self, id, yolo_mode)
    }

    fn send_message(&self, request: SendMessageRequest) -> RepositoryResult<ChatMessage> {
        let thread = ThreadRepository::load_thread(self, &request.thread_id)?;

        if thread.is_resolved() {
            return Err(ChatDomainError::ThreadResolved {
                thread_title: thread.title,
            });
        }

        let message = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: request.conversation_id.clone(),
            thread_id: Some(request.thread_id.clone()),
            role: request.role,
            content: request.content,
            timestamp: chrono::Utc::now(),
            status: MessageStatus::Sent,
            metadata: None,
        };

        MessageRepository::add_message(self, &message)?;

        Ok(message)
    }

    fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        MessageRepository::load_messages(self, conversation_id)
    }

    fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        MessageRepository::load_thread_messages(self, thread_id)
    }

    fn add_message(&self, message: &ChatMessage) -> RepositoryResult<()> {
        MessageRepository::add_message(self, message)
    }

    fn delete_message(&self, id: &str) -> RepositoryResult<()> {
        MessageRepository::delete_message(self, id)
    }

    fn update_message_status(&self, id: &str, status: MessageStatus) -> RepositoryResult<()> {
        MessageRepository::update_message_status(self, id, status)
    }

    fn update_message_feedback(
        &self,
        id: &str,
        feedback: Option<MessageFeedback>,
    ) -> RepositoryResult<()> {
        MessageRepository::update_message_feedback(self, id, feedback)
    }
}
