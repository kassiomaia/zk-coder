//! Permission checking and approval flow handling.

use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::{AgentSession, PendingAction};
use crate::core::agent::types::ActionPreview;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::approval::{ApprovalEvent, ApprovalQueue};
use crate::infrastructure::agent::events::AgentEvent;
use crate::infrastructure::agent::events::AGENT_EVENT_NAME;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::runtime::types::AgentRuntimeError;
use crate::infrastructure::agent::runtime::infer_action_type;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;
use crate::infrastructure::tools::ToolRegistry;
use log::{info, warn};

/// Timeout for waiting on user approval (seconds).
const APPROVAL_TIMEOUT_SECS: u64 = 300;
const CANCELLATION_POLL_MS: u64 = 250;

/// Execute a tool call that requires user approval first.
///
/// This function handles the complete approval flow:
/// - Generate action preview (diff for file operations)
/// - Queue action for user approval
/// - Wait for approval decision (approve/deny/timeout)
/// - Update session state appropriately
/// - Emit events for state changes
/// - Execute tool if approved, return error otherwise
///
/// # Arguments
///
/// * `call` - The tool call to execute
/// * `session` - The current agent session
/// * `task_id` - The current task ID
/// * `app_handle` - Tauri app handle for event emission
/// * `approval_queue` - Queue for approval-required actions
/// * `tool_registry` - Registry of available tools
/// * `file_snapshot_registry` - Registry for file state snapshots
/// * `session_repo` - Session repository for state persistence
/// * `telemetry_repo` - Telemetry repository for logging
/// * `task_repo` - Task repository for task/subtask data
///
/// # Returns
///
/// The result of the tool execution if approved, or an error.
///
/// # Errors
///
/// Returns an error if:
/// - The action is denied by the user
/// - The approval times out
/// - The session is cancelled while waiting
pub(super) async fn execute_with_approval(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    approval_queue: &ApprovalQueue,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    session_repo: &dyn SessionRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
) -> Result<OrchestratorToolResult, AgentRuntimeError> {
    let action_type = infer_action_type(&call.name, &call.arguments);
    let resume_core_state = session_repo
        .get_session(&crate::core::agent::session::SessionId(
            session.id.0.clone(),
        ))
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?
        .state;
    let mut pending = PendingAction::new(
        session.id.clone(),
        call.name.clone(),
        action_type,
        format!("Agent wants to call tool: {}", call.name),
        call.arguments.clone(),
    );

    // Generate diff preview for file write operations
    if call.name == "fs_write_file" {
        if let (Some(path), Some(content)) = (
            call.arguments.get("path").and_then(|v| v.as_str()),
            call.arguments.get("content").and_then(|v| v.as_str()),
        ) {
            if let Some(ref workspace) = session.workspace_root {
                let diff = crate::infrastructure::tools::fs::write_file::generate_diff(
                    workspace, path, content,
                );
                if let Some(diff) = diff {
                    let lang = std::path::Path::new(path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(String::from);
                    pending = pending.with_preview(ActionPreview {
                        content_type: "diff".to_string(),
                        content: diff,
                        language: lang,
                    });
                }
            }
        }
    } else if call.name == "fs_replace_in_file" {
        if let Some(ref workspace) = session.workspace_root {
            if let Ok(Some(diff)) = crate::infrastructure::tools::fs::generate_replace_preview_diff(
                workspace,
                &call.arguments,
            ) {
                let path = call
                    .arguments
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let lang = std::path::Path::new(path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(String::from);
                pending = pending.with_preview(ActionPreview {
                    content_type: "diff".to_string(),
                    content: diff,
                    language: lang,
                });
            }
        }
    }

    let action_id = pending.id.clone();

    // Update session state to waiting for input
    let waiting_state = crate::core::agent::session::SessionState::UserInputRequired {
        pending_action_ids: vec![action_id.clone()],
    };
    session_repo
        .update_state(
            &crate::core::agent::session::SessionId(session.id.0.clone()),
            &waiting_state,
        )
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    // Emit approval required event
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ApprovalRequired {
            session_id: session.id.0.clone(),
            action: pending.clone(),
        },
    );

    // Queue action
    approval_queue.queue(pending).await;

    // Wait for approval decision
    let mut rx = approval_queue.subscribe();
    let timeout = tokio::time::sleep(Duration::from_secs(APPROVAL_TIMEOUT_SECS));
    let mut cancellation_poll = tokio::time::interval(Duration::from_millis(CANCELLATION_POLL_MS));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = cancellation_poll.tick() => {
                let current_session = session_repo
                    .get_session(&crate::core::agent::session::SessionId(session.id.0.clone()))
                    .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
                if matches!(
                    current_session.state,
                    crate::core::agent::session::SessionState::Cancelled { .. }
                ) {
                    approval_queue.clear_session(&session.id.0).await;
                    return Err(AgentRuntimeError::Cancelled);
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(ApprovalEvent::ActionApproved { action_id: approved_id })
                        if approved_id == action_id =>
                    {
                        info!("[AgentRuntime] Action {action_id} approved");
                        session_repo
                            .update_state(
                                &crate::core::agent::session::SessionId(session.id.0.clone()),
                                &resume_core_state,
                            )
                            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::SessionStateChanged {
                                session_id: session.id.0.clone(),
                                state: resume_core_state.clone(),
                            },
                        );
                        return Ok(super::executor::execute_tool(
                            call,
                            session,
                            task_id,
                            app_handle,
                            tool_registry,
                            file_snapshot_registry,
                            telemetry_repo,
                            task_repo,
                        )
                        .await);
                    }
                    Ok(ApprovalEvent::ActionDenied { action_id: denied_id })
                        if denied_id == action_id =>
                    {
                        info!("[AgentRuntime] Action {action_id} denied");
                        session_repo
                            .update_state(
                                &crate::core::agent::session::SessionId(session.id.0.clone()),
                                &resume_core_state,
                            )
                            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::SessionStateChanged {
                                session_id: session.id.0.clone(),
                                state: resume_core_state.clone(),
                            },
                        );
                        return Ok(OrchestratorToolResult {
                            call_id: call.id.clone(),
                            name: call.name.clone(),
                            output: serde_json::json!({
                                "error": "Action denied by user"
                            }),
                            is_error: true,
                        });
                    }
                    Ok(ApprovalEvent::ActionTimeout { action_id: timeout_id })
                        if timeout_id == action_id =>
                    {
                        warn!("[AgentRuntime] Action {action_id} timed out");
                        session_repo
                            .update_state(
                                &crate::core::agent::session::SessionId(session.id.0.clone()),
                                &resume_core_state,
                            )
                            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::SessionStateChanged {
                                session_id: session.id.0.clone(),
                                state: resume_core_state.clone(),
                            },
                        );
                        return Ok(OrchestratorToolResult {
                            call_id: call.id.clone(),
                            name: call.name.clone(),
                            output: serde_json::json!({
                                "error": "Action timed out waiting for approval"
                            }),
                            is_error: true,
                        });
                    }
                    Err(_) => {
                        // Channel closed
                        return Err(AgentRuntimeError::ApprovalTimeout);
                    }
                    _ => continue,
                }
            }
            () = &mut timeout => {
                // Clean up pending action
                approval_queue.deny(&action_id).await;
                let _ = session_repo.update_state(
                    &crate::core::agent::session::SessionId(session.id.0.clone()),
                    &resume_core_state,
                );
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::SessionStateChanged {
                        session_id: session.id.0.clone(),
                        state: resume_core_state.clone(),
                    },
                );
                return Err(AgentRuntimeError::ApprovalTimeout);
            }
        }
    }
}
