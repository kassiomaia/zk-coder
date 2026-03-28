//! Permission handling and tool execution for agent runtime.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use chrono::Utc;
use log::{info, warn};
use tauri::{AppHandle, Emitter};

use super::types::AgentRuntimeError;
use super::{generate_diff_from_content, infer_action_type};
use super::tool_feedback::{build_tool_call_feedback, build_tool_result_feedback};
use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::{AgentSession, PendingAction};
use crate::core::agent::types::{ActionPreview, ActionType};
use crate::core::llm::entities::{ToolCall, ToolResult};
use crate::infrastructure::agent::approval::{ApprovalEvent, ApprovalQueue};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::context::ToolExecutionContext;
use crate::infrastructure::tools::OrchestratorToolResult;
use crate::infrastructure::tools::ToolRegistry;

static PLAN_RETRY_TRACKER: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static SUBTASK_RATIONALES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
static SUBTASK_FAILURE_COUNTS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
static DECOMPOSED_SUBTASKS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// Timeout for waiting on user approval (seconds).
const APPROVAL_TIMEOUT_SECS: u64 = 300;
const CANCELLATION_POLL_MS: u64 = 250;

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

fn runtime_session_state_from_core(
    state: &crate::core::agent::entities::SessionState,
) -> crate::core::agent::session::SessionState {
    match state {
        crate::core::agent::entities::SessionState::WaitForRequest { last_action_at } => {
            crate::core::agent::session::SessionState::WaitForRequest {
                last_action_at: *last_action_at,
            }
        }
        crate::core::agent::entities::SessionState::Processing { .. } => {
            crate::core::agent::session::SessionState::Running {
                current_step: 0,
                context_usage: 0.0,
            }
        }
        crate::core::agent::entities::SessionState::AwaitingApproval { action_id, .. } => {
            crate::core::agent::session::SessionState::UserInputRequired {
                pending_action_ids: vec![action_id.clone()],
            }
        }
        crate::core::agent::entities::SessionState::Finished { completed_at, .. } => {
            crate::core::agent::session::SessionState::Finished {
                completed_at: *completed_at,
            }
        }
        crate::core::agent::entities::SessionState::Error { error, .. } => {
            crate::core::agent::session::SessionState::Error {
                message: error.clone(),
                recoverable: false,
            }
        }
        crate::core::agent::entities::SessionState::Cancelled { reason, .. } => {
            crate::core::agent::session::SessionState::Cancelled {
                reason: reason.clone(),
            }
        }
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
pub async fn process_tool_calls_with_permissions(
    tool_calls: &[ToolCall],
    session: &AgentSession,
    task_id: &str,
    permission_resolver: &crate::core::agent::permissions::PermissionResolver,
    app_handle: &AppHandle,
    approval_queue: &ApprovalQueue,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    session_repo: &dyn SessionRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
) -> Result<Vec<OrchestratorToolResult>, AgentRuntimeError> {
    let mut results = Vec::with_capacity(tool_calls.len());

    for call in tool_calls {
        let action_type = infer_action_type(&call.name, &call.arguments);
        let permission = permission_resolver.check(&call.name, &action_type);

        // Generate unique ID, persist, and emit tool call event
        let tool_call_id = uuid::Uuid::new_v4().to_string();

        // Get current subtask_id for tool call linkage
        let current_subtask_id = if let Ok(subtasks) = task_repo.get_subtasks(&task_id) {
            subtasks
                .iter()
                .find(|st| st.status == crate::core::agent::entities::TaskStatus::InProgress)
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

        let result = match permission {
            crate::core::agent::types::PermissionLevel::AutoRun => {
                execute_tool(
                    call,
                    session,
                    task_id,
                    app_handle,
                    tool_registry,
                    file_snapshot_registry,
                    telemetry_repo,
                    task_repo,
                )
                .await
            }
            crate::core::agent::types::PermissionLevel::ApprovalRequired => {
                execute_with_approval(
                    call,
                    session,
                    task_id,
                    app_handle,
                    approval_queue,
                    tool_registry,
                    file_snapshot_registry,
                    session_repo,
                    telemetry_repo,
                    task_repo,
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

        if let Some(content) = build_tool_result_feedback(&call.name, &result.output, result.is_error)
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
async fn execute_tool(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
) -> OrchestratorToolResult {
    if let Some(result) = handle_agent_tool_call(
        call,
        session,
        task_id,
        app_handle,
        task_repo,
        telemetry_repo,
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
async fn execute_with_approval(
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
        .get_session(&crate::core::agent::entities::SessionId(session.id.0.clone()))
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
    }

    let action_id = pending.id.clone();

    // Update session state to waiting for input
    let core_state: crate::core::agent::entities::SessionState =
        crate::core::agent::session::SessionState::UserInputRequired {
            pending_action_ids: vec![action_id.clone()],
        }
        .into();
    session_repo
        .update_state(
            &crate::core::agent::entities::SessionId(session.id.0.clone()),
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
                    .get_session(&crate::core::agent::entities::SessionId(session.id.0.clone()))
                    .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
                if matches!(
                    current_session.state,
                    crate::core::agent::entities::SessionState::Cancelled { .. }
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
                                &crate::core::agent::entities::SessionId(session.id.0.clone()),
                                &resume_core_state,
                            )
                            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::SessionStateChanged {
                                session_id: session.id.0.clone(),
                                state: runtime_session_state_from_core(&resume_core_state),
                            },
                        );
                        return Ok(execute_tool(
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
                        info!("[AgentRuntime] Action {} denied", action_id);
                        session_repo
                            .update_state(
                                &crate::core::agent::entities::SessionId(session.id.0.clone()),
                                &resume_core_state,
                            )
                            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::SessionStateChanged {
                                session_id: session.id.0.clone(),
                                state: runtime_session_state_from_core(&resume_core_state),
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
                                &crate::core::agent::entities::SessionId(session.id.0.clone()),
                                &resume_core_state,
                            )
                            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::SessionStateChanged {
                                session_id: session.id.0.clone(),
                                state: runtime_session_state_from_core(&resume_core_state),
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
                    &crate::core::agent::entities::SessionId(session.id.0.clone()),
                    &resume_core_state,
                );
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::SessionStateChanged {
                        session_id: session.id.0.clone(),
                        state: runtime_session_state_from_core(&resume_core_state),
                    },
                );
                return Err(AgentRuntimeError::ApprovalTimeout);
            }
        }
    }
}

fn handle_agent_tool_call(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> Option<OrchestratorToolResult> {
    match call.name.as_str() {
        "agent_plan_subtasks" => Some(handle_plan_subtasks(
            call,
            session,
            task_id,
            app_handle,
            task_repo,
            telemetry_repo,
        )),
        "agent_advance_to_next_subtask" => Some(handle_advance_subtask(
            call,
            session,
            task_id,
            app_handle,
            task_repo,
            telemetry_repo,
        )),
        "agent_mark_subtask_status" => Some(handle_mark_subtask_status(
            call,
            session,
            task_id,
            app_handle,
            task_repo,
            telemetry_repo,
        )),
        "agent_narrate" => Some(handle_narrate(call, session, app_handle, telemetry_repo)),
        _ => None,
    }
}

fn handle_plan_subtasks(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> OrchestratorToolResult {
    let subtasks = match call.arguments.get("subtasks").and_then(|v| v.as_array()) {
        Some(subtasks) if !subtasks.is_empty() => subtasks,
        _ => {
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "Missing or empty 'subtasks' array parameter"
                }),
                is_error: true,
            };
        }
    };

    let existing = task_repo.get_subtasks(task_id).ok().unwrap_or_default();
    if !existing.is_empty() {
        return OrchestratorToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            output: serde_json::json!({
                "success": true,
                "message": "Subtasks already planned for this task. Continue with current subtask."
            }),
            is_error: false,
        };
    }

    let requires_verification =
        detect_code_change_intent(session, task_id, task_repo, telemetry_repo);
    if let Err(feedback) = validate_plan_subtasks(subtasks, requires_verification) {
        let retry_map = PLAN_RETRY_TRACKER.get_or_init(|| Mutex::new(HashSet::new()));
        let mut retries = retry_map.lock().unwrap_or_else(|e| e.into_inner());
        let retry_key = format!("{}:{}", session.id.0, task_id);
        let already_retried = retries.contains(&retry_key);
        if !already_retried {
            retries.insert(retry_key);
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "Plan validation failed",
                    "requires_replan": true,
                    "feedback": feedback
                }),
                is_error: true,
            };
        }
        return OrchestratorToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            output: serde_json::json!({
                "error": "Plan validation failed twice",
                "requires_replan": false,
                "feedback": feedback
            }),
            is_error: true,
        };
    }

    let mut created = 0usize;

    for (idx, item) in subtasks.iter().enumerate() {
        let title = match item.get("title").and_then(|v| v.as_str()) {
            Some(title) if !title.trim().is_empty() => title.trim().to_string(),
            _ => {
                return OrchestratorToolResult {
                    call_id: call.id.clone(),
                    name: call.name.clone(),
                    output: serde_json::json!({
                        "error": "Each subtask must include a non-empty 'title'"
                    }),
                    is_error: true,
                };
            }
        };
        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let core_subtask = crate::core::agent::entities::AgentTask {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session.id.0.clone(),
            parent_task_id: Some(task_id.to_string()),
            title: title.clone(),
            description: description.clone(),
            status: crate::core::agent::entities::TaskStatus::Pending,
            order: idx as i32,
            mode: Some(crate::core::agent::entities::TaskMode::FullAccess),
            needs_review: false,
        };

        if let Err(e) = task_repo.create_subtask(task_id, &core_subtask) {
            warn!("[AgentRuntime] Failed to create subtask: {e}");
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "Failed to persist subtask"
                }),
                is_error: true,
            };
        }

        let mut event_subtask = crate::core::agent::task::SubTask::new(
            task_id.to_string(),
            title,
            crate::core::agent::task::SubTaskMode::FullAccess,
            idx as i32,
        );
        event_subtask.id = core_subtask.id.clone();
        if let Some(desc) = description {
            event_subtask.description = Some(desc);
        }

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::SubtaskCreated {
                session_id: session.id.0.clone(),
                subtask: event_subtask,
            },
        );

        created += 1;
    }

    if created > 0 {
        let created_subtasks = task_repo.get_subtasks(task_id).ok().unwrap_or_default();
        if let Some(first) = created_subtasks.iter().find(|st| st.order == 0).cloned() {
            let _ = task_repo.update_subtask_status(
                &first.id,
                crate::core::agent::entities::TaskStatus::InProgress,
            );

            let mut updated = core_task_to_event_subtask(first, task_id);
            updated.status = crate::core::agent::task::TaskStatus::InProgress;
            updated.started_at = Some(Utc::now());

            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::SubtaskUpdated {
                    session_id: session.id.0.clone(),
                    subtask: updated,
                },
            );
        }
    }

    OrchestratorToolResult {
        call_id: call.id.clone(),
        name: call.name.clone(),
        output: serde_json::json!({
            "success": true,
            "message": format!("Planned {created} subtasks"),
            "count": created
        }),
        is_error: false,
    }
}

fn handle_advance_subtask(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> OrchestratorToolResult {
    let mut subtasks = match task_repo.get_subtasks(task_id) {
        Ok(subtasks) if !subtasks.is_empty() => subtasks,
        _ => {
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "No subtasks found for this task"
                }),
                is_error: true,
            };
        }
    };

    subtasks.sort_by_key(|st| st.order);

    let current_index = subtasks
        .iter()
        .position(|st| st.status == crate::core::agent::entities::TaskStatus::InProgress);

    let Some(current_pos) = current_index else {
        return OrchestratorToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            output: serde_json::json!({
                "error": "No subtask is currently in progress"
            }),
            is_error: true,
        };
    };

    let current = subtasks[current_pos].clone();
    let has_evidence = has_subtask_evidence(session, &current.id, telemetry_repo)
        || has_subtask_rationale(&current.id);
    if !has_evidence {
        return OrchestratorToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            output: serde_json::json!({
                "error": "Cannot advance subtask: no evidence found (file change, command execution, or explicit rationale)"
            }),
            is_error: true,
        };
    }

    if let Err(e) = task_repo.update_subtask_status(
        &current.id,
        crate::core::agent::entities::TaskStatus::Completed,
    ) {
        warn!("[AgentRuntime] Failed to complete subtask: {e}");
        return OrchestratorToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            output: serde_json::json!({
                "error": "Failed to update subtask status"
            }),
            is_error: true,
        };
    }

    let mut completed_event = core_task_to_event_subtask(current, task_id);
    completed_event.status = crate::core::agent::task::TaskStatus::Completed;
    completed_event.completed_at = Some(Utc::now());
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SubtaskUpdated {
            session_id: session.id.0.clone(),
            subtask: completed_event,
        },
    );

    let next = subtasks
        .iter()
        .skip(current_pos + 1)
        .find(|st| st.status == crate::core::agent::entities::TaskStatus::Pending)
        .cloned();

    if let Some(next) = next {
        if let Err(e) = task_repo.update_subtask_status(
            &next.id,
            crate::core::agent::entities::TaskStatus::InProgress,
        ) {
            warn!("[AgentRuntime] Failed to start subtask: {e}");
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "Failed to update next subtask status"
                }),
                is_error: true,
            };
        }

        let mut in_progress_event = core_task_to_event_subtask(next, task_id);
        in_progress_event.status = crate::core::agent::task::TaskStatus::InProgress;
        in_progress_event.started_at = Some(Utc::now());
        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::SubtaskUpdated {
                session_id: session.id.0.clone(),
                subtask: in_progress_event,
            },
        );

        return OrchestratorToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            output: serde_json::json!({
                "success": true,
                "message": "Advanced to next subtask"
            }),
            is_error: false,
        };
    }

    let _ =
        task_repo.update_task_status(task_id, crate::core::agent::entities::TaskStatus::Completed);
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: session.id.0.clone(),
            task_id: task_id.to_string(),
            status: crate::core::agent::task::TaskStatus::Completed,
        },
    );

    OrchestratorToolResult {
        call_id: call.id.clone(),
        name: call.name.clone(),
        output: serde_json::json!({
            "success": true,
            "message": "Advanced to next subtask (no remaining subtasks)"
        }),
        is_error: false,
    }
}

fn handle_mark_subtask_status(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
    _telemetry_repo: &dyn AgentTelemetryRepository,
) -> OrchestratorToolResult {
    let index = match call.arguments.get("index").and_then(|v| v.as_i64()) {
        Some(index) if index >= 0 => index as usize,
        _ => {
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "Missing or invalid 'index' parameter"
                }),
                is_error: true,
            };
        }
    };

    let status_str = match call.arguments.get("status").and_then(|v| v.as_str()) {
        Some(status) => status,
        _ => {
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "Missing 'status' parameter"
                }),
                is_error: true,
            };
        }
    };
    let rationale = call
        .arguments
        .get("rationale")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let (core_status, event_status) = match parse_status(status_str) {
        Some(status) => status,
        None => {
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": format!("Invalid status '{}'", status_str)
                }),
                is_error: true,
            };
        }
    };

    let mut subtasks = match task_repo.get_subtasks(task_id) {
        Ok(subtasks) if index < subtasks.len() => subtasks,
        _ => {
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "Subtask index out of range"
                }),
                is_error: true,
            };
        }
    };

    subtasks.sort_by_key(|st| st.order);
    let current = subtasks[index].clone();
    let current_for_decompose = current.clone();

    if let Err(e) = task_repo.update_subtask_status(&current.id, core_status) {
        warn!("[AgentRuntime] Failed to update subtask status: {e}");
        return OrchestratorToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            output: serde_json::json!({
                "error": "Failed to update subtask status"
            }),
            is_error: true,
        };
    }

    let mut updated = core_task_to_event_subtask(current, task_id);
    updated.status = event_status.clone();
    if matches!(
        event_status,
        crate::core::agent::task::TaskStatus::InProgress
    ) {
        updated.started_at = Some(Utc::now());
    }
    if matches!(
        event_status,
        crate::core::agent::task::TaskStatus::Completed
            | crate::core::agent::task::TaskStatus::Failed
    ) {
        updated.completed_at = Some(Utc::now());
    }

    let updated_id = updated.id.clone();
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SubtaskUpdated {
            session_id: session.id.0.clone(),
            subtask: updated,
        },
    );

    if let Some(reason) = rationale {
        remember_subtask_rationale(&updated_id, &reason);
    }

    if matches!(event_status, crate::core::agent::task::TaskStatus::Failed) {
        let fail_count = increment_subtask_failure_count(&updated_id);
        if fail_count >= 2 && mark_subtask_decomposed_once(&updated_id) {
            auto_decompose_failed_subtask(
                session,
                task_id,
                &core_task_to_event_subtask(current_for_decompose, task_id),
                app_handle,
                task_repo,
            );
        }
    }

    OrchestratorToolResult {
        call_id: call.id.clone(),
        name: call.name.clone(),
        output: serde_json::json!({
            "success": true,
            "message": format!("Marked subtask {} as {}", index, status_str)
        }),
        is_error: false,
    }
}

fn handle_narrate(
    call: &ToolCall,
    session: &AgentSession,
    app_handle: &AppHandle,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> OrchestratorToolResult {
    let message = call
        .arguments
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Working...");

    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::AgentNarration {
            session_id: session.id.0.clone(),
            conversation_id: session.conversation_id.clone(),
            thread_id: session.thread_id.clone(),
            content: message.to_string(),
            timestamp: Utc::now(),
        },
    );

    let _ = telemetry_repo.add_status_phrase(&session.id, message);

    OrchestratorToolResult {
        call_id: call.id.clone(),
        name: call.name.clone(),
        output: serde_json::json!({
            "success": true,
            "message": "Update sent"
        }),
        is_error: false,
    }
}

fn parse_status(
    status: &str,
) -> Option<(
    crate::core::agent::entities::TaskStatus,
    crate::core::agent::task::TaskStatus,
)> {
    match status {
        "pending" => Some((
            crate::core::agent::entities::TaskStatus::Pending,
            crate::core::agent::task::TaskStatus::Pending,
        )),
        "in_progress" => Some((
            crate::core::agent::entities::TaskStatus::InProgress,
            crate::core::agent::task::TaskStatus::InProgress,
        )),
        "completed" => Some((
            crate::core::agent::entities::TaskStatus::Completed,
            crate::core::agent::task::TaskStatus::Completed,
        )),
        "failed" => Some((
            crate::core::agent::entities::TaskStatus::Failed,
            crate::core::agent::task::TaskStatus::Failed,
        )),
        _ => None,
    }
}

fn validate_plan_subtasks(
    subtasks: &[serde_json::Value],
    requires_verification: bool,
) -> Result<(), String> {
    if subtasks.is_empty() {
        return Err("Plan must contain at least one subtask".to_string());
    }

    let actionable_verbs = [
        "analyze",
        "investigate",
        "identify",
        "understand",
        "trace",
        "inspect",
        "read",
        "search",
        "implement",
        "update",
        "refactor",
        "fix",
        "write",
        "create",
        "add",
        "remove",
        "rename",
        "run",
        "test",
        "verify",
        "build",
        "check",
        "debug",
        "review",
    ];
    let verification_terms = [
        "test",
        "verify",
        "validation",
        "regression",
        "check",
        "build",
        "compile",
        "lint",
    ];
    let mut seen = HashSet::new();
    let mut has_verification_step = false;

    for item in subtasks {
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Each subtask must have a non-empty title".to_string())?;
        let normalized = title.to_lowercase();
        if !seen.insert(normalized.clone()) {
            return Err(format!("Duplicate subtask title found: '{}'", title));
        }

        let first_word = normalized
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();
        let first_word = first_word
            .trim_matches(|c: char| c.is_ascii_punctuation())
            .trim_matches(|c: char| c.is_ascii_digit())
            .to_string();
        if !actionable_verbs.contains(&first_word.as_str()) {
            return Err(format!(
                "Subtask '{}' is not actionable; start titles with an action verb",
                title
            ));
        }

        if verification_terms
            .iter()
            .any(|term| normalized.contains(term))
        {
            has_verification_step = true;
        }
    }

    if requires_verification && !has_verification_step {
        return Err(
            "Code-change intent detected. Add a verification subtask (test/verify/build/check)."
                .to_string(),
        );
    }

    Ok(())
}

fn detect_code_change_intent(
    session: &AgentSession,
    task_id: &str,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> bool {
    let task_text = task_repo
        .get_session_tasks(&session.id.0)
        .ok()
        .and_then(|tasks| tasks.into_iter().find(|t| t.id == task_id))
        .map(|t| t.title.to_lowercase())
        .unwrap_or_default();

    let keywords = [
        "fix",
        "bug",
        "implement",
        "update",
        "write",
        "refactor",
        "create",
        "add",
        "remove",
        "rename",
        "build",
        "compile",
        "test",
    ];
    let heuristic_hit = keywords.iter().any(|k| task_text.contains(k));
    if heuristic_hit {
        return true;
    }

    telemetry_repo
        .get_tool_calls_for_session(&session.id)
        .ok()
        .map(|calls| {
            calls.into_iter().any(|c| {
                matches!(
                    c.tool_name.as_str(),
                    "fs_write_file" | "fs_delete_file" | "system_execute_command"
                ) && !c.is_error
            })
        })
        .unwrap_or(false)
}

fn has_subtask_evidence(
    session: &AgentSession,
    subtask_id: &str,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> bool {
    telemetry_repo
        .get_tool_calls_for_session(&session.id)
        .ok()
        .map(|calls| {
            calls.into_iter().any(|c| {
                c.subtask_id.as_deref() == Some(subtask_id)
                    && !c.is_error
                    && matches!(
                        c.tool_name.as_str(),
                        "fs_write_file" | "fs_delete_file" | "system_execute_command"
                    )
            })
        })
        .unwrap_or(false)
}

fn remember_subtask_rationale(subtask_id: &str, rationale: &str) {
    let map = SUBTASK_RATIONALES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut lock = map.lock().unwrap_or_else(|e| e.into_inner());
    lock.insert(subtask_id.to_string(), rationale.to_string());
}

fn has_subtask_rationale(subtask_id: &str) -> bool {
    let map = SUBTASK_RATIONALES.get_or_init(|| Mutex::new(HashMap::new()));
    let lock = map.lock().unwrap_or_else(|e| e.into_inner());
    lock.contains_key(subtask_id)
}

fn increment_subtask_failure_count(subtask_id: &str) -> u32 {
    let map = SUBTASK_FAILURE_COUNTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut lock = map.lock().unwrap_or_else(|e| e.into_inner());
    let entry = lock.entry(subtask_id.to_string()).or_insert(0);
    *entry += 1;
    *entry
}

fn mark_subtask_decomposed_once(subtask_id: &str) -> bool {
    let set = DECOMPOSED_SUBTASKS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut lock = set.lock().unwrap_or_else(|e| e.into_inner());
    if lock.contains(subtask_id) {
        return false;
    }
    lock.insert(subtask_id.to_string());
    true
}

fn auto_decompose_failed_subtask(
    session: &AgentSession,
    task_id: &str,
    failed_subtask: &crate::core::agent::task::SubTask,
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
) {
    let existing = task_repo.get_subtasks(task_id).ok().unwrap_or_default();
    let max_order = existing.iter().map(|s| s.order).max().unwrap_or(0);
    let base_order = max_order + 1;
    let titles = [
        format!("Analyze failure cause: {}", failed_subtask.title),
        format!("Implement focused fix: {}", failed_subtask.title),
        format!("Verify outcome: {}", failed_subtask.title),
    ];

    let mut created_ids: Vec<String> = Vec::new();
    for (idx, title) in titles.iter().enumerate() {
        let core_subtask = crate::core::agent::entities::AgentTask {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session.id.0.clone(),
            parent_task_id: Some(task_id.to_string()),
            title: title.clone(),
            description: Some(format!(
                "Auto-decomposed from failed subtask '{}' ({})",
                failed_subtask.title, failed_subtask.id
            )),
            status: crate::core::agent::entities::TaskStatus::Pending,
            order: base_order + idx as i32,
            mode: Some(crate::core::agent::entities::TaskMode::FullAccess),
            needs_review: false,
        };
        if let Err(e) = task_repo.create_subtask(task_id, &core_subtask) {
            warn!("[AgentRuntime] Failed to auto-create decomposed subtask: {e}");
            continue;
        }

        let mut event_subtask = crate::core::agent::task::SubTask::new(
            task_id.to_string(),
            core_subtask.title.clone(),
            crate::core::agent::task::SubTaskMode::FullAccess,
            core_subtask.order,
        );
        event_subtask.id = core_subtask.id.clone();
        event_subtask.description = core_subtask.description.clone();

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::SubtaskCreated {
                session_id: session.id.0.clone(),
                subtask: event_subtask,
            },
        );
        created_ids.push(core_subtask.id.clone());
    }

    if let Some(first_id) = created_ids.first() {
        let _ = task_repo.update_subtask_status(
            first_id,
            crate::core::agent::entities::TaskStatus::InProgress,
        );
        let refreshed = task_repo.get_subtasks(task_id).ok().unwrap_or_default();
        if let Some(first) = refreshed.iter().find(|s| &s.id == first_id).cloned() {
            let mut event = core_task_to_event_subtask(first, task_id);
            event.status = crate::core::agent::task::TaskStatus::InProgress;
            event.started_at = Some(Utc::now());
            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::SubtaskUpdated {
                    session_id: session.id.0.clone(),
                    subtask: event,
                },
            );
        }
    }
}

/// Convert a database agent task entity to an event subtask.
///
/// This is used when emitting subtask-related events to the frontend.
pub fn core_task_to_event_subtask(
    task: crate::core::agent::entities::AgentTask,
    task_id: &str,
) -> crate::core::agent::task::SubTask {
    let mode = match task
        .mode
        .unwrap_or(crate::core::agent::entities::TaskMode::FullAccess)
    {
        crate::core::agent::entities::TaskMode::Read => crate::core::agent::task::SubTaskMode::Read,
        crate::core::agent::entities::TaskMode::Write => {
            crate::core::agent::task::SubTaskMode::Write
        }
        crate::core::agent::entities::TaskMode::Execute => {
            crate::core::agent::task::SubTaskMode::Execute
        }
        crate::core::agent::entities::TaskMode::FullAccess => {
            crate::core::agent::task::SubTaskMode::FullAccess
        }
    };

    // Convert status from core entity to runtime type
    let status = match task.status {
        crate::core::agent::entities::TaskStatus::Pending => {
            crate::core::agent::task::TaskStatus::Pending
        }
        crate::core::agent::entities::TaskStatus::InProgress => {
            crate::core::agent::task::TaskStatus::InProgress
        }
        crate::core::agent::entities::TaskStatus::Completed => {
            crate::core::agent::task::TaskStatus::Completed
        }
        crate::core::agent::entities::TaskStatus::Failed => {
            crate::core::agent::task::TaskStatus::Failed
        }
        crate::core::agent::entities::TaskStatus::Cancelled => {
            crate::core::agent::task::TaskStatus::Cancelled
        }
        crate::core::agent::entities::TaskStatus::Aborted => {
            crate::core::agent::task::TaskStatus::Cancelled
        }
    };

    crate::core::agent::task::SubTask {
        id: task.id,
        task_id: task.parent_task_id.unwrap_or_else(|| task_id.to_string()),
        title: task.title,
        description: task.description,
        mode,
        status,
        order: task.order,
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
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
        "fs_write_file" => {
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
mod tests {
    use super::*;
    use crate::core::agent::session::SessionId;
    use crate::core::agent::types::TrustMode;
    use serde_json::json;

    #[test]
    fn test_pending_action_ext_with_preview() {
        let pending = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test action".to_string(),
            json!({}),
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

    #[test]
    fn test_pending_action_creation() {
        let session_id = SessionId("session-123".to_string());
        let action = PendingAction::new(
            session_id.clone(),
            "fs_write_file".to_string(),
            ActionType::WriteFile {
                path: "/tmp/test.txt".to_string(),
                size_bytes: 100,
            },
            "Write a test file".to_string(),
            json!({"path": "/tmp/test.txt", "content": "hello"}),
        );

        assert_eq!(action.session_id.0, "session-123");
        assert_eq!(action.tool_name, "fs_write_file");
        assert!(action.preview.is_none());
    }

    #[test]
    fn test_action_type_read_file() {
        let action = ActionType::ReadFile {
            path: "/path/to/file.rs".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("read_file"));

        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::ReadFile { path } => {
                assert_eq!(path, "/path/to/file.rs");
            }
            _ => panic!("Expected ReadFile"),
        }
    }

    #[test]
    fn test_action_type_write_file() {
        let action = ActionType::WriteFile {
            path: "/path/to/file.rs".to_string(),
            size_bytes: 1024,
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("write_file"));
        assert!(serialized.contains("1024"));
    }

    #[test]
    fn test_action_type_delete_file() {
        let action = ActionType::DeleteFile {
            path: "/tmp/old_file.txt".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("delete_file"));
    }

    #[test]
    fn test_action_type_execute_command() {
        let action = ActionType::ExecuteCommand {
            command: "cargo test".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("execute_command"));
        assert!(serialized.contains("cargo test"));
    }

    #[test]
    fn test_action_type_web_request() {
        let action = ActionType::WebRequest {
            url: "https://example.com".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("web_request"));
        assert!(serialized.contains("https://example.com"));
    }

    #[test]
    fn test_action_preview_creation() {
        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- old\n+++ new".to_string(),
            language: Some("typescript".to_string()),
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(serialized.contains("diff"));
        assert!(serialized.contains("typescript"));
    }

    #[test]
    fn test_action_preview_without_language() {
        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Plain text preview".to_string(),
            language: None,
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(serialized.contains("text"));

        let deserialized: ActionPreview = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.language.is_none());
    }

    #[test]
    fn test_pending_action_with_multiple_previews() {
        let session_id = SessionId("test".to_string());
        let base = PendingAction::new(
            session_id,
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test".to_string(),
            json!({}),
        );

        let preview1 = ActionPreview {
            content_type: "diff".to_string(),
            content: "diff content".to_string(),
            language: Some("rust".to_string()),
        };

        let with_preview1 = base.with_preview(preview1);
        assert!(with_preview1.preview.is_some());
        assert_eq!(
            with_preview1.preview.unwrap().language,
            Some("rust".to_string())
        );
    }

    #[test]
    fn test_permission_level_serialization() {
        let levels = vec![
            crate::core::agent::types::PermissionLevel::AutoRun,
            crate::core::agent::types::PermissionLevel::ApprovalRequired,
            crate::core::agent::types::PermissionLevel::Disabled,
        ];

        for level in levels {
            let serialized = serde_json::to_string(&level).unwrap();
            let deserialized: crate::core::agent::types::PermissionLevel =
                serde_json::from_str(&serialized).unwrap();
            assert_eq!(level, deserialized);
        }
    }

    #[test]
    fn test_trust_mode_serialization() {
        let modes = vec![
            TrustMode::Trusted,
            TrustMode::Untrusted,
            TrustMode::Restricted,
        ];

        for mode in modes {
            let serialized = serde_json::to_string(&mode).unwrap();
            let deserialized: TrustMode = serde_json::from_str(&serialized).unwrap();
            assert_eq!(mode, deserialized);
        }
    }

    // === Permission Level Tests ===

    #[test]
    fn test_permission_level_auto_run() {
        let level = crate::core::agent::types::PermissionLevel::AutoRun;
        let serialized = serde_json::to_string(&level).unwrap();
        assert!(serialized.contains("auto_run"));
    }

    #[test]
    fn test_permission_level_approval_required() {
        let level = crate::core::agent::types::PermissionLevel::ApprovalRequired;
        let serialized = serde_json::to_string(&level).unwrap();
        assert!(serialized.contains("approval_required"));
    }

    #[test]
    fn test_permission_level_disabled() {
        let level = crate::core::agent::types::PermissionLevel::Disabled;
        let serialized = serde_json::to_string(&level).unwrap();
        assert!(serialized.contains("disabled"));
    }

    // === PendingAction Tests ===

    #[test]
    fn test_pending_action_with_all_fields() {
        let action = PendingAction::new(
            SessionId("session-123".to_string()),
            "fs_write_file".to_string(),
            ActionType::WriteFile {
                path: "/tmp/test.txt".to_string(),
                size_bytes: 100,
            },
            "Write test file".to_string(),
            json!({"path": "/tmp/test.txt", "content": "hello"}),
        );

        assert_eq!(action.session_id.0, "session-123");
        assert_eq!(action.tool_name, "fs_write_file");
        assert!(action.preview.is_none());
    }

    #[test]
    fn test_pending_action_id_format() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test".to_string(),
            json!({}),
        );

        // ID should be a valid UUID string
        assert!(!action.id.is_empty());
        assert!(action.id.len() > 10);
    }

    #[test]
    fn test_pending_action_serialization() {
        let action = PendingAction::new(
            SessionId("session-456".to_string()),
            "delete_file".to_string(),
            ActionType::DeleteFile {
                path: "/tmp/old.txt".to_string(),
            },
            "Delete old file".to_string(),
            json!({"path": "/tmp/old.txt"}),
        );

        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: PendingAction = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.session_id.0, "session-456");
        assert_eq!(deserialized.tool_name, "delete_file");
    }

    #[test]
    fn test_pending_action_with_preview_serialization() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "write_file".to_string(),
            ActionType::WriteFile {
                path: "/tmp/test.txt".to_string(),
                size_bytes: 50,
            },
            "Write file".to_string(),
            json!({}),
        );

        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- old\n+++ new".to_string(),
            language: Some("rust".to_string()),
        };

        let with_preview = action.with_preview(preview);

        let serialized = serde_json::to_string(&with_preview).unwrap();
        let deserialized: PendingAction = serde_json::from_str(&serialized).unwrap();

        assert!(deserialized.preview.is_some());
        assert_eq!(deserialized.preview.unwrap().content_type, "diff");
    }

    #[test]
    fn test_pending_action_multiple_previews_replace() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "tool".to_string(),
            ActionType::ReadFile {
                path: "file.txt".to_string(),
            },
            "Read file".to_string(),
            json!({}),
        );

        let preview1 = ActionPreview {
            content_type: "text".to_string(),
            content: "First preview".to_string(),
            language: None,
        };

        let preview2 = ActionPreview {
            content_type: "diff".to_string(),
            content: "Second preview".to_string(),
            language: Some("typescript".to_string()),
        };

        let with_first = action.with_preview(preview1);
        let with_second = with_first.with_preview(preview2);

        // Second preview should replace the first
        assert_eq!(with_second.preview.unwrap().content, "Second preview");
    }

    // === Action Type Comprehensive Tests ===

    #[test]
    fn test_action_type_read_file_fields() {
        let action = ActionType::ReadFile {
            path: "/home/user/project/src/main.rs".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("read_file"));
        assert!(serialized.contains("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_action_type_write_file_fields() {
        let action = ActionType::WriteFile {
            path: "/tmp/output.json".to_string(),
            size_bytes: 2048,
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("write_file"));
        assert!(serialized.contains("2048"));
    }

    #[test]
    fn test_action_type_delete_file_fields() {
        let action = ActionType::DeleteFile {
            path: "/tmp/unused.tmp".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("delete_file"));
    }

    #[test]
    fn test_action_type_execute_command_fields() {
        let action = ActionType::ExecuteCommand {
            command: "cargo test --release".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("execute_command"));
        assert!(serialized.contains("cargo test --release"));
    }

    #[test]
    fn test_action_type_web_request_fields() {
        let action = ActionType::WebRequest {
            url: "https://api.example.com/v1/users".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("web_request"));
        assert!(serialized.contains("https://api.example.com"));
    }

    #[test]
    fn test_action_type_all_variants_roundtrip() {
        let actions = vec![
            ActionType::ReadFile {
                path: "/path/to/file.rs".to_string(),
            },
            ActionType::WriteFile {
                path: "/path/to/file.rs".to_string(),
                size_bytes: 100,
            },
            ActionType::DeleteFile {
                path: "/path/to/file.rs".to_string(),
            },
            ActionType::ExecuteCommand {
                command: "cargo build".to_string(),
            },
            ActionType::WebRequest {
                url: "https://example.com".to_string(),
            },
        ];

        for action in actions {
            let serialized = serde_json::to_string(&action).unwrap();
            let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();

            match (&action, &deserialized) {
                (ActionType::ReadFile { path: p1 }, ActionType::ReadFile { path: p2 }) => {
                    assert_eq!(p1, p2);
                }
                (
                    ActionType::WriteFile {
                        path: p1,
                        size_bytes: s1,
                    },
                    ActionType::WriteFile {
                        path: p2,
                        size_bytes: s2,
                    },
                ) => {
                    assert_eq!(p1, p2);
                    assert_eq!(s1, s2);
                }
                (ActionType::DeleteFile { path: p1 }, ActionType::DeleteFile { path: p2 }) => {
                    assert_eq!(p1, p2);
                }
                (
                    ActionType::ExecuteCommand { command: c1 },
                    ActionType::ExecuteCommand { command: c2 },
                ) => {
                    assert_eq!(c1, c2);
                }
                (ActionType::WebRequest { url: u1 }, ActionType::WebRequest { url: u2 }) => {
                    assert_eq!(u1, u2);
                }
                _ => panic!("Action types don't match"),
            }
        }
    }

    #[test]
    fn test_runtime_session_state_from_core_processing_maps_to_running() {
        let runtime = runtime_session_state_from_core(
            &crate::core::agent::entities::SessionState::Processing {
                started_at: Utc::now(),
            },
        );

        match runtime {
            crate::core::agent::session::SessionState::Running { .. } => {}
            other => panic!("Expected Running state, got {:?}", other),
        }
    }
    // === ActionPreview Tests ===

    #[test]
    fn test_action_preview_all_fields() {
        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-old\n+new".to_string(),
            language: Some("rust".to_string()),
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(serialized.contains("diff"));
        assert!(serialized.contains("rust"));
    }

    #[test]
    fn test_action_preview_roundtrip() {
        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Plain text content".to_string(),
            language: None,
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        let deserialized: ActionPreview = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.content_type, "text");
        assert_eq!(deserialized.content, "Plain text content");
        assert!(deserialized.language.is_none());
    }

    #[test]
    fn test_action_preview_with_language_none() {
        let previews = vec![
            ActionPreview {
                content_type: "text".to_string(),
                content: "content".to_string(),
                language: None,
            },
            ActionPreview {
                content_type: "json".to_string(),
                content: "{}".to_string(),
                language: None,
            },
        ];

        for preview in previews {
            assert!(preview.language.is_none());
        }
    }

    #[test]
    fn test_action_preview_with_various_languages() {
        let languages = vec![
            ("rust", "diff"),
            ("typescript", "diff"),
            ("python", "diff"),
            ("javascript", "diff"),
            ("json", "json"),
            ("html", "html"),
        ];

        for (lang, content_type) in languages {
            let preview = ActionPreview {
                content_type: content_type.to_string(),
                content: format!("{} content", lang),
                language: Some(lang.to_string()),
            };

            assert_eq!(preview.language.unwrap(), lang);
        }
    }

    // === PendingActionExt Trait Tests ===

    #[test]
    fn test_pending_action_ext_trait_available() {
        // Test that the PendingActionExt trait works on PendingAction
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test".to_string(),
            json!({}),
        );

        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Preview".to_string(),
            language: None,
        };

        let _modified = action.with_preview(preview);
    }

    #[test]
    fn test_pending_action_ext_preserves_other_fields() {
        let action = PendingAction::new(
            SessionId("session-xyz".to_string()),
            "custom_tool".to_string(),
            ActionType::ExecuteCommand {
                command: "npm install".to_string(),
            },
            "Install packages".to_string(),
            json!({"command": "npm install"}),
        );

        let original_id = action.id.clone();
        let original_session_id = action.session_id.0.clone();
        let original_tool_name = action.tool_name.clone();

        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Will run: npm install".to_string(),
            language: None,
        };

        let modified = action.with_preview(preview);

        assert_eq!(modified.id, original_id);
        assert_eq!(modified.session_id.0, original_session_id);
        assert_eq!(modified.tool_name, original_tool_name);
        assert!(modified.preview.is_some());
    }

    // === Approval Timeout Tests ===

    #[test]
    fn test_approval_timeout_constant() {
        // The approval timeout should be set to 300 seconds (5 minutes)
        assert_eq!(APPROVAL_TIMEOUT_SECS, 300);
    }

    #[test]
    fn test_approval_timeout_reasonable() {
        // Approval timeout should be at least 1 minute and at most 10 minutes
        assert!(APPROVAL_TIMEOUT_SECS >= 60);
        assert!(APPROVAL_TIMEOUT_SECS <= 600);
    }

    // === TrustMode and PermissionLevel Integration Tests ===

    #[test]
    fn test_trust_mode_affects_permission_level() {
        // Verify that different trust modes can be serialized
        let modes = vec![
            TrustMode::Trusted,
            TrustMode::Untrusted,
            TrustMode::Restricted,
        ];

        for mode in modes {
            let serialized = serde_json::to_string(&mode).unwrap();
            let _deserialized: TrustMode = serde_json::from_str(&serialized).unwrap();
        }
    }

    #[test]
    fn test_action_type_edge_cases() {
        // Test with empty path
        let action = ActionType::ReadFile {
            path: "".to_string(),
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::ReadFile { path } => assert_eq!(path, ""),
            _ => panic!("Expected ReadFile"),
        }

        // Test with very long path
        let long_path = "/".repeat(100) + "file.txt";
        let action = ActionType::WriteFile {
            path: long_path.clone(),
            size_bytes: 0,
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::WriteFile { path, .. } => assert_eq!(path, long_path),
            _ => panic!("Expected WriteFile"),
        }

        // Test with zero size
        let action = ActionType::WriteFile {
            path: "empty.txt".to_string(),
            size_bytes: 0,
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::WriteFile { size_bytes, .. } => assert_eq!(size_bytes, 0),
            _ => panic!("Expected WriteFile"),
        }

        // Test with large size
        let action = ActionType::WriteFile {
            path: "large.bin".to_string(),
            size_bytes: 1_000_000_000,
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::WriteFile { size_bytes, .. } => assert_eq!(size_bytes, 1_000_000_000),
            _ => panic!("Expected WriteFile"),
        }
    }

    #[test]
    fn test_action_preview_with_special_characters() {
        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- old\n+++ new\n@@ -1,1 +1,1 @@\n-Hello, 世界\n+Hello, World\n".to_string(),
            language: Some("rust".to_string()),
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        let deserialized: ActionPreview = serde_json::from_str(&serialized).unwrap();

        assert!(deserialized.content.contains("世界"));
    }

    #[test]
    fn test_pending_action_with_complex_arguments() {
        let complex_args = json!({
            "files": ["file1.rs", "file2.rs", "file3.rs"],
            "options": {
                "recursive": true,
                "force": false
            },
            "filters": [
                {"type": "extension", "value": ".rs"},
                {"type": "size", "operator": ">", "value": 100}
            ]
        });

        let action = PendingAction::new(
            SessionId("test".to_string()),
            "complex_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Complex operation".to_string(),
            complex_args,
        );

        assert!(action.arguments["files"].is_array());
        assert_eq!(action.arguments["files"].as_array().unwrap().len(), 3);
    }
}
