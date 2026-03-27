use crate::core::chat::entities::{ChatMessage, MessageFeedback, MessageRole, MessageStatus};
use crate::core::chat::services::SendMessageRequest;

use super::state::ChatState;

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
    state
        .service
        .send_message(request)
        .map_err(|e| e.to_string())
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
