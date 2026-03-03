//! Chat Tauri IPC commands.
//!
//! This module provides Tauri commands for chat conversation management.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::nlp::chat::{ChatMessage, Conversation, MessageRole, MessageStatus, Thread};
use crate::storage::ChatStore;

/// Application state for chat commands.
pub struct ChatState {
    /// Chat store for persistence
    pub store: Arc<Mutex<ChatStore>>,
}

impl ChatState {
    /// Creates a new chat state.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let store = ChatStore::new(db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
        })
    }

    /// Creates a new chat state with an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn in_memory() -> Result<Self, String> {
        let store = ChatStore::in_memory().map_err(|e| e.to_string())?;
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
        })
    }
}

// =========================================================================
// Conversation Commands
// =========================================================================

/// Loads all conversations.
#[tauri::command]
pub async fn load_conversations(
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<Conversation>, String> {
    let store = state.store.lock().await;
    store
        .load_all_conversations()
        .map_err(|e| e.to_string())
}

/// Creates a new conversation.
#[tauri::command]
pub async fn create_conversation(
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    let store = state.store.lock().await;
    store
        .create_conversation(&title)
        .map_err(|e| e.to_string())
}

/// Loads a single conversation by ID.
#[tauri::command]
pub async fn load_conversation(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    let store = state.store.lock().await;
    store
        .load_conversation(&id)
        .map_err(|e| e.to_string())
}

/// Renames a conversation.
#[tauri::command]
pub async fn rename_conversation(
    id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    let store = state.store.lock().await;
    store
        .update_conversation_title(&id, &title)
        .map_err(|e| e.to_string())
}

/// Deletes a conversation.
#[tauri::command]
pub async fn delete_conversation(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    let store = state.store.lock().await;
    store
        .delete_conversation(&id)
        .map_err(|e| e.to_string())
}

// =========================================================================
// Thread Commands
// =========================================================================

/// Loads all threads for a conversation.
#[tauri::command]
pub async fn load_threads(
    conversation_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<Thread>, String> {
    let store = state.store.lock().await;
    store
        .load_threads(&conversation_id)
        .map_err(|e| e.to_string())
}

/// Creates a new thread in a conversation.
#[tauri::command]
pub async fn create_thread(
    conversation_id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Thread, String> {
    let store = state.store.lock().await;
    store
        .create_thread(&conversation_id, &title)
        .map_err(|e| e.to_string())
}

/// Updates a thread's resolved status.
#[tauri::command]
pub async fn resolve_thread(
    id: String,
    resolved: bool,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    let store = state.store.lock().await;
    store
        .update_thread_resolved(&id, resolved)
        .map_err(|e| e.to_string())
}

/// Deletes a thread.
#[tauri::command]
pub async fn delete_thread(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    let store = state.store.lock().await;
    store
        .delete_thread(&id)
        .map_err(|e| e.to_string())
}

/// Renames a thread.
#[tauri::command]
pub async fn rename_thread(
    id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Thread, String> {
    let store = state.store.lock().await;
    store
        .rename_thread(&id, &title)
        .map_err(|e| e.to_string())
}

/// Creates a thread with its first message atomically.
#[tauri::command]
pub async fn create_thread_with_message(
    conversation_id: String,
    thread_title: String,
    content: String,
    role: String,
    state: tauri::State<'_, ChatState>,
) -> Result<ThreadWithMessageResponse, String> {
    let message_role = match role.as_str() {
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        _ => MessageRole::User,
    };

    let store = state.store.lock().await;
    let (thread, message) = store
        .create_thread_with_message(&conversation_id, &thread_title, &content, message_role)
        .map_err(|e| e.to_string())?;

    Ok(ThreadWithMessageResponse { thread, message })
}

/// Response for creating a thread with a message.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadWithMessageResponse {
    /// The created thread
    pub thread: Thread,
    /// The created message
    pub message: ChatMessage,
}

// =========================================================================
// Message Commands
// =========================================================================

/// Loads all messages for a conversation.
#[tauri::command]
pub async fn load_messages(
    conversation_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<ChatMessage>, String> {
    let store = state.store.lock().await;
    store
        .load_messages(&conversation_id)
        .map_err(|e| e.to_string())
}

/// Loads all messages for a specific thread.
#[tauri::command]
pub async fn load_thread_messages(
    thread_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<ChatMessage>, String> {
    let store = state.store.lock().await;
    store
        .load_thread_messages(&thread_id)
        .map_err(|e| e.to_string())
}

/// Sends a message to a conversation (requires thread_id).
#[tauri::command]
pub async fn send_message(
    conversation_id: String,
    content: String,
    role: String,
    thread_id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<ChatMessage, String> {
    let store = state.store.lock().await;

    // Check if thread is resolved before accepting the message
    let thread = store
        .load_thread(&thread_id)
        .map_err(|e| e.to_string())?;

    if thread.metadata.resolved {
        return Err(format!(
            "Thread '{}' is resolved and cannot accept new messages. Reopen the thread to continue the conversation.",
            thread.title
        ));
    }

    // Create the message
    let message = match role.as_str() {
        "assistant" => ChatMessage::assistant(&conversation_id, &content),
        "system" => ChatMessage {
            role: MessageRole::System,
            ..ChatMessage::user(&conversation_id, &content)
        },
        _ => ChatMessage::user(&conversation_id, &content),
    };

    let message = message.with_thread(&thread_id);

    store
        .add_message(&message)
        .map_err(|e| e.to_string())?;

    Ok(message)
}

/// Deletes a message.
#[tauri::command]
pub async fn delete_message(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    let store = state.store.lock().await;
    store
        .delete_message(&id)
        .map_err(|e| e.to_string())
}

/// Updates a message's status.
#[tauri::command]
pub async fn update_message_status(
    id: String,
    status: String,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    let status = match status.as_str() {
        "sending" => MessageStatus::Sending,
        "delivered" => MessageStatus::Delivered,
        "error" => MessageStatus::Error,
        _ => MessageStatus::Sent,
    };

    let store = state.store.lock().await;
    store
        .update_message_status(&id, status)
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
