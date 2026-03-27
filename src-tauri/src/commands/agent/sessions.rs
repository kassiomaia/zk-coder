use tauri::Emitter;

use crate::application::agent::conversions::core_session_to_agent_session;
use crate::core::agent::services::{AgentService, CreateSessionRequest};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};

use super::AgentState;

/// Create a new agent session tied to a conversation and thread.
#[tauri::command]
pub async fn create_agent_session(
    agent_id: String,
    conversation_id: String,
    thread_id: String,
    workspace_root: Option<String>,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<crate::core::agent::session::AgentSession, String> {
    let session = state
        .service
        .create_session(CreateSessionRequest {
            agent_id,
            conversation_id,
            thread_id,
            workspace_root,
        })
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionCreated {
            session_id: session.id.0.clone(),
            agent_id: session.agent_id.0.clone(),
            agent_name: session.agent_name.clone(),
        },
    );

    Ok(core_session_to_agent_session(session))
}

/// Get session state by ID.
#[tauri::command]
pub async fn get_agent_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<crate::core::agent::session::AgentSession, String> {
    state
        .service
        .get_session(&session_id)
        .map(core_session_to_agent_session)
        .map_err(|e| e.to_string())
}

/// Get all sessions for a conversation.
#[tauri::command]
pub async fn get_agent_sessions_for_conversation(
    conversation_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<crate::core::agent::session::AgentSession>, String> {
    state
        .service
        .get_conversation_sessions(&conversation_id)
        .map(|sessions| {
            sessions
                .into_iter()
                .map(core_session_to_agent_session)
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Cancel an active agent session.
#[tauri::command]
pub async fn cancel_agent_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state
        .service
        .cancel_session(&session_id)
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionStateChanged {
            session_id,
            state: crate::core::agent::session::SessionState::Cancelled {
                reason: "User cancelled".to_string(),
            },
        },
    );

    Ok(())
}

/// Delete a session and its associated data.
#[tauri::command]
pub async fn delete_agent_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    state
        .service
        .delete_session(&session_id)
        .map_err(|e| e.to_string())
}

/// Update the workspace root for a session.
#[tauri::command]
pub async fn update_agent_session_workspace(
    session_id: String,
    workspace_root: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    state
        .service
        .update_session_workspace(&session_id, &workspace_root)
        .map_err(|e| e.to_string())
}

/// Send a request to an agent session for execution.
#[tauri::command]
pub async fn send_agent_request(
    session_id: String,
    content: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    state
        .service
        .spawn_execution(session_id, content, app_handle)
        .await
        .map_err(|e| e.to_string())
}

/// Retry execution with the latest persisted request for a session.
#[tauri::command]
pub async fn retry_last_agent_request(
    session_id: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    state
        .service
        .retry_last_request(session_id, app_handle)
        .await
        .map_err(|e| e.to_string())
}
