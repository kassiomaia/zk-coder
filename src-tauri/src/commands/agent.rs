//! Agent Tauri IPC commands.
//!
//! Thin adapter that delegates to the AgentService in the application layer.
//! Handles Tauri-specific concerns like event emission and async spawning.

use std::sync::Arc;

use log::{error, info};
use tauri::Emitter;

use crate::agents::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::agents::session::{AgentSession, PendingAction, SessionId, SessionState};
use crate::agents::store::SessionStore;
use crate::agents::task::{AgentRequest, SubTask};
use crate::agents::types::{Agent, AgentId};
use crate::application::agent::AgentServiceImpl;
use crate::core::agent::services::{
    AgentService, CreateSessionRequest,
};

/// Application state for agent commands.
pub struct AgentState {
    /// Agent service for business logic
    pub service: Arc<AgentServiceImpl>,
}

impl AgentState {
    /// Creates a new agent state.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let session_store = Arc::new(
            SessionStore::new(db_path)
                .map_err(|e| format!("Failed to init session store: {e}"))?,
        );
        let approval_queue = Arc::new(crate::agents::approval::ApprovalQueue::new());
        let tool_registry = Arc::new(
            crate::tools::ToolRegistry::new()
                .map_err(|e| format!("Failed to init tool registry: {e}"))?,
        );
        let runtime = Arc::new(crate::agents::runtime::AgentRuntime::new(
            session_store.clone(),
            approval_queue.clone(),
            tool_registry,
        ));
        let registry = Arc::new(crate::agents::registry::AgentRegistry::new());

        let service = Arc::new(AgentServiceImpl::new(
            registry,
            session_store,
            approval_queue,
            runtime,
        ));

        Ok(Self { service })
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
    let request = CreateSessionRequest {
        agent_id,
        conversation_id,
        thread_id,
        workspace_root,
    };

    let session = state.service.create_session(request)
        .map_err(|e| e.to_string())?;

    // Emit session created event
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionCreated {
            session_id: session.id.0.clone(),
            agent_id: session.agent_id.0.clone(),
            agent_name: session.agent_name.clone(),
        },
    );

    // Convert core session back to agents session for response
    Ok(to_agents_session(session))
}

/// Get session state by ID.
#[tauri::command]
pub async fn get_agent_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<AgentSession, String> {
    let core_session = state.service.get_session(&session_id)
        .map_err(|e| e.to_string())?;
    Ok(to_agents_session(core_session))
}

/// Get all sessions for a conversation.
#[tauri::command]
pub async fn get_agent_sessions_for_conversation(
    conversation_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<AgentSession>, String> {
    let core_sessions = state.service.get_conversation_sessions(&conversation_id)
        .map_err(|e| e.to_string())?;
    Ok(core_sessions.into_iter().map(to_agents_session).collect())
}

/// Cancel an active agent session.
#[tauri::command]
pub async fn cancel_agent_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    state.service.cancel_session(&session_id)
        .map_err(|e| e.to_string())?;

    // Clear pending approvals for this session
    state.service.runtime().rollback_store().clear_session(&session_id);

    // Emit state change event
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionStateChanged {
            session_id,
            state: SessionState::Cancelled {
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
    state.service.delete_session(&session_id)
        .map_err(|e| e.to_string())?;

    // Clear pending approvals
    state.service.runtime().rollback_store().clear_session(&session_id);

    Ok(())
}

/// Update the workspace root for a session.
#[tauri::command]
pub async fn update_agent_session_workspace(
    session_id: String,
    workspace_root: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    state.service.update_session_workspace(&session_id, &workspace_root)
        .map_err(|e| e.to_string())
}

// =========================================================================
// Execution Commands
// =========================================================================

/// Send a request to an agent session for execution.
#[tauri::command]
pub async fn send_agent_request(
    session_id: String,
    content: String,
    state: tauri::State<'_, AgentState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let sid = SessionId(session_id.clone());

    let session = state.service.session_store().get_session(&sid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found: {session_id}"))?;

    let agent = state.service.registry().get(&session.agent_id).await
        .ok_or_else(|| format!("Agent not found: {}", session.agent_id.0))?;

    let request = AgentRequest {
        content,
        attachments: vec![],
        context: None,
    };

    let runtime = state.service.runtime().clone();
    let system_prompt = agent.system_prompt.clone();
    let trust_mode = agent.trust_mode;

    info!("[AgentCommand] Spawning agent execution for session {}", session_id);

    // Spawn async execution - results come back via events
    tokio::spawn(async move {
        let result = runtime
            .execute(&session, request, &system_prompt, trust_mode, &app_handle)
            .await;

        match &result {
            Ok(execution_result) => {
                info!(
                    "[AgentCommand] Agent execution completed for session {}, content length: {}",
                    session_id,
                    execution_result.content.len()
                );
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
    state.service.approve_action(&action_id)
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
    state.service.deny_action(&action_id)
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
) -> Result<Vec<PendingAction>, String> {
    // Get from approval queue directly (not through service)
    Ok(state.service.approval_queue().get_pending_for_session(&session_id).await)
}

// =========================================================================
// Registry Commands
// =========================================================================

/// List all available agents.
#[tauri::command]
pub async fn list_agents(
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<Agent>, String> {
    Ok(state.service.registry().list().await)
}

/// Get a specific agent by ID.
#[tauri::command]
pub async fn get_agent(
    agent_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Agent, String> {
    state.service.registry().get(&AgentId(agent_id.clone())).await
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
    state.service.create_subtask(task_id, title, description, order)
        .map_err(|e| e.to_string())
}

/// Update subtask status.
#[tauri::command]
pub async fn update_subtask_status(
    subtask_id: String,
    status: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    use crate::core::agent::entities::TaskStatus as CoreTaskStatus;

    let task_status: CoreTaskStatus = serde_json::from_str(&status)
        .map_err(|e| format!("Invalid status: {e}"))?;

    state.service.update_subtask_status(subtask_id, task_status)
        .map_err(|e| e.to_string())
}

/// Get all subtasks for a task.
#[tauri::command]
pub async fn get_subtasks(
    task_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<SubTask>, String> {
    state.service.session_store().get_subtasks(&task_id)
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
    let core_sessions = state.service.get_conversation_history(&conversation_id)
        .map_err(|e| e.to_string())?;

    Ok(core_sessions.into_iter().map(to_agents_session).collect())
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
    #[serde(rename = "tasks", skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<crate::agents::task::AgentTask>>,
    #[serde(rename = "toolCalls", skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<crate::agents::store::ToolCallEntry>>,
}

/// Get full session details including runtime data.
#[tauri::command]
pub async fn get_session_details(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<SessionDetailsResponse, String> {
    let sid = SessionId(session_id.clone());

    let session = state.service.session_store().get_session(&sid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found: {session_id}"))?;

    // Fetch runtime data
    let files_touched = state.service.session_store().get_session_files(&sid)
        .ok()
        .map(|files| {
            files.into_iter().map(|f| {
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
            }).collect()
        });

    let terminal_output = state.service.session_store().get_session_terminal_output(&sid)
        .ok()
        .map(|lines| {
            lines.into_iter().map(|l| {
                serde_json::json!({
                    "id": l.id,
                    "type": l.line_type,
                    "content": l.content,
                    "timestamp": l.timestamp,
                })
            }).collect()
        });

    let execution_steps = state.service.session_store().get_session_execution_steps(&sid)
        .ok()
        .map(|steps| {
            steps.into_iter().map(|step| serde_json::to_value(step).map_err(|e| e.to_string()))
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?;

    let status_phrases = state.service.session_store().get_session_status_phrases(&sid).ok();
    let tasks = state.service.session_store().get_session_tasks(&sid).ok();
    let tool_calls = state.service.session_store().get_tool_calls_for_session(&sid).ok();

    Ok(SessionDetailsResponse {
        session,
        files_touched,
        terminal_output,
        execution_steps,
        status_phrases,
        tasks,
        tool_calls,
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
    Ok(state.service.runtime().rollback_store().get_session_snapshots(&session_id))
}

/// Roll back a single file to its pre-agent state.
#[tauri::command]
pub async fn rollback_file(
    session_id: String,
    path: String,
    state: tauri::State<'_, AgentState>,
) -> Result<bool, String> {
    state.service.runtime().rollback_store()
        .rollback_file(&session_id, std::path::Path::new(&path))
        .map_err(|e| e.to_string())
}

/// Roll back all files modified in a session to their pre-agent state.
#[tauri::command]
pub async fn rollback_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<String>, String> {
    let reverted = state.service.runtime().rollback_store()
        .rollback_session(&session_id)
        .map_err(|e| e.to_string())?;

    Ok(reverted.into_iter().map(|p| p.display().to_string()).collect())
}

// =========================================================================
// Helper Functions
// =========================================================================

/// Convert core session to agents session.
fn to_agents_session(core: crate::core::agent::entities::AgentSession) -> AgentSession {
    AgentSession {
        id: SessionId(core.id.0),
        agent_id: AgentId(core.agent_id.0),
        agent_name: core.agent_name,
        conversation_id: core.conversation_id,
        thread_id: core.thread_id,
        workspace_root: core.workspace_root,
        parent_session_id: None,
        state: match core.state {
            crate::core::agent::entities::SessionState::WaitForRequest { last_action_at } => {
                SessionState::WaitForRequest { last_action_at }
            }
            crate::core::agent::entities::SessionState::Processing { .. } => {
                SessionState::Running {
                    current_step: 0,
                    context_usage: 0.0,
                }
            }
            crate::core::agent::entities::SessionState::AwaitingApproval { .. } => {
                SessionState::UserInputRequired {
                    pending_action_ids: vec![],
                }
            }
            crate::core::agent::entities::SessionState::Finished { completed_at, .. } => {
                SessionState::Finished { completed_at }
            }
            crate::core::agent::entities::SessionState::Cancelled { reason, .. } => {
                SessionState::Cancelled { reason }
            }
            crate::core::agent::entities::SessionState::Error { error, .. } => {
                SessionState::Error {
                    message: error,
                    recoverable: false,
                }
            }
        },
        created_at: core.created_at,
        updated_at: core.created_at,
    }
}
