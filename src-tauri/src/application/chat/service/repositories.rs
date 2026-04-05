use crate::core::chat::entities::{
    ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole, MessageStatus,
    Thread,
};
use crate::core::chat::repositories::{
    ChatRepository, MessageRepository, RepositoryResult, ThreadRepository,
};

use super::ChatServiceImpl;

impl ChatRepository for ChatServiceImpl {
    fn create_conversation(&self, title: &str) -> RepositoryResult<Conversation> {
        self.repository.create_conversation(title)
    }

    fn load_conversation(&self, id: &str) -> RepositoryResult<Conversation> {
        self.repository.load_conversation(id)
    }

    fn load_all_conversations(&self) -> RepositoryResult<Vec<Conversation>> {
        self.repository.load_all_conversations()
    }

    fn update_conversation_title(&self, id: &str, title: &str) -> RepositoryResult<Conversation> {
        self.repository.update_conversation_title(id, title)
    }

    fn delete_conversation(&self, id: &str) -> RepositoryResult<()> {
        self.repository.delete_conversation(id)
    }

    fn update_conversation_settings(
        &self,
        id: &str,
        settings: &ConversationSettings,
    ) -> RepositoryResult<Conversation> {
        self.repository.update_conversation_settings(id, settings)
    }

    fn update_conversation_workspace(
        &self,
        id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<Conversation> {
        self.repository
            .update_conversation_workspace(id, workspace_root)
    }

    fn update_conversation_archived(&self, id: &str, archived: bool) -> RepositoryResult<()> {
        self.repository.update_conversation_archived(id, archived)
    }
}

impl ThreadRepository for ChatServiceImpl {
    fn create_thread(&self, conversation_id: &str, title: &str) -> RepositoryResult<Thread> {
        self.repository.create_thread(conversation_id, title)
    }

    fn load_threads(&self, conversation_id: &str) -> RepositoryResult<Vec<Thread>> {
        self.repository.load_threads(conversation_id)
    }

    fn load_thread(&self, id: &str) -> RepositoryResult<Thread> {
        self.repository.load_thread(id)
    }

    fn update_thread_resolved(&self, id: &str, resolved: bool) -> RepositoryResult<()> {
        self.repository.update_thread_resolved(id, resolved)
    }

    fn update_thread_archived(&self, id: &str, archived: bool) -> RepositoryResult<()> {
        self.repository.update_thread_archived(id, archived)
    }

    fn set_thread_agent_session(
        &self,
        id: &str,
        agent_session_id: Option<String>,
    ) -> RepositoryResult<()> {
        self.repository
            .set_thread_agent_session(id, agent_session_id)
    }

    fn update_thread_yolo_mode(&self, id: &str, yolo_mode: bool) -> RepositoryResult<()> {
        self.repository.update_thread_yolo_mode(id, yolo_mode)
    }

    fn delete_thread(&self, id: &str) -> RepositoryResult<()> {
        self.repository.delete_thread(id)
    }

    fn rename_thread(&self, id: &str, title: &str) -> RepositoryResult<Thread> {
        self.repository.rename_thread(id, title)
    }

    fn create_thread_with_message(
        &self,
        conversation_id: &str,
        thread_title: &str,
        message_content: &str,
        message_role: MessageRole,
    ) -> RepositoryResult<(Thread, ChatMessage)> {
        self.repository.create_thread_with_message(
            conversation_id,
            thread_title,
            message_content,
            message_role,
        )
    }
}

impl MessageRepository for ChatServiceImpl {
    fn add_message(&self, message: &ChatMessage) -> RepositoryResult<()> {
        self.repository.add_message(message)
    }

    fn load_messages(&self, conversation_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        self.repository.load_messages(conversation_id)
    }

    fn load_thread_messages(&self, thread_id: &str) -> RepositoryResult<Vec<ChatMessage>> {
        self.repository.load_thread_messages(thread_id)
    }

    fn delete_message(&self, id: &str) -> RepositoryResult<()> {
        self.repository.delete_message(id)
    }

    fn update_message_status(&self, id: &str, status: MessageStatus) -> RepositoryResult<()> {
        self.repository.update_message_status(id, status)
    }

    fn update_message_feedback(
        &self,
        id: &str,
        feedback: Option<MessageFeedback>,
    ) -> RepositoryResult<()> {
        self.repository.update_message_feedback(id, feedback)
    }
}
