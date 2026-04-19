use tauri::Emitter;

use crate::commands::ChatState;
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
    chat_state: tauri::State<'_, ChatState>,
    app_handle: tauri::AppHandle,
) -> Result<crate::core::agent::session::AgentSession, String> {
    // Fetch conversation to get model_id from settings
    let model_id = match chat_state.service.load_conversation(&conversation_id) {
        Ok(conversation) => Some(conversation.settings.model),
        Err(e) => {
            log::warn!(
                "[Agent] Failed to load conversation {conversation_id} for model_id: {e}. Using default.",
            );
            None
        }
    };

    let session = state
        .service
        .create_session(CreateSessionRequest {
            agent_id,
            conversation_id,
            thread_id,
            workspace_root,
            model_id,
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

    Ok(session)
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

/// Update the model ID for a session.
#[tauri::command]
pub async fn update_agent_session_model(
    session_id: String,
    model_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    state
        .service
        .update_session_model(&session_id, &model_id)
        .map_err(|e| e.to_string())
}

/// Send a request to an agent session for execution.
///
/// When the session is parked on a pending clarification, the service
/// auto-routes to [`reply_to_clarification`] so typing into the main
/// chat composer resumes the run. See the agent-clarification-resume
/// plan, Phase 5.
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

/// Deliver a reply to a pending clarification for a paused session.
///
/// Persists the reply to the chat transcript, forwards it to the
/// parked coordinator (live path) or repairs the session to a
/// recoverable error (post-restart path; Phase 6 will replace the
/// repair with a real coordinator rebuild). Emits
/// `AgentEvent::ClarificationReplied`.
#[tauri::command]
pub async fn reply_to_clarification(
    session_id: String,
    content: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state
        .service
        .reply_to_clarification(&session_id, &content, app_handle)
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

/// Continue execution from a paused session.
#[tauri::command]
pub async fn continue_agent_execution(
    session_id: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    state
        .service
        .continue_execution(session_id, app_handle)
        .await
        .map_err(|e| e.to_string())
}

/// Replan and continue execution from a paused session.
#[tauri::command]
pub async fn replan_agent_execution(
    session_id: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    state
        .service
        .replan_and_continue(session_id, app_handle)
        .await
        .map_err(|e| e.to_string())
}

/// Initialize agent recovery on app startup.
///
/// This command should be called when the app starts to:
/// 1. Detect any unterminated sessions from previous runs
/// 2. Move them to Paused state
/// 3. Emit `SessionPaused` events so the frontend can show recovery UI
#[tauri::command]
pub async fn initialize_agent_recovery(
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<usize, String> {
    let recovered = state
        .service
        .recover_unterminated_executions()
        .map_err(|e| e.to_string())?;

    // Emit SessionPaused events for each recovered session
    for session in &recovered {
        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::SessionPaused {
                session_id: session.session_id.clone(),
                message: session.message.clone(),
                paused_at: session.paused_at,
                current_subtask_id: None,
            },
        );
    }

    // agent-clarification-resume plan, Phase 6 — surface any sessions
    // that were parked on a clarification question when the app
    // closed. Emits `ClarificationRestored` / `ClarificationTimedOut`
    // and repairs orphan pause rows.
    if let Err(e) = state
        .service
        .rehydrate_paused_clarifications(&app_handle)
    {
        log::warn!("[AgentRecovery] rehydrate_paused_clarifications failed: {e}");
    }

    Ok(recovered.len())
}
