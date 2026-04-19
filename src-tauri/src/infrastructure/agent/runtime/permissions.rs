//! Permission handling and tool execution for agent runtime.

use std::time::Duration;

use chrono::Utc;
use log::{info, warn};
use tauri::{AppHandle, Emitter};

use super::session_scope::RuntimeSessionScope;
use super::tool_feedback::{build_tool_call_feedback, build_tool_result_feedback};
use super::tool_handlers::handle_agent_tool_call;
use super::types::AgentRuntimeError;

// Re-exported so existing cross-module callers (e.g. termination.rs)
// keep importing from `permissions`.
pub(crate) use super::tool_handlers::shared::core_task_to_event_subtask;
use super::{generate_diff_from_content, infer_action_type};
use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::{AgentSession, PendingAction};
use crate::core::agent::types::ActionPreview;
// `ActionType` is only referenced by the sibling test module loaded via
// `#[path]`. Keep the import here so the tests can name it via
// `super::ActionType`.
#[allow(unused_imports)]
use crate::core::agent::types::ActionType;
use crate::core::llm::entities::{ToolCall, ToolResult};
use crate::infrastructure::agent::approval::{ApprovalEvent, ApprovalQueue};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::llm::malformed_args::{build_error_result, extract_marker};
use crate::infrastructure::tools::context::ToolExecutionContext;
use crate::infrastructure::tools::OrchestratorToolResult;
use crate::infrastructure::tools::ToolRegistry;

/// Timeout for waiting on user approval (seconds).
const APPROVAL_TIMEOUT_SECS: u64 = 300;
const CANCELLATION_POLL_MS: u64 = 250;

/// Provides dynamic YOLO mode checks for runtime permission overrides.
pub trait YoloModeProvider: Send + Sync {
    /// Returns true when YOLO mode is enabled for the given thread.
    fn is_yolo_enabled(&self, thread_id: &str) -> bool;
}

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

fn emit_persisted_streaming_message(
    app_handle: &AppHandle,
    session: &AgentSession,
    message_id: String,
    content: String,
) {
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::StreamingContent {
            session_id: session.id.0.clone(),
            conversation_id: session.conversation_id.clone(),
            thread_id: session.thread_id.clone(),
            chunk: content,
            is_final: true,
            message_id: Some(message_id),
            persist_message: Some(true),
        },
    );
}

/// Process tool calls with permission checks, requesting approval as needed.
///
/// `allowed_tool_names`, when `Some`, restricts which tool names may actually
/// be invoked. Calls whose name is not in the list are rejected at runtime with
/// an error result — the LLM still saw the tool in its catalog (M2.2 stable
/// session-visible catalog), but the subtask mode or final-summary path
/// disallows calling it. `None` means no runtime restriction.
#[allow(clippy::too_many_arguments)]
pub async fn process_tool_calls_with_permissions(
    tool_calls: &[ToolCall],
    session: &AgentSession,
    task_id: &str,
    permission_resolver: &crate::core::agent::permissions::PermissionResolver,
    yolo_mode_provider: &dyn YoloModeProvider,
    allowed_tool_names: Option<&[String]>,
    app_handle: &AppHandle,
    approval_queue: &ApprovalQueue,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    session_repo: &dyn SessionRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
    scope: &mut RuntimeSessionScope,
) -> Result<Vec<OrchestratorToolResult>, AgentRuntimeError> {
    let mut results = Vec::with_capacity(tool_calls.len());

    for call in tool_calls {
        // Recoverable-malformed-args fast path: the LLM client wraps an
        // unparseable `function.arguments` string in a sentinel object so we
        // can surface it as a tool-level error instead of aborting the whole
        // session. Do not dispatch, do not check permissions — produce an
        // error result with diagnostic context and move on. The normal tool-
        // result feedback loop then asks the model to self-correct on the
        // next step. See docs/issues/agent-plan-subtasks-malformed-json.
        if let Some(marker) = extract_marker(&call.arguments) {
            let tool_call_id = uuid::Uuid::new_v4().to_string();
            let output = build_error_result(&call.name, marker);
            if let Err(e) = telemetry_repo.insert_tool_call(
                &session.id,
                &tool_call_id,
                &call.name,
                &call.arguments,
                None,
            ) {
                warn!("[AgentRuntime] Failed to persist malformed tool call: {e}");
            }
            if let Err(e) =
                telemetry_repo.update_tool_call_result(&tool_call_id, &output, true, None)
            {
                warn!("[AgentRuntime] Failed to persist malformed tool result: {e}");
            }
            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::ToolResult {
                    session_id: session.id.0.clone(),
                    tool_call_id: tool_call_id.clone(),
                    tool: call.name.clone(),
                    result: output.clone(),
                    is_error: true,
                    content: Some(format!(
                        "`{}` was called with malformed JSON arguments; asking the model to retry.",
                        call.name
                    )),
                },
            );
            results.push(OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output,
                is_error: true,
            });
            continue;
        }

        let disallowed_by_mode = allowed_tool_names
            .is_some_and(|allowed| !allowed.iter().any(|name| name == &call.name));

        let action_type = infer_action_type(&call.name, &call.arguments);
        let mut permission = permission_resolver.check(&call.name, &action_type);
        if yolo_mode_provider.is_yolo_enabled(&session.thread_id)
            && permission != crate::core::agent::types::PermissionLevel::Disabled
        {
            permission = crate::core::agent::types::PermissionLevel::AutoRun;
        }

        // Generate unique ID, persist, and emit tool call event
        let tool_call_id = uuid::Uuid::new_v4().to_string();

        // Get current subtask_id for tool call linkage
        let current_subtask_id = if let Ok(subtasks) = task_repo.get_subtasks(&task_id) {
            subtasks
                .iter()
                .find(|st| st.status == crate::core::agent::task::TaskStatus::InProgress)
                .map(|st| st.id.clone())
        } else {
            None
        };

        if let Err(e) = telemetry_repo.insert_tool_call(
            &session.id,
            &tool_call_id,
            &call.name,
            &call.arguments,
            current_subtask_id.as_deref(),
        ) {
            warn!("[AgentRuntime] Failed to persist tool call: {e}");
        }

        if let Some(content) = build_tool_call_feedback(&call.name, &call.arguments) {
            emit_persisted_streaming_message(
                app_handle,
                session,
                format!("streaming-tool-call-{}", tool_call_id),
                content.clone(),
            );
        }

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ToolCall {
                session_id: session.id.0.clone(),
                tool_call_id: tool_call_id.clone(),
                tool: call.name.clone(),
                arguments: call.arguments.clone(),
                content: build_tool_call_feedback(&call.name, &call.arguments),
            },
        );

        let result = if disallowed_by_mode {
            OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": format!(
                        "Tool '{}' is not permitted under the current subtask mode. \
                         Call a tool from the allowed set, or call \
                         `agent_advance_to_next_subtask` to move to a subtask whose \
                         mode allows this tool.",
                        call.name
                    ),
                    "reason": "subtask_mode_disallowed",
                    "allowedToolNames": allowed_tool_names.unwrap_or(&[]),
                }),
                is_error: true,
            }
        } else {
            match permission {
                crate::core::agent::types::PermissionLevel::AutoRun => {
                    execute_tool(
                        call,
                        session,
                        task_id,
                        current_subtask_id.as_deref(),
                        app_handle,
                        tool_registry,
                        file_snapshot_registry,
                        session_repo,
                        telemetry_repo,
                        task_repo,
                        scope,
                    )
                    .await
                }
                crate::core::agent::types::PermissionLevel::ApprovalRequired => {
                    execute_with_approval(
                        call,
                        session,
                        task_id,
                        current_subtask_id.as_deref(),
                        app_handle,
                        approval_queue,
                        tool_registry,
                        file_snapshot_registry,
                        session_repo,
                        telemetry_repo,
                        task_repo,
                        scope,
                    )
                    .await?
                }
                crate::core::agent::types::PermissionLevel::Disabled => OrchestratorToolResult {
                    call_id: call.id.clone(),
                    name: call.name.clone(),
                    output: serde_json::json!({
                        "error": "This tool is disabled by the agent's permission policy"
                    }),
                    is_error: true,
                },
            }
        };

        // Persist result and emit tool result event
        if let Err(e) = telemetry_repo.update_tool_call_result(
            &tool_call_id,
            &result.output,
            result.is_error,
            None,
        ) {
            warn!("[AgentRuntime] Failed to persist tool result: {e}");
        }

        if let Some(content) =
            build_tool_result_feedback(&call.name, &result.output, result.is_error)
        {
            emit_persisted_streaming_message(
                app_handle,
                session,
                format!("streaming-tool-result-{}", tool_call_id),
                content.clone(),
            );
        }

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ToolResult {
                session_id: session.id.0.clone(),
                tool_call_id: tool_call_id.clone(),
                tool: call.name.clone(),
                result: result.output.clone(),
                is_error: result.is_error,
                content: build_tool_result_feedback(&call.name, &result.output, result.is_error),
            },
        );

        results.push(result);
    }

    Ok(results)
}

/// Execute a tool call using the tool registry.
#[allow(clippy::too_many_arguments)]
async fn execute_tool(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    current_subtask_id: Option<&str>,
    app_handle: &AppHandle,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    session_repo: &dyn SessionRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
    scope: &mut RuntimeSessionScope,
) -> OrchestratorToolResult {
    if let Some(result) = handle_agent_tool_call(
        call,
        session,
        task_id,
        current_subtask_id,
        app_handle,
        session_repo,
        task_repo,
        telemetry_repo,
        scope,
    ) {
        return result;
    }

    if call.name == "system_execute_command" {
        if let Some(ref workspace) = session.workspace_root {
            let stats =
                file_snapshot_registry.capture_workspace_best_effort(&session.id.0, workspace);
            info!(
                "[AgentRuntime] Captured workspace snapshot before command: scanned={}, captured={}, skipped_large={}, truncated={}",
                stats.scanned_files,
                stats.captured_files,
                stats.skipped_large_files,
                stats.truncated
            );
        }
    }

    // Capture file state before write operations for rollback
    if call.name == "fs_write_file" {
        if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
            if let Some(ref workspace) = session.workspace_root {
                let target = if std::path::Path::new(path).is_absolute() {
                    std::path::PathBuf::from(path)
                } else {
                    workspace.join(path)
                };
                file_snapshot_registry.capture(&session.id.0, &target);
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

    let llm_call = ToolCall {
        id: call.id.clone(),
        name: call.name.clone(),
        arguments: call.arguments.clone(),
    };

    let result: ToolResult = tool_registry
        .execute(llm_call, context, app_handle.clone())
        .await;

    // Emit file touched event for successful file operations
    if !result.is_error {
        emit_file_events(
            call,
            session,
            app_handle,
            file_snapshot_registry,
            telemetry_repo,
        );
    }

    OrchestratorToolResult {
        call_id: result.call_id,
        name: call.name.clone(),
        output: result.output,
        is_error: result.is_error,
    }
}

/// Execute a tool call that requires user approval first.
#[allow(clippy::too_many_arguments)]
async fn execute_with_approval(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    current_subtask_id: Option<&str>,
    app_handle: &AppHandle,
    approval_queue: &ApprovalQueue,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    session_repo: &dyn SessionRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
    scope: &mut RuntimeSessionScope,
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
                    pending = pending.with_preview(crate::core::agent::types::ActionPreview {
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
                pending = pending.with_preview(crate::core::agent::types::ActionPreview {
                    content_type: "diff".to_string(),
                    content: diff,
                    language: lang,
                });
            }
        }
    }

    let action_id = pending.id.clone();

    // Update session state to waiting for input
    let core_state: crate::core::agent::session::SessionState =
        crate::core::agent::session::SessionState::UserInputRequired {
            pending_action_ids: vec![action_id.clone()],
        }
        .into();
    session_repo
        .update_state(
            &crate::core::agent::session::SessionId(session.id.0.clone()),
            &core_state,
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

    // Queue the action
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
                        info!("[AgentRuntime] Action {} approved", action_id);
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
                        return Ok(execute_tool(
                            call,
                            session,
                            task_id,
                            current_subtask_id,
                            app_handle,
                            tool_registry,
                            file_snapshot_registry,
                            session_repo,
                            telemetry_repo,
                            task_repo,
                            scope,
                        )
                        .await);
                    }
                    Ok(ApprovalEvent::ActionDenied { action_id: denied_id })
                        if denied_id == action_id =>
                    {
                        info!("[AgentRuntime] Action {} denied", action_id);
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
                        warn!("[AgentRuntime] Action {} timed out", action_id);
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
            _ = &mut timeout => {
                // Clean up the pending action
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


/// Emits file-related events after successful tool execution.
fn emit_file_events(
    call: &ToolCall,
    session: &AgentSession,
    app_handle: &AppHandle,
    file_snapshot_registry: &FileSnapshotRegistry,
    telemetry_repo: &dyn AgentTelemetryRepository,
) {
    match call.name.as_str() {
        "fs_write_file" | "fs_replace_in_file" => {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                // Generate diff for post-session review
                let diff = if let Some(workspace_root) = &session.workspace_root {
                    let original_content =
                        file_snapshot_registry.get_snapshot_content(&session.id.0, path);
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
                        action: crate::infrastructure::agent::events::FileAction::Write,
                        diff: diff.clone(),
                    },
                );

                let _ =
                    telemetry_repo.add_file_touched(&session.id, path, "write", diff.as_deref());
            }
        }
        "fs_delete_file" => {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::FileTouched {
                        session_id: session.id.0.clone(),
                        file_path: path.to_string(),
                        action: crate::infrastructure::agent::events::FileAction::Delete,
                        diff: None,
                    },
                );

                let _ = telemetry_repo.add_file_touched(&session.id, path, "delete", None);
            }
        }
        "fs_read_file" => {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::FileTouched {
                        session_id: session.id.0.clone(),
                        file_path: path.to_string(),
                        action: crate::infrastructure::agent::events::FileAction::Read,
                        diff: None,
                    },
                );

                let _ = telemetry_repo.add_file_touched(&session.id, path, "read", None);
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

                let _ = telemetry_repo.add_status_phrase(&session.id, message);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "permissions_tests.rs"]
mod tests;
