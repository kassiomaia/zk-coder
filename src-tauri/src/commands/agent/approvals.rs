use tauri::Emitter;

use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};

use super::AgentState;

/// Approve a pending action for execution.
#[tauri::command]
pub async fn approve_agent_action(
    session_id: String,
    action_id: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state
        .service
        .approve_action_async(&action_id)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ApprovalApproved {
            session_id,
            action_id,
        },
    );

    Ok(())
}

/// Deny a pending action.
#[tauri::command]
pub async fn deny_agent_action(
    session_id: String,
    action_id: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state
        .service
        .deny_action_async(&action_id)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ApprovalDenied {
            session_id,
            action_id,
        },
    );

    Ok(())
}

/// Get all pending actions for a session.
#[tauri::command]
pub async fn get_pending_agent_actions(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<crate::core::agent::session::PendingAction>, String> {
    Ok(state
        .service
        .pending_actions_view(&session_id)
        .await
        .actions)
}
