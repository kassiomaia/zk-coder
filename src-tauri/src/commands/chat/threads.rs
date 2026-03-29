use crate::core::chat::entities::{ChatMessage, MessageRole, Thread};
use crate::core::chat::services::CreateThreadRequest;

use super::state::ChatState;

/// Load all threads for a conversation.
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

/// Create a new thread in a conversation.
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
    state
        .service
        .create_thread(request)
        .map_err(|e| e.to_string())
}

/// Mark a thread as resolved or unresolved.
#[tauri::command]
pub async fn resolve_thread(
    id: String,
    resolved: bool,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    state
        .service
        .resolve_thread(&id, resolved)
        .map_err(|e| e.to_string())
}

/// Archive or unarchive a thread.
#[tauri::command]
pub async fn archive_thread(
    id: String,
    archived: bool,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    state
        .service
        .archive_thread(&id, archived)
        .map_err(|e| e.to_string())
}

/// Delete a thread by ID.
#[tauri::command]
pub async fn delete_thread(id: String, state: tauri::State<'_, ChatState>) -> Result<(), String> {
    state.service.delete_thread(&id).map_err(|e| e.to_string())
}

/// Rename a thread.
#[tauri::command]
pub async fn rename_thread(
    id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Thread, String> {
    state
        .service
        .rename_thread(&id, &title)
        .map_err(|e| e.to_string())
}

/// Associate an agent session with a thread.
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
