use crate::core::chat::entities::{Conversation, ConversationSettings};

use super::state::ChatState;

#[tauri::command]
pub async fn load_conversations(
    state: tauri::State<'_, ChatState>,
) -> Result<Vec<Conversation>, String> {
    state
        .service
        .load_all_conversations()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_conversation(
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state
        .service
        .create_conversation(&title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_conversation(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state
        .service
        .load_conversation(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_conversation(
    id: String,
    title: String,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String> {
    state
        .service
        .rename_conversation(&id, &title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_conversation(
    id: String,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    state
        .service
        .delete_conversation(&id)
        .map_err(|e| e.to_string())
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
