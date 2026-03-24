//! Agent Tauri IPC commands.
//!
//! This module provides Tauri commands for agent session management,
//! approval handling, and state queries.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use log::{error, info};
use tauri::Emitter;

use crate::agents::approval::ApprovalQueue;
use crate::agents::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::agents::registry::AgentRegistry;
use crate::agents::runtime::AgentRuntime;
use crate::agents::session::{AgentSession, PendingAction, SessionId, SessionState};
use crate::agents::store::SessionStore;
use crate::agents::task::AgentRequest;
use crate::agents::types::AgentId;
use crate::tools::ToolRegistry;

/// Application state for agent commands.
pub struct AgentState {
    pub registry: Arc<AgentRegistry>,
    pub session_store: Arc<SessionStore>,
    pub approval_queue: Arc<ApprovalQueue>,
    pub runtime: Arc<AgentRuntime>,
}

impl AgentState {
    /// Creates a new agent state.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let session_store = Arc::new(
            SessionStore::new(db_path)
                .map_err(|e| format!("Failed to init session store: {e}"))?,
        );
        let approval_queue = Arc::new(ApprovalQueue::new());
        let tool_registry = Arc::new(
            ToolRegistry::new().map_err(|e| format!("Failed to init tool registry: {e}"))?,
        );
        let runtime = Arc::new(AgentRuntime::new(
            session_store.clone(),
            approval_queue.clone(),
            tool_registry,
        ));
        Ok(Self {
            registry: Arc::new(AgentRegistry::new()),
            session_store,
            approval_queue,
            runtime,
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
    info!(
        "[AgentCommand] create_agent_session called with workspace_root: {:?}",
        workspace_root
    );

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

    info!(
        "[AgentCommand] Session created with workspace_root: {:?}",
        session.workspace_root
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

    // Transition to WaitForRequest state so the session is ready to accept requests
    let new_state = SessionState::WaitForRequest {
        last_action_at: Utc::now(),
    };
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionStateChanged {
            session_id: session.id.0.clone(),
            state: new_state,
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

/// Update the workspace root for a session.
#[tauri::command]
pub async fn update_agent_session_workspace(
    session_id: String,
    workspace_root: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    let sid = SessionId(session_id.clone());
    info!(
        "[AgentCommand] Updating workspace for session {} to {}",
        session_id, workspace_root
    );

    state
        .session_store
        .update_workspace(&sid, &workspace_root)
        .map_err(|e| e.to_string())?;

    info!(
        "[AgentCommand] Workspace updated successfully for session {}",
        session_id
    );
    Ok(())
}

// =========================================================================
// Execution Commands
// =========================================================================

/// Send a request to an agent session for execution.
///
/// Spawns the agent runtime asynchronously. Results and progress
/// are delivered via Tauri events (agent:event).
#[tauri::command]
pub async fn send_agent_request(
    session_id: String,
    content: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let sid = SessionId(session_id.clone());

    let session = state
        .session_store
        .get_session(&sid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found: {session_id}"))?;

    info!(
        "[AgentCommand] Session {} loaded: workspace_root={:?}",
        session_id, session.workspace_root
    );

    let agent = state
        .registry
        .get(&session.agent_id)
        .await
        .ok_or_else(|| format!("Agent not found: {}", session.agent_id.0))?;

    let request = AgentRequest {
        content,
        attachments: vec![],
        context: None,
    };

    let runtime = state.runtime.clone();
    let system_prompt = agent.system_prompt.clone();
    let trust_mode = agent.trust_mode;

    info!("[AgentCommand] Spawning agent execution for session {}", session_id);

    // Spawn async execution - results come back via events
    tokio::spawn(async move {
        info!("[AgentCommand] Agent task started for session {}", session_id);
        let result = runtime
            .execute(
                &session,
                request,
                &system_prompt,
                trust_mode,
                &app_handle,
            )
            .await;

        match &result {
            Ok(execution_result) => {
                info!("[AgentCommand] Agent execution completed for session {}, content length: {}",
                    session_id, execution_result.content.len());
            }
            Err(e) => {
                error!("[AgentRuntime] Execution failed for session {}: {}", session_id, e);
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::ExecutionError {
                        session_id,
                        error: e.to_string(),
                    },
                );
            }
        }
    });

    Ok(sid.0)
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

// =========================================================================
// Subtask Commands
// =========================================================================

/// Create a new subtask for decomposition.
#[tauri::command]
pub async fn create_subtask(
    task_id: String,
    title: String,
    description: Option<String>,
    order: i32,
    state: tauri::State<'_, AgentState>,
) -> Result<String, String> {
    let subtask = crate::agents::task::SubTask::new(task_id, title, order);
    let subtask = if let Some(desc) = description {
        subtask.with_description(desc)
    } else {
        subtask
    };

    state
        .session_store
        .create_subtask(&subtask)
        .map_err(|e| e.to_string())?;

    Ok(subtask.id)
}

/// Update subtask status.
#[tauri::command]
pub async fn update_subtask_status(
    subtask_id: String,
    status: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    let task_status: crate::agents::task::TaskStatus = serde_json::from_str(&status)
        .map_err(|e| format!("Invalid status: {e}"))?;

    state
        .session_store
        .update_subtask_status(&subtask_id, &task_status)
        .map_err(|e| e.to_string())
}

/// Get all subtasks for a task.
#[tauri::command]
pub async fn get_subtasks(
    task_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<crate::agents::task::SubTask>, String> {
    state
        .session_store
        .get_subtasks(&task_id)
        .map_err(|e| e.to_string())
}

// =========================================================================
// Session History Commands
// =========================================================================

/// Get completed session history for a conversation.
#[tauri::command]
pub async fn get_conversation_session_history(
    conversation_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<AgentSession>, String> {
    let sessions = state
        .session_store
        .get_sessions_for_conversation(&conversation_id)
        .map_err(|e| e.to_string())?;

    // Filter to only completed sessions (plus those that failed/cancelled for review)
    let history: Vec<AgentSession> = sessions
        .into_iter()
        .filter(|s| {
            matches!(
                s.state,
                SessionState::Finished { .. }
                    | SessionState::Error { .. }
                    | SessionState::Cancelled { .. }
            )
        })
        .collect();

    Ok(history)
}

/// Session details response with runtime data.
#[derive(Debug, serde::Serialize)]
pub struct SessionDetailsResponse {
    #[serde(flatten)]
    pub session: AgentSession,
    #[serde(rename = "filesTouched", skip_serializing_if = "Option::is_none")]
    pub files_touched: Option<Vec<serde_json::Value>>,
    #[serde(rename = "terminalOutput", skip_serializing_if = "Option::is_none")]
    pub terminal_output: Option<Vec<serde_json::Value>>,
    #[serde(rename = "executionSteps", skip_serializing_if = "Option::is_none")]
    pub execution_steps: Option<Vec<serde_json::Value>>,
    #[serde(rename = "statusPhrases", skip_serializing_if = "Option::is_none")]
    pub status_phrases: Option<Vec<String>>,
}

/// Get full session details including runtime data (files, terminal, execution steps, status phrases).
#[tauri::command]
pub async fn get_session_details(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<SessionDetailsResponse, String> {
    let session = state
        .session_store
        .get_session(&SessionId(session_id.clone()))
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found: {session_id}"))?;

    // Fetch runtime data
    let files_touched = state
        .session_store
        .get_session_files(&session.id)
        .ok()
        .map(|files| {
            files
                .into_iter()
                .map(|f| {
                    serde_json::json!({
                        "path": f.file_path,
                        "modified": f.action != "read",
                        "action": match f.action.as_str() {
                            "read" => "read",
                            "write" => "write",
                            "delete" => "delete",
                            _ => "read",
                        },
                        "diff": f.diff,
                    })
                })
                .collect()
        });

    let terminal_output = state
        .session_store
        .get_session_terminal_output(&session.id)
        .ok()
        .map(|lines| {
            lines
                .into_iter()
                .map(|l| {
                    serde_json::json!({
                        "id": l.id,
                        "type": l.line_type,
                        "content": l.content,
                        "timestamp": l.timestamp,
                    })
                })
                .collect()
        });

    // Fetch tasks and extract execution steps
    let execution_steps = state
        .session_store
        .get_session_tasks(&session.id)
        .ok()
        .map(|tasks| {
            let mut all_steps: Vec<serde_json::Value> = Vec::new();
            for task in tasks {
                for step in task.steps {
                    all_steps.push(serde_json::to_value(step).map_err(|e| e.to_string())?);
                }
            }
            Ok::<Vec<serde_json::Value>, String>(all_steps)
        })
        .transpose()?;

    let status_phrases = state
        .session_store
        .get_session_status_phrases(&session.id)
        .ok();

    Ok(SessionDetailsResponse {
        session,
        files_touched,
        terminal_output,
        execution_steps,
        status_phrases,
    })
}

// =========================================================================
// Rollback Commands
// =========================================================================

/// Get all file snapshots tracked for a session.
#[tauri::command]
pub async fn get_session_file_snapshots(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<crate::agents::rollback::FileSnapshot>, String> {
    Ok(state.runtime.rollback_store().get_session_snapshots(&session_id))
}

/// Roll back a single file to its pre-agent state.
#[tauri::command]
pub async fn rollback_file(
    session_id: String,
    path: String,
    state: tauri::State<'_, AgentState>,
) -> Result<bool, String> {
    state
        .runtime
        .rollback_store()
        .rollback_file(&session_id, std::path::Path::new(&path))
        .map_err(|e| e.to_string())
}

/// Roll back all files modified in a session to their pre-agent state.
#[tauri::command]
pub async fn rollback_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<String>, String> {
    let reverted = state
        .runtime
        .rollback_store()
        .rollback_session(&session_id)
        .map_err(|e| e.to_string())?;

    Ok(reverted.into_iter().map(|p| p.display().to_string()).collect())
}
