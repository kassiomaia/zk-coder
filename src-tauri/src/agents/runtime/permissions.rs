//! Permission handling and tool execution for agent runtime.

use std::time::Duration;

use chrono::Utc;
use log::{info, warn};
use tauri::{AppHandle, Emitter};

use crate::agents::approval::{ApprovalEvent, ApprovalQueue};
use crate::agents::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::agents::rollback::RollbackStore;
use crate::agents::session::{AgentSession, PendingAction};
use crate::agents::store::SessionStore;
use crate::agents::types::{ActionPreview, ActionType};
use crate::llm::ToolCall;
use crate::tools::context::ToolExecutionContext;
use crate::tools::OrchestratorToolResult;
use crate::tools::ToolRegistry;

use super::types::AgentRuntimeError;
use super::{infer_action_type, generate_diff_from_content};

/// Timeout for waiting on user approval (seconds).
const APPROVAL_TIMEOUT_SECS: u64 = 300;

/// Extension trait for adding preview to pending actions.
trait PendingActionExt {
    fn with_preview(self, preview: ActionPreview) -> PendingAction;
}

impl PendingActionExt for PendingAction {
    fn with_preview(mut self, preview: ActionPreview) -> PendingAction {
        self.preview = Some(preview);
        self
    }
}

/// Process tool calls with permission checks, requesting approval as needed.
pub async fn process_tool_calls_with_permissions(
    tool_calls: &[ToolCall],
    session: &AgentSession,
    task_id: &str,
    permission_resolver: &crate::agents::permissions::PermissionResolver,
    app_handle: &AppHandle,
    approval_queue: &ApprovalQueue,
    tool_registry: &ToolRegistry,
    rollback_store: &RollbackStore,
    session_store: &SessionStore,
) -> Result<Vec<OrchestratorToolResult>, AgentRuntimeError> {
    let mut results = Vec::with_capacity(tool_calls.len());

    for call in tool_calls {
        let action_type = infer_action_type(&call.name, &call.arguments);
        let permission = permission_resolver.check(&call.name, &action_type);

        // Generate unique ID, persist, and emit tool call event
        let tool_call_id = uuid::Uuid::new_v4().to_string();

        // Get current subtask_id for tool call linkage
        let current_subtask_id = if let Ok(subtasks) = session_store.get_subtasks(&task_id) {
            subtasks
                .iter()
                .find(|st| st.status == crate::agents::task::TaskStatus::InProgress)
                .map(|st| st.id.clone())
        } else {
            None
        };

        if let Err(e) = session_store.insert_tool_call(
            &session.id,
            &tool_call_id,
            &call.name,
            &call.arguments,
            current_subtask_id.as_deref(),
        ) {
            warn!("[AgentRuntime] Failed to persist tool call: {e}");
        }

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ToolCall {
                session_id: session.id.0.clone(),
                tool_call_id: tool_call_id.clone(),
                tool: call.name.clone(),
                arguments: call.arguments.clone(),
            },
        );

        let result = match permission {
            crate::agents::types::PermissionLevel::AutoRun => {
                execute_tool(
                    call,
                    session,
                    app_handle,
                    tool_registry,
                    rollback_store,
                    session_store,
                )
                .await
            }
            crate::agents::types::PermissionLevel::ApprovalRequired => {
                execute_with_approval(
                    call,
                    session,
                    task_id,
                    app_handle,
                    approval_queue,
                    tool_registry,
                    rollback_store,
                    session_store,
                )
                .await?
            }
            crate::agents::types::PermissionLevel::Disabled => OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "This tool is disabled by the agent's permission policy"
                }),
                is_error: true,
            },
        };

        // Persist result and emit tool result event
        if let Err(e) = session_store.update_tool_call_result(
            &tool_call_id,
            &result.output,
            result.is_error,
            None,
        ) {
            warn!("[AgentRuntime] Failed to persist tool result: {e}");
        }

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ToolResult {
                session_id: session.id.0.clone(),
                tool_call_id: tool_call_id.clone(),
                tool: call.name.clone(),
                result: result.output.clone(),
                is_error: result.is_error,
            },
        );

        results.push(result);
    }

    Ok(results)
}

/// Execute a tool call using the tool registry.
async fn execute_tool(
    call: &ToolCall,
    session: &AgentSession,
    app_handle: &AppHandle,
    tool_registry: &ToolRegistry,
    rollback_store: &RollbackStore,
    session_store: &SessionStore,
) -> OrchestratorToolResult {
    // Capture file state before write operations for rollback
    if call.name == "fs_write_file" {
        if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
            if let Some(ref workspace) = session.workspace_root {
                let target = if std::path::Path::new(path).is_absolute() {
                    std::path::PathBuf::from(path)
                } else {
                    workspace.join(path)
                };
                rollback_store.capture(&session.id.0, &target);
            }
        }
    }

    let context = ToolExecutionContext::new(&session.conversation_id)
        .with_thread(session.thread_id.clone())
        .with_session(session.id.0.clone());
    let context = if let Some(ref workspace) = session.workspace_root {
        context.with_workspace(workspace.clone())
    } else {
        context
    };

    let llm_call = crate::llm::ToolCall {
        id: call.id.clone(),
        name: call.name.clone(),
        arguments: call.arguments.clone(),
    };

    let result: crate::llm::ToolResult = tool_registry
        .execute(llm_call, context, app_handle.clone())
        .await;

    // Emit file touched event for successful file operations
    if !result.is_error {
        emit_file_events(call, session, app_handle, rollback_store, session_store);
    }

    OrchestratorToolResult {
        call_id: result.call_id,
        name: call.name.clone(),
        output: result.output,
        is_error: result.is_error,
    }
}

/// Execute a tool call that requires user approval first.
async fn execute_with_approval(
    call: &ToolCall,
    session: &AgentSession,
    _task_id: &str,
    app_handle: &AppHandle,
    approval_queue: &ApprovalQueue,
    tool_registry: &ToolRegistry,
    rollback_store: &RollbackStore,
    session_store: &SessionStore,
) -> Result<OrchestratorToolResult, AgentRuntimeError> {
    let action_type = infer_action_type(&call.name, &call.arguments);
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
                let diff = crate::tools::fs::write_file::generate_diff(workspace, path, content);
                if let Some(diff) = diff {
                    let lang = std::path::Path::new(path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(String::from);
                    pending = pending.with_preview(crate::agents::types::ActionPreview {
                        content_type: "diff".to_string(),
                        content: diff,
                        language: lang,
                    });
                }
            }
        }
    }

    let action_id = pending.id.clone();

    // Update session state to waiting for input
    session_store
        .update_state(
            &session.id,
            &crate::agents::session::SessionState::UserInputRequired {
                pending_action_ids: vec![action_id.clone()],
            },
        )
        .map_err(|e| AgentRuntimeError::Store(e.to_string()))?;

    // Emit approval required event
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ApprovalRequired {
            session_id: session.id.0.clone(),
            action: pending.clone(),
        },
    );

    // Queue the action
    approval_queue.queue(pending).await;

    // Wait for approval decision
    let mut rx = approval_queue.subscribe();
    let timeout = tokio::time::sleep(Duration::from_secs(APPROVAL_TIMEOUT_SECS));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(ApprovalEvent::ActionApproved { action_id: approved_id })
                        if approved_id == action_id =>
                    {
                        info!("[AgentRuntime] Action {} approved", action_id);
                        return Ok(execute_tool(call, session, app_handle, tool_registry, rollback_store, session_store).await);
                    }
                    Ok(ApprovalEvent::ActionDenied { action_id: denied_id })
                        if denied_id == action_id =>
                    {
                        info!("[AgentRuntime] Action {} denied", action_id);
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
                        warn!("[AgentRuntime] Action {} timed out", action_id);
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
            _ = &mut timeout => {
                // Clean up the pending action
                approval_queue.deny(&action_id).await;
                return Err(AgentRuntimeError::ApprovalTimeout);
            }
        }
    }
}

/// Emits file-related events after successful tool execution.
fn emit_file_events(
    call: &ToolCall,
    session: &AgentSession,
    app_handle: &AppHandle,
    rollback_store: &RollbackStore,
    session_store: &SessionStore,
) {
    match call.name.as_str() {
        "fs_write_file" => {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                // Generate diff for post-session review
                let diff = if let Some(workspace_root) = &session.workspace_root {
                    let original_content = rollback_store.get_snapshot_content(&session.id.0, path);
                    let current_content = std::fs::read_to_string(workspace_root.join(path)).ok();

                    match (original_content, current_content) {
                        (Some(original), Some(current)) => {
                            Some(generate_diff_from_content(path, &original, &current))
                        }
                        (None, Some(current)) => {
                            let line_count = current.lines().count();
                            let lines: String = current
                                .lines()
                                .map(|line| format!("+{}", line))
                                .collect::<Vec<_>>()
                                .join("\n");
                            Some(format!(
                                "--- /dev/null\n+++ b/{}\n@@ -0,0 +1,{} @@\n{}",
                                path, line_count, lines
                            ))
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::FileTouched {
                        session_id: session.id.0.clone(),
                        file_path: path.to_string(),
                        action: crate::agents::events::FileAction::Write,
                        diff: diff.clone(),
                    },
                );

                let _ = session_store.add_file_touched(&session.id, path, "write", diff.as_deref());
            }
        }
        "fs_delete_file" => {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::FileTouched {
                        session_id: session.id.0.clone(),
                        file_path: path.to_string(),
                        action: crate::agents::events::FileAction::Delete,
                        diff: None,
                    },
                );

                let _ = session_store.add_file_touched(&session.id, path, "delete", None);
            }
        }
        "fs_read_file" => {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::FileTouched {
                        session_id: session.id.0.clone(),
                        file_path: path.to_string(),
                        action: crate::agents::events::FileAction::Read,
                        diff: None,
                    },
                );

                let _ = session_store.add_file_touched(&session.id, path, "read", None);
            }
        }
        "agent_emit_status" => {
            if let Some(message) = call.arguments.get("message").and_then(|v| v.as_str()) {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::StatusPhrase {
                        session_id: session.id.0.clone(),
                        message: message.to_string(),
                        timestamp: Utc::now(),
                    },
                );

                let _ = session_store.add_status_phrase(&session.id, message);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_action_ext_with_preview() {
        use crate::agents::session::SessionId;

        let pending = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test action".to_string(),
            serde_json::json!({}),
        );

        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "test diff".to_string(),
            language: Some("rust".to_string()),
        };

        let with_preview = pending.with_preview(preview.clone());
        assert!(with_preview.preview.is_some());
        assert_eq!(with_preview.preview.unwrap().content, "test diff");
    }
}
