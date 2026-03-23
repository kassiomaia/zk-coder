//! Agent Tauri IPC commands.
//!
//! This module provides Tauri commands for agent session management,
//! approval handling, and state queries.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Emitter;

use crate::agents::approval::ApprovalQueue;
use crate::agents::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::agents::registry::AgentRegistry;
use crate::agents::session::{AgentSession, PendingAction, SessionId, SessionState};
use crate::agents::store::SessionStore;
use crate::agents::types::AgentId;

/// Application state for agent commands.
pub struct AgentState {
    pub registry: Arc<AgentRegistry>,
    pub session_store: Arc<SessionStore>,
    pub approval_queue: Arc<ApprovalQueue>,
}

impl AgentState {
    /// Creates a new agent state.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let session_store =
            SessionStore::new(db_path).map_err(|e| format!("Failed to init session store: {e}"))?;
        Ok(Self {
            registry: Arc::new(AgentRegistry::new()),
            session_store: Arc::new(session_store),
            approval_queue: Arc::new(ApprovalQueue::new()),
        })
    }
}

// =========================================================================
// Session Commands
// =========================================================================

/// Create a new agent session tied to a conversation and thread.
#[tauri::command]
pub async fn create_agent_session(
    agent_id: String,
    conversation_id: String,
    thread_id: String,
    workspace_root: Option<String>,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<AgentSession, String> {
    let agent = state
        .registry
        .get(&AgentId(agent_id.clone()))
        .await
        .ok_or_else(|| format!("Agent not found: {agent_id}"))?;

    let session = AgentSession::new(
        agent.id.clone(),
        agent.name.clone(),
        conversation_id,
        thread_id,
        workspace_root.map(PathBuf::from),
    );

    state
        .session_store
        .create_session(&session)
        .map_err(|e| e.to_string())?;

    // Emit session created event
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionCreated {
            session_id: session.id.0.clone(),
            agent_id: agent.id.0.clone(),
            agent_name: agent.name.clone(),
        },
    );

    Ok(session)
}

/// Get session state by ID.
#[tauri::command]
pub async fn get_agent_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<AgentSession, String> {
    state
        .session_store
        .get_session(&SessionId(session_id.clone()))
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found: {session_id}"))
}

/// Get all sessions for a conversation.
#[tauri::command]
pub async fn get_agent_sessions_for_conversation(
    conversation_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<AgentSession>, String> {
    state
        .session_store
        .get_sessions_for_conversation(&conversation_id)
        .map_err(|e| e.to_string())
}

/// Cancel an active agent session.
#[tauri::command]
pub async fn cancel_agent_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let sid = SessionId(session_id.clone());
    let new_state = SessionState::Cancelled {
        reason: "User cancelled".to_string(),
    };

    state
        .session_store
        .update_state(&sid, &new_state)
        .map_err(|e| e.to_string())?;

    // Clear any pending approvals for this session
    state.approval_queue.clear_session(&session_id).await;

    // Emit state change event
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionStateChanged {
            session_id,
            state: new_state,
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
    let sid = SessionId(session_id.clone());

    // Clear pending approvals
    state.approval_queue.clear_session(&session_id).await;

    state
        .session_store
        .delete_session(&sid)
        .map_err(|e| e.to_string())
}

// =========================================================================
// Approval Commands
// =========================================================================

/// Approve a pending action for execution.
#[tauri::command]
pub async fn approve_agent_action(
    session_id: String,
    action_id: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state
        .approval_queue
        .approve(&action_id)
        .await
        .ok_or_else(|| format!("Action not found: {action_id}"))?;

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
    let denied = state.approval_queue.deny(&action_id).await;
    if !denied {
        return Err(format!("Action not found: {action_id}"));
    }

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
) -> Result<Vec<PendingAction>, String> {
    Ok(state
        .approval_queue
        .get_pending_for_session(&session_id)
        .await)
}

// =========================================================================
// Registry Commands
// =========================================================================

/// List all available agents.
#[tauri::command]
pub async fn list_agents(
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<crate::agents::types::Agent>, String> {
    Ok(state.registry.list().await)
}

/// Get a specific agent by ID.
#[tauri::command]
pub async fn get_agent(
    agent_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<crate::agents::types::Agent, String> {
    state
        .registry
        .get(&AgentId(agent_id.clone()))
        .await
        .ok_or_else(|| format!("Agent not found: {agent_id}"))
}
