//! Chat Tauri IPC commands.
//!
//! Thin adapter layer that delegates to the ChatService.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::application::ChatServiceImpl;
use crate::core::chat::entities::{
    ChatMessage, Conversation, ConversationSettings, MessageFeedback, MessageRole, MessageStatus,
    Thread,
};
use crate::core::chat::services::{ChatService, CreateThreadRequest, SendMessageRequest};
use crate::storage::SqliteChatStore;

/// Application state for chat commands.
pub struct ChatState {
    /// Chat service for business logic
    pub service: Arc<dyn ChatService>,
    /// Direct store access for operations not yet migrated to service
    pub store: Arc<Mutex<SqliteChatStore>>,
}

impl ChatState {
    /// Creates a new chat state with a file-backed database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let store = SqliteChatStore::new(db_path).map_err(|e| e.to_string())?;
        let store_arc = Arc::new(store);
        let service = Arc::new(ChatServiceImpl::new(store_arc)) as Arc<dyn ChatService>;
        Ok(Self {
            service,
            store: Arc::new(Mutex::new(
                SqliteChatStore::new(db_path).map_err(|e| e.to_string())?,
            )),
        })
    }

    /// Creates a new chat state with an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, String> {
        let store = SqliteChatStore::in_memory().map_err(|e| e.to_string())?;
        let store_arc = Arc::new(store);
        let service = Arc::new(ChatServiceImpl::new(store_arc)) as Arc<dyn ChatService>;
        Ok(Self {
            service,
            store: Arc::new(Mutex::new(
                SqliteChatStore::in_memory().map_err(|e| e.to_string())?,
            )),
        })
    }
}

// =========================================================================
// Conversation Commands
// =========================================================================

#[tauri::command]
pub async fn load_conversations(
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<Conversation>, String> {
    state.service.load_all_conversations().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_conversation(
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state.service.create_conversation(&title).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_conversation(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state.service.load_conversation(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_conversation(
    id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state.service.rename_conversation(&id, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_conversation(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    state.service.delete_conversation(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_conversation_settings(
    id: String,
    settings: ConversationSettings,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state
        .service
        .update_conversation_settings(&id, &settings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_conversation_workspace(
    id: String,
    workspace_root: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state
        .service
        .set_conversation_workspace(&id, &workspace_root)
        .map_err(|e| e.to_string())
}

// =========================================================================
// Thread Commands
// =========================================================================

#[tauri::command]
pub async fn load_threads(
    conversation_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<Thread>, String> {
    state
        .service
        .load_threads(&conversation_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_thread(
    conversation_id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Thread, String> {
    let request = CreateThreadRequest {
        conversation_id,
        title,
    };
    state.service.create_thread(request).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_thread(
    id: String,
    resolved: bool,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    state.service.resolve_thread(&id, resolved).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_thread(id: String, state: tauri::State<'_, ChatState>) -> Result<(), String> {
    state.service.delete_thread(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_thread(
    id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Thread, String> {
    state.service.rename_thread(&id, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_thread_agent_session(
    id: String,
    agent_session_id: Option<String>,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    state
        .service
        .set_thread_agent_session(&id, agent_session_id)
        .map_err(|e| e.to_string())
}

/// Response for creating a thread with a message.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadWithMessageResponse {
    pub thread: Thread,
    pub message: ChatMessage,
}

#[tauri::command]
pub async fn create_thread_with_message(
    conversation_id: String,
    thread_title: String,
    content: String,
    role: String,
    state: tauri::State<'_, ChatState>,
) -> Result<ThreadWithMessageResponse, String> {
    let message_role = MessageRole::from_str(&role);
    let (thread, message) = state
        .service
        .create_thread_with_message(&conversation_id, &thread_title, &content, message_role)
        .map_err(|e| e.to_string())?;
    Ok(ThreadWithMessageResponse { thread, message })
}

// =========================================================================
// Message Commands
// =========================================================================

#[tauri::command]
pub async fn load_messages(
    conversation_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<ChatMessage>, String> {
    state
        .service
        .load_messages(&conversation_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_thread_messages(
    thread_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<ChatMessage>, String> {
    state
        .service
        .load_thread_messages(&thread_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_message(
    conversation_id: String,
    content: String,
    role: String,
    thread_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<ChatMessage, String> {
    let request = SendMessageRequest {
        conversation_id,
        thread_id,
        content,
        role: MessageRole::from_str(&role),
    };
    state.service.send_message(request).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_message(id: String, state: tauri::State<'_, ChatState>) -> Result<(), String> {
    state.service.delete_message(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_message_status(
    id: String,
    status: String,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    let parsed_status = match status.as_str() {
        "sending" => MessageStatus::Sending,
        "delivered" => MessageStatus::Delivered,
        "error" => MessageStatus::Error,
        _ => MessageStatus::Sent,
    };
    state
        .service
        .update_message_status(&id, parsed_status)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_message_feedback(
    id: String,
    feedback: Option<String>,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    let parsed_feedback = feedback.and_then(|f| match f.as_str() {
        "like" => Some(MessageFeedback::Like),
        "dislike" => Some(MessageFeedback::Dislike),
        _ => None,
    });
    state
        .service
        .update_message_feedback(&id, parsed_feedback)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_state_creation() {
        let state = ChatState::in_memory();
        assert!(state.is_ok());
    }
}
