//! Permission handling and tool execution for agent runtime.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use chrono::Utc;
use log::{info, warn};
use tauri::{AppHandle, Emitter};

use super::tool_feedback::{build_tool_call_feedback, build_tool_result_feedback};
use super::types::AgentRuntimeError;
use super::{generate_diff_from_content, infer_action_type};
use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::{AgentSession, PendingAction};
use crate::core::agent::types::ActionPreview;
#[allow(unused_imports)]
use crate::core::agent::types::ActionType;
#[allow(unused_imports)]
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
) -> Result<Vec<OrchestratorToolResult>, AgentRuntimeError> {
    let mut results = Vec::with_capacity(tool_calls.len());

    for call in tool_calls {
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
                        app_handle,
                        tool_registry,
                        file_snapshot_registry,
                        session_repo,
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
async fn execute_tool(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    session_repo: &dyn SessionRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
) -> OrchestratorToolResult {
    if let Some(result) = handle_agent_tool_call(
        call,
        session,
        task_id,
        app_handle,
        session_repo,
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
                            app_handle,
                            tool_registry,
                            file_snapshot_registry,
                            session_repo,
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

fn handle_agent_tool_call(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    session_repo: &dyn SessionRepository,
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
            session_repo,
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

    // Workstream H.2.b — validate and persist the optional top-level
    // `framing` record before any subtask is created. Malformed or
    // underspecified framing is rejected at planning time so the
    // agent cannot silently fall back to the genre default.
    let framing = match parse_and_validate_framing(call.arguments.get("framing")) {
        Ok(framing) => framing,
        Err(msg) => {
            return OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": msg,
                    "requires_replan": true,
                }),
                is_error: true,
            };
        }
    };
    if let Some(framing) = &framing {
        if let Err(e) = task_repo.update_task_framing(task_id, Some(framing.clone())) {
            warn!("[AgentRuntime] Failed to persist task framing: {e}");
            // Non-fatal — planning still proceeds, but surface the issue
            // so the trace review can catch it. The runtime reads the
            // framing back from the task repo on every step, so a
            // persistence failure means the default framing is used.
        }
    }

    let existing = task_repo.get_subtasks(task_id).ok().unwrap_or_default();

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

    // Workstream α.1 — when the auto-planner at task creation has already
    // persisted a contract-less skeleton plan, merge the model's richer
    // plan into it by order rather than early-returning. See
    // `merge_model_plan_into_existing` for the per-slot rules, Completed
    // subtask guard, append logic, and the `requires_replan` retry hook
    // that preserves the single-retry window managed by
    // `PLAN_RETRY_TRACKER`.
    if !existing.is_empty() {
        return merge_model_plan_into_existing(
            call,
            session,
            task_id,
            subtasks,
            &existing,
            app_handle,
            task_repo,
        );
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

        // Workstream B.2 — parse the optional `contract` field per
        // subtask. Malformed contracts fail the plan with a clear
        // message so the planner can fix the declaration. Absent
        // contracts preserve pre-B behavior (scope-drift detection
        // short-circuits for that subtask).
        let contract = match item.get("contract") {
            Some(raw) if !raw.is_null() => {
                match serde_json::from_value::<crate::core::agent::task::SubTaskContract>(
                    raw.clone(),
                ) {
                    Ok(contract) if !contract.is_empty() => Some(contract),
                    Ok(_empty) => None,
                    Err(e) => {
                        return OrchestratorToolResult {
                            call_id: call.id.clone(),
                            name: call.name.clone(),
                            output: serde_json::json!({
                                "error": format!(
                                    "Subtask {idx} has a malformed `contract` field: {e}"
                                ),
                            }),
                            is_error: true,
                        };
                    }
                }
            }
            _ => None,
        };

        let mut subtask = crate::core::agent::task::SubTask::new(
            task_id.to_string(),
            title.clone(),
            crate::core::agent::task::SubTaskMode::FullAccess,
            idx as i32,
        );
        if let Some(ref desc) = description {
            subtask.description = Some(desc.clone());
        }
        if let Some(contract) = contract {
            subtask.contract = Some(contract);
        }

        if let Err(e) = task_repo.create_subtask(&subtask) {
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

        let mut event_subtask = subtask.clone();
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
                crate::core::agent::task::TaskStatus::InProgress,
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

/// Merge a model-declared plan into an already-persisted skeleton plan
/// (Workstream α.1).
///
/// The bootstrap auto-planner creates a contract-less, FullAccess-mode
/// skeleton at task-creation time. Before α.1, the tool-planner
/// early-returned "Subtasks already planned" whenever that skeleton
/// existed, which silently discarded the model's richer plan — every
/// contract and framing declaration it emitted was dropped, and the
/// Phases B/F/G features that depend on contracts degenerated to
/// no-ops in production.
///
/// The merge rules, applied iteratively over the model's subtasks:
/// - For each `(order, item)` the model declares, locate the existing
///   subtask at the same `order`.
/// - If found and the existing status is `Pending` or `InProgress`:
///   attach a non-empty contract, update a non-empty and differing
///   description, update a non-empty and differing title, and update
///   the mode when declared and parseable. Each column update routes
///   through [`TaskRepository::refine_subtask`].
/// - If found and status is `Completed` and the model's counterpart
///   differs materially (title/description/contract), return a
///   structured `requires_replan: true` error so
///   `PLAN_RETRY_TRACKER` catches the single-retry window. Completed
///   work is never rewritten.
/// - If the model declared more subtasks than the skeleton carries,
///   append the extras at the tail — but only when no existing
///   subtask is `Completed`, so ordering integrity is preserved.
/// - If the model declared fewer subtasks, ignore the shortfall.
///   The skeleton is not deleted; the extra persisted slots stay
///   Pending until the model later refines them or
///   `agent_advance_to_next_subtask` walks past them.
///
/// The contract parsing logic mirrors the "create from scratch" path
/// so scope-drift detection sees the same shapes regardless of which
/// branch persisted the subtask. Malformed contracts fail the tool
/// call with a clear message so the planner can fix the declaration.
fn merge_model_plan_into_existing(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    model_plan: &[serde_json::Value],
    existing: &[crate::core::agent::task::SubTask],
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
) -> OrchestratorToolResult {
    use crate::core::agent::repositories::SubTaskRefinement;
    use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};

    let any_completed = existing
        .iter()
        .any(|st| st.status == TaskStatus::Completed);

    let mut refined = 0usize;
    let mut appended = 0usize;

    for (idx, item) in model_plan.iter().enumerate() {
        let model_title = match item.get("title").and_then(|v| v.as_str()) {
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

        let model_description = item
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string);

        let model_contract = match item.get("contract") {
            Some(raw) if !raw.is_null() => {
                match serde_json::from_value::<crate::core::agent::task::SubTaskContract>(
                    raw.clone(),
                ) {
                    Ok(contract) if !contract.is_empty() => Some(contract),
                    Ok(_empty) => None,
                    Err(e) => {
                        return OrchestratorToolResult {
                            call_id: call.id.clone(),
                            name: call.name.clone(),
                            output: serde_json::json!({
                                "error": format!(
                                    "Subtask {idx} has a malformed `contract` field: {e}"
                                ),
                            }),
                            is_error: true,
                        };
                    }
                }
            }
            _ => None,
        };

        let model_mode: Option<SubTaskMode> = item.get("mode").and_then(|v| {
            serde_json::from_value::<SubTaskMode>(v.clone()).ok()
        });

        // The persisted skeleton keys by `order`. The tool-planner
        // always emits dense orders 0..N (validated earlier in
        // `validate_plan_subtasks`), so the enumeration index and the
        // declared order agree unless the model intentionally skips.
        let target_order = item
            .get("order")
            .and_then(serde_json::Value::as_i64)
            .map_or(idx as i32, |v| v as i32);

        let existing_slot = existing
            .iter()
            .find(|st| st.order == target_order);

        match existing_slot {
            Some(slot) if slot.status == TaskStatus::Completed => {
                let title_differs = slot.title.trim() != model_title.as_str();
                let description_differs = match (&slot.description, &model_description) {
                    (Some(a), Some(b)) => a.trim() != b.trim(),
                    (None, Some(_)) => true,
                    _ => false,
                };
                let contract_differs = match (&slot.contract, &model_contract) {
                    (Some(a), Some(b)) => a != b,
                    (None, Some(_)) => true,
                    _ => false,
                };
                if title_differs || description_differs || contract_differs {
                    return OrchestratorToolResult {
                        call_id: call.id.clone(),
                        name: call.name.clone(),
                        output: serde_json::json!({
                            "error": "Completed subtasks cannot be redefined",
                            "requires_replan": true,
                            "feedback": "Completed subtasks cannot be redefined. Reconcile your plan with the session state."
                        }),
                        is_error: true,
                    };
                }
                // Identical shape — nothing to refine.
            }
            Some(slot) => {
                // Pending or InProgress — apply "set-if-different / non-empty".
                let mut refinement = SubTaskRefinement::default();
                if slot.title.trim() != model_title.as_str() {
                    refinement.title = Some(model_title.clone());
                }
                if let Some(new_desc) = model_description.as_ref() {
                    let current = slot.description.as_deref().unwrap_or("");
                    if current.trim() != new_desc.trim() {
                        refinement.description = Some(new_desc.clone());
                    }
                }
                if let Some(new_mode) = model_mode.as_ref() {
                    if &slot.mode != new_mode {
                        refinement.mode = Some(new_mode.clone());
                    }
                }
                if let Some(new_contract) = model_contract.as_ref() {
                    let current = slot.contract.as_ref();
                    if current != Some(new_contract) {
                        refinement.contract = Some(new_contract.clone());
                    }
                }

                if !refinement.is_empty() {
                    if let Err(e) = task_repo.refine_subtask(&slot.id, refinement) {
                        warn!(
                            "[AgentRuntime] Failed to refine subtask {sid}: {e}",
                            sid = slot.id,
                        );
                        return OrchestratorToolResult {
                            call_id: call.id.clone(),
                            name: call.name.clone(),
                            output: serde_json::json!({
                                "error": "Failed to refine subtask"
                            }),
                            is_error: true,
                        };
                    }
                    refined += 1;
                }

                // Rebroadcast the refined subtask to the frontend so
                // the UI can attach the new contract/mode/description
                // without waiting for the next status transition.
                let mut event_subtask = slot.clone();
                event_subtask.title = model_title.clone();
                if let Some(desc) = model_description.as_ref() {
                    event_subtask.description = Some(desc.clone());
                }
                if let Some(mode) = model_mode.as_ref() {
                    event_subtask.mode = mode.clone();
                }
                if let Some(contract) = model_contract.as_ref() {
                    event_subtask.contract = Some(contract.clone());
                }
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::SubtaskUpdated {
                        session_id: session.id.0.clone(),
                        subtask: event_subtask,
                    },
                );
            }
            None => {
                // The model declared more subtasks than exist. Appending is
                // only safe when no pre-existing subtask is Completed — if
                // any is, the model is fighting with the persisted
                // timeline and must reconcile.
                if any_completed {
                    return OrchestratorToolResult {
                        call_id: call.id.clone(),
                        name: call.name.clone(),
                        output: serde_json::json!({
                            "error": "Cannot append subtasks while earlier subtasks are Completed",
                            "requires_replan": true,
                            "feedback": "Completed subtasks cannot be redefined. Reconcile your plan with the session state."
                        }),
                        is_error: true,
                    };
                }
                let append_order = existing
                    .iter()
                    .map(|st| st.order)
                    .max()
                    .map_or(target_order, |max| {
                        std::cmp::max(max + 1 + appended as i32, target_order)
                    });
                let mut appended_subtask = SubTask::new(
                    task_id.to_string(),
                    model_title.clone(),
                    model_mode.clone().unwrap_or(SubTaskMode::FullAccess),
                    append_order,
                );
                if let Some(desc) = model_description.as_ref() {
                    appended_subtask.description = Some(desc.clone());
                }
                if let Some(contract) = model_contract.as_ref() {
                    appended_subtask.contract = Some(contract.clone());
                }
                if let Err(e) = task_repo.create_subtask(&appended_subtask) {
                    warn!(
                        "[AgentRuntime] Failed to append subtask during merge: {e}"
                    );
                    return OrchestratorToolResult {
                        call_id: call.id.clone(),
                        name: call.name.clone(),
                        output: serde_json::json!({
                            "error": "Failed to append subtask"
                        }),
                        is_error: true,
                    };
                }
                appended += 1;
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::SubtaskCreated {
                        session_id: session.id.0.clone(),
                        subtask: appended_subtask,
                    },
                );
            }
        }
    }

    OrchestratorToolResult {
        call_id: call.id.clone(),
        name: call.name.clone(),
        output: serde_json::json!({
            "success": true,
            "message": format!(
                "Merged model plan into existing subtasks (refined {refined}, appended {appended})"
            ),
            "refined": refined,
            "appended": appended,
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
        .position(|st| st.status == crate::core::agent::task::TaskStatus::InProgress);

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

    // Workstream F.1 — when the current subtask declared a contract,
    // every `expected_artifacts` pattern must resolve (on disk, or in
    // the files-touched overlay as fallback) before the handler may
    // transition to `Completed`. Absent/empty contracts preserve
    // pre-B behavior: the handler advances on evidence alone.
    if let Some(ref contract) = current.contract {
        if !contract.is_empty() {
            if let Err(failure) =
                verify_contract_artifacts(contract, session, telemetry_repo)
            {
                return failure.into_error_result(call);
            }
        }
    }

    if let Err(e) = task_repo.update_subtask_status(
        &current.id,
        crate::core::agent::task::TaskStatus::Completed,
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
        .find(|st| st.status == crate::core::agent::task::TaskStatus::Pending)
        .cloned();

    if let Some(next) = next {
        if let Err(e) = task_repo.update_subtask_status(
            &next.id,
            crate::core::agent::task::TaskStatus::InProgress,
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
        task_repo.update_task_status(task_id, crate::core::agent::task::TaskStatus::Completed);
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
    session_repo: &dyn SessionRepository,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
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

    // Workstream F.1 — when the planner marks a subtask `Completed`,
    // the same contract verification that gates `advance` must apply.
    // `Failed` is explicitly allowed even when the contract is
    // unsatisfied so the model can cleanly signal it cannot produce
    // the declared artifacts (Workstream F.2 then flags the session
    // for review below). `Pending` and `InProgress` mutations skip
    // verification — they do not commit to completion.
    if matches!(
        core_status,
        crate::core::agent::task::TaskStatus::Completed
    ) {
        if let Some(ref contract) = current.contract {
            if !contract.is_empty() {
                if let Err(failure) =
                    verify_contract_artifacts(contract, session, telemetry_repo)
                {
                    return failure.into_error_result(call);
                }
            }
        }
    }

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

    // Workstream F.2 — when the model explicitly marks a subtask
    // `Failed`, flip the session's `needs_review` flag so the UI can
    // surface a post-run review affordance and the session record
    // persists the signal across reload.
    if matches!(
        event_status,
        crate::core::agent::task::TaskStatus::Failed
    ) {
        if let Err(e) = session_repo.set_needs_review(&session.id, true) {
            warn!("[AgentRuntime] Failed to set needs_review on session: {e}");
        }
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
    let intent =
        crate::infrastructure::agent::events::NarrationIntent::from_arg(
            call.arguments.get("intent").and_then(|v| v.as_str()),
        );

    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::AgentNarration {
            session_id: session.id.0.clone(),
            conversation_id: session.conversation_id.clone(),
            thread_id: session.thread_id.clone(),
            content: message.to_string(),
            intent,
            timestamp: Utc::now(),
        },
    );

    let _ = telemetry_repo.add_status_phrase(&session.id, message);

    // Workstream E.2 — a `question` intent records a pending
    // clarification for this session. The step-gate consults the
    // registry before the next LLM call and pauses until the user
    // replies (E.4) or the clarification timeout (E.3) fires.
    if matches!(
        intent,
        crate::infrastructure::agent::events::NarrationIntent::Question
    ) {
        crate::application::agent::clarification::record_pending_clarification(
            &session.id.0,
            message,
        );
    }

    OrchestratorToolResult {
        call_id: call.id.clone(),
        name: call.name.clone(),
        output: serde_json::json!({
            "success": true,
            "message": "Update sent",
            "intent": match intent {
                crate::infrastructure::agent::events::NarrationIntent::Narration => "narration",
                crate::infrastructure::agent::events::NarrationIntent::Question => "question",
            }
        }),
        is_error: false,
    }
}

/// Workstream F.1 — verify that a subtask's contract artifacts exist
/// before the handler allows a transition to `Completed`.
///
/// Ground truth is the filesystem: for each `ArtifactSpec` in the
/// contract, every file the glob resolves to must exist on disk. When
/// the session has no `workspace_root`, glob evaluation would be
/// ambiguous, so the verification falls back to the
/// `agent_files_touched` overlay and treats a pattern as satisfied when
/// any recorded file path matches it.
///
/// Returns `Ok(())` when every `path_pattern` resolves; otherwise
/// returns the list of unsatisfied patterns plus a de-duplicated list
/// of concrete missing paths (empty when the pattern had no match at
/// all).
fn verify_contract_artifacts(
    contract: &crate::core::agent::task::SubTaskContract,
    session: &AgentSession,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> Result<(), ContractVerificationFailure> {
    use globset::{Glob, GlobSetBuilder};

    let workspace_root: Option<&std::path::Path> =
        session.workspace_root.as_deref();

    // Build the overlay matcher once; used as fallback when the
    // filesystem check cannot evaluate (no workspace root) and as an
    // auxiliary signal when on-disk globbing fails.
    let touched_paths: Vec<String> = telemetry_repo
        .get_session_files(&session.id)
        .ok()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            let action = entry.action.to_ascii_lowercase();
            if matches!(action.as_str(), "write" | "delete") {
                Some(entry.file_path)
            } else {
                None
            }
        })
        .collect();

    let mut unsatisfied_patterns: Vec<String> = Vec::new();
    for spec in &contract.expected_artifacts {
        let pattern = spec.path_pattern.trim();
        if pattern.is_empty() {
            unsatisfied_patterns.push(spec.path_pattern.clone());
            continue;
        }
        if pattern_is_satisfied_on_disk(pattern, workspace_root) {
            continue;
        }
        if pattern_matches_touched_overlay(pattern, &touched_paths) {
            continue;
        }
        unsatisfied_patterns.push(spec.path_pattern.clone());
    }

    if unsatisfied_patterns.is_empty() {
        return Ok(());
    }

    // Provide the model with a concrete path list where possible so
    // the rejection message is actionable. Empty when patterns
    // genuinely produced zero matches.
    let mut missing_paths: Vec<String> = Vec::new();
    if let Some(root) = workspace_root {
        let mut builder = GlobSetBuilder::new();
        for pattern in &unsatisfied_patterns {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        if let Ok(matcher) = builder.build() {
            if matcher.len() > 0 {
                // Walk each pattern's literal-prefix parent when
                // evaluable. The globset crate does not provide a
                // walk-and-match primitive, so we surface pattern-level
                // info rather than false concrete paths; the model is
                // expected to produce the declared artifacts, not
                // inspect the workspace.
                let _ = root; // keep `root` binding intentional
                let _ = matcher;
            }
        }
    }

    for path in &touched_paths {
        // Surface touched paths that *almost* satisfy the contract — a
        // path recorded in `agent_files_touched` but that no
        // `expected_artifacts` glob accepted. This is the concrete
        // signal the model can act on.
        let mut builder = GlobSetBuilder::new();
        for pattern in &contract.expected_artifacts {
            if let Ok(glob) = Glob::new(&pattern.path_pattern) {
                builder.add(glob);
            }
        }
        if let Ok(matcher) = builder.build() {
            if !matcher.is_match(path) && !missing_paths.contains(path) {
                missing_paths.push(path.clone());
            }
        }
    }

    Err(ContractVerificationFailure {
        missing_patterns: unsatisfied_patterns,
        concrete_missing_paths: missing_paths,
    })
}

fn pattern_is_satisfied_on_disk(
    pattern: &str,
    workspace_root: Option<&std::path::Path>,
) -> bool {
    use globset::Glob;
    let Some(root) = workspace_root else {
        return false;
    };
    // Plain path (no glob metacharacters): short-circuit to `exists`.
    if !pattern.contains(['*', '?', '[', '{']) {
        return root.join(pattern).exists();
    }

    let glob = match Glob::new(pattern) {
        Ok(g) => g,
        Err(_) => return false,
    };
    let matcher = glob.compile_matcher();

    walk_workspace_for_match(root, &matcher)
}

/// Bounded workspace walk that returns `true` as soon as any file
/// matches the compiled glob. Depth-limited to keep accidental
/// submits against oversized workspaces cheap; the hard bound is
/// enforced by the max-entries cap below.
fn walk_workspace_for_match(
    root: &std::path::Path,
    matcher: &globset::GlobMatcher,
) -> bool {
    const MAX_ENTRIES: usize = 20_000;
    let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
    let mut seen = 0usize;
    while let Some(current) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in read_dir.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                return false;
            }
            let path = entry.path();
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let relative_str = relative.to_string_lossy();
            if matcher.is_match(relative_str.as_ref()) {
                return true;
            }
            if path.is_dir() {
                // Skip common heavy directories that are never
                // relevant to a subtask contract output.
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if matches!(
                        name,
                        ".git"
                            | "node_modules"
                            | "target"
                            | "dist"
                            | "build"
                            | ".next"
                            | ".svelte-kit"
                    ) {
                        continue;
                    }
                }
                stack.push(path);
            }
        }
    }
    false
}

fn pattern_matches_touched_overlay(pattern: &str, touched_paths: &[String]) -> bool {
    use globset::Glob;
    let Ok(glob) = Glob::new(pattern) else {
        return false;
    };
    let matcher = glob.compile_matcher();
    touched_paths.iter().any(|path| matcher.is_match(path))
}

/// Payload carried by `contract_unsatisfied` errors so the model sees
/// both the patterns that did not resolve and any concrete paths that
/// were written outside the contract (helpful when scope drift landed).
#[derive(Debug, Clone)]
struct ContractVerificationFailure {
    missing_patterns: Vec<String>,
    concrete_missing_paths: Vec<String>,
}

impl ContractVerificationFailure {
    fn into_error_result(self, call: &ToolCall) -> OrchestratorToolResult {
        let missing = self
            .missing_patterns
            .iter()
            .map(|p| serde_json::Value::String(p.clone()))
            .collect::<Vec<_>>();
        let concrete = self
            .concrete_missing_paths
            .iter()
            .map(|p| serde_json::Value::String(p.clone()))
            .collect::<Vec<_>>();
        OrchestratorToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            output: serde_json::json!({
                "error": "Cannot advance subtask: contract_unsatisfied. The subtask's expected outputs do not exist on disk or in the session's file-touch overlay.",
                "reason": "contract_unsatisfied",
                "missingPatterns": missing,
                "concreteMissingPaths": concrete,
            }),
            is_error: true,
        }
    }
}

fn parse_status(
    status: &str,
) -> Option<(
    crate::core::agent::task::TaskStatus,
    crate::core::agent::task::TaskStatus,
)> {
    match status {
        "pending" => Some((
            crate::core::agent::task::TaskStatus::Pending,
            crate::core::agent::task::TaskStatus::Pending,
        )),
        "in_progress" => Some((
            crate::core::agent::task::TaskStatus::InProgress,
            crate::core::agent::task::TaskStatus::InProgress,
        )),
        "completed" => Some((
            crate::core::agent::task::TaskStatus::Completed,
            crate::core::agent::task::TaskStatus::Completed,
        )),
        "failed" => Some((
            crate::core::agent::task::TaskStatus::Failed,
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

    // Workstream G.1 — plan-quality validation. These checks only
    // trigger when contracts are declared (overlap) or when the final
    // subtask's text + empty contract matches the no-op heuristic;
    // legacy plans without contracts fall through unchanged.
    detect_overlapping_contracts(subtasks)?;
    detect_noop_final_subtask(subtasks)?;

    Ok(())
}

/// Workstream G.1 — rejects plans where two subtasks declare
/// overlapping `contract.expected_artifacts.path_pattern` entries.
///
/// The check is intentionally conservative so ordinary plans with
/// neighbouring but distinct contracts are not rejected. Two patterns
/// are treated as overlapping when any of the following hold:
///
/// 1. They are byte-identical after trimming whitespace.
/// 2. Their literal path prefixes (everything before the first glob
///    metacharacter, trimmed of trailing separators and bare `*` or
///    `**` tokens) are in a proper prefix-of relationship aligned on
///    a `/` boundary (e.g. `docs/social/` is a prefix of
///    `docs/social/twitter/`).
/// 3. One pattern is literal (no glob metacharacters) and is matched
///    by the other pattern's compiled `GlobMatcher`.
///
/// Anything beyond these three rules is treated as non-overlapping to
/// avoid false-positive rejections.
fn detect_overlapping_contracts(subtasks: &[serde_json::Value]) -> Result<(), String> {
    // Gather (subtask_index, title, pattern) tuples for every declared
    // path_pattern on every subtask with a contract. Subtasks without
    // contracts or with empty expected_artifacts contribute nothing
    // and therefore cannot trigger overlap rejection.
    let mut patterns: Vec<(usize, String, String)> = Vec::new();
    for (idx, item) in subtasks.iter().enumerate() {
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let Some(contract) = item.get("contract") else {
            continue;
        };
        if contract.is_null() {
            continue;
        }
        // Accept both the schema-declared snake_case key and the
        // camelCase variant produced by serde on SubTaskContract so
        // we don't regress when either spelling appears.
        let artifacts = contract
            .get("expected_artifacts")
            .or_else(|| contract.get("expectedArtifacts"))
            .and_then(|v| v.as_array());
        let Some(artifacts) = artifacts else {
            continue;
        };
        for spec in artifacts {
            let pattern_raw = spec
                .get("path_pattern")
                .or_else(|| spec.get("pathPattern"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim();
            if pattern_raw.is_empty() {
                continue;
            }
            patterns.push((idx, title.clone(), pattern_raw.to_string()));
        }
    }

    if patterns.len() < 2 {
        return Ok(());
    }

    for i in 0..patterns.len() {
        for j in (i + 1)..patterns.len() {
            let (a_idx, a_title, a_pat) = &patterns[i];
            let (b_idx, b_title, b_pat) = &patterns[j];
            if a_idx == b_idx {
                // Two entries on the same subtask's contract are
                // permitted — they describe parallel outputs owned
                // by the same unit of work.
                continue;
            }
            if patterns_overlap(a_pat, b_pat) {
                return Err(format!(
                    "Subtask '{a_title}' (index {a_idx}) and subtask '{b_title}' (index {b_idx}) declare overlapping expected-artifact patterns ('{a_pat}' and '{b_pat}'). Each subtask must own a distinct output path; rewrite the contracts so each pattern covers exactly one subtask's outputs, or merge the subtasks."
                ));
            }
        }
    }

    Ok(())
}

/// Returns `true` when `a` and `b` overlap under the conservative
/// rules documented on [`detect_overlapping_contracts`]. Patterns
/// deemed unrelated return `false`.
fn patterns_overlap(a: &str, b: &str) -> bool {
    let a = a.trim();
    let b = b.trim();
    if a.is_empty() || b.is_empty() {
        return false;
    }
    // Rule 1 — byte identical.
    if a == b {
        return true;
    }

    // Rule 2 — literal-prefix relationship on `/` boundaries.
    let prefix_a = literal_prefix(a);
    let prefix_b = literal_prefix(b);
    if is_path_prefix_of(&prefix_a, &prefix_b) || is_path_prefix_of(&prefix_b, &prefix_a) {
        return true;
    }

    // Rule 3 — one side is literal (no glob metacharacters) and is
    // matched by the other's compiled matcher.
    if let Some(overlap) = literal_matches_glob(a, b) {
        return overlap;
    }
    if let Some(overlap) = literal_matches_glob(b, a) {
        return overlap;
    }

    false
}

/// Returns the literal directory prefix of a glob pattern — everything
/// up to (but not including) the first glob metacharacter, trimmed of
/// trailing separators. A pattern like `docs/social/*.md` yields
/// `docs/social`; a pattern like `*.md` yields an empty string.
fn literal_prefix(pattern: &str) -> String {
    let meta_idx = pattern
        .char_indices()
        .find(|(_, c)| matches!(*c, '*' | '?' | '[' | '{'))
        .map_or(pattern.len(), |(i, _)| i);
    pattern[..meta_idx]
        .trim_end_matches('/')
        .trim_end_matches('\\')
        .to_string()
}

/// Returns `true` when `parent` is a proper path prefix of `child`,
/// aligned on a `/` boundary. Empty `parent` never matches (the
/// check is meaningless for globs with no literal anchor).
fn is_path_prefix_of(parent: &str, child: &str) -> bool {
    if parent.is_empty() || child.is_empty() {
        return false;
    }
    if parent == child {
        // Pure equality is handled by Rule 1 already; return true
        // here so pattern forms like `docs` and `docs` (after
        // stripping trailing glob) are still flagged.
        return true;
    }
    if !child.starts_with(parent) {
        return false;
    }
    // Ensure the boundary is a path separator so `docs/a` isn't
    // flagged as a prefix of `docs/alpha`.
    let remainder = &child[parent.len()..];
    remainder.starts_with('/') || remainder.starts_with('\\')
}

/// Returns `Some(bool)` when `candidate` is a literal path and can be
/// tested against `pattern` as a glob; `None` when `candidate` has
/// glob metacharacters and therefore can't be used as a concrete-path
/// probe.
fn literal_matches_glob(candidate: &str, pattern: &str) -> Option<bool> {
    if candidate
        .chars()
        .any(|c| matches!(c, '*' | '?' | '[' | '{'))
    {
        return None;
    }
    let glob = globset::Glob::new(pattern).ok()?;
    let matcher = glob.compile_matcher();
    Some(matcher.is_match(candidate))
}

/// Workstream G.1 — rejects plans whose final subtask is a no-op
/// "generate documentation of prior subtasks" index with no contract.
///
/// The detector is deliberately narrow: it only fires on the **last**
/// subtask, requires both a lexical trigger *and* an absent/empty
/// contract, and lets any subtask with a declared contract through
/// untouched. This keeps legitimate final-step documentation subtasks
/// (which declare where they write) out of the reject set.
fn detect_noop_final_subtask(subtasks: &[serde_json::Value]) -> Result<(), String> {
    // Need at least two subtasks — a single-subtask plan cannot be a
    // "documentation of prior subtasks" redundancy.
    if subtasks.len() < 2 {
        return Ok(());
    }

    let Some(last) = subtasks.last() else {
        return Ok(());
    };

    // If the final subtask has a non-empty contract, trust the
    // declaration and do not reject — the contract itself justifies
    // the subtask's presence.
    let contract_is_trivial = match last.get("contract") {
        None => true,
        Some(v) if v.is_null() => true,
        Some(v) => {
            let artifacts = v
                .get("expected_artifacts")
                .or_else(|| v.get("expectedArtifacts"))
                .and_then(|a| a.as_array());
            match artifacts {
                Some(list) => list.is_empty(),
                None => true,
            }
        }
    };
    if !contract_is_trivial {
        return Ok(());
    }

    let title = last
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    let description = last
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    let haystack = format!("{title} {description}");

    // Triggers are fragments so inflected forms ("summarize",
    // "summarizing", "summary") all match a single stem. The
    // phrases are picked to cover the observed regression plus
    // close paraphrases without reaching into words that appear
    // in legitimate action-oriented subtasks.
    const TRIGGERS: &[&str] = &[
        "summariz",
        "index",
        "documentation of",
        "list of files",
        "recap",
        "overview of what",
        "document the above",
        "document what was",
    ];

    let matched = TRIGGERS.iter().find(|t| haystack.contains(**t));
    if let Some(trigger) = matched {
        let original_title = last
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("<untitled>")
            .to_string();
        return Err(format!(
            "Final subtask '{original_title}' looks like a no-op index of prior work (matched heuristic '{trigger}') and declares no output contract. Remove it, or replace it with a concrete artifact contract describing the index file it must produce."
        ));
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
        .and_then(|t| t.title.map(|title| title.to_lowercase()))
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
        let mut subtask = crate::core::agent::task::SubTask::new(
            task_id.to_string(),
            title.clone(),
            crate::core::agent::task::SubTaskMode::FullAccess,
            base_order + idx as i32,
        );
        subtask.description = Some(format!(
            "Auto-decomposed from failed subtask '{}' ({})",
            failed_subtask.title, failed_subtask.id
        ));

        if let Err(e) = task_repo.create_subtask(&subtask) {
            warn!("[AgentRuntime] Failed to auto-create decomposed subtask: {e}");
            continue;
        }

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::SubtaskCreated {
                session_id: session.id.0.clone(),
                subtask: subtask.clone(),
            },
        );
        created_ids.push(subtask.id);
    }

    if let Some(first_id) = created_ids.first() {
        let _ = task_repo.update_subtask_status(
            first_id,
            crate::core::agent::task::TaskStatus::InProgress,
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

/// Ensures a subtask has its `task_id` set, defaulting to the caller-supplied
/// parent when the field is empty. Post type-consolidation this is a thin
/// pass-through — the task repo already returns canonical [`SubTask`] values,
/// so no field translation is required. Kept as a helper so call sites read
/// intention-first.
pub fn core_task_to_event_subtask(
    mut subtask: crate::core::agent::task::SubTask,
    task_id: &str,
) -> crate::core::agent::task::SubTask {
    if subtask.task_id.is_empty() {
        subtask.task_id = task_id.to_string();
    }
    subtask
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

/// Parse and validate the optional top-level `framing` argument from an
/// `agent_plan_subtasks` call (Workstream H.2.b).
///
/// Returns `Ok(None)` when no framing is declared (legacy behavior
/// preserved). Returns `Ok(Some(_))` when a valid framing is supplied.
/// Returns `Err(msg)` when the framing is malformed or fails the
/// validation rules in [`crate::core::agent::task::TaskFraming::validate`].
fn parse_and_validate_framing(
    raw: Option<&serde_json::Value>,
) -> Result<Option<crate::core::agent::task::TaskFraming>, String> {
    let Some(raw) = raw else { return Ok(None) };
    if raw.is_null() {
        return Ok(None);
    }
    let framing: crate::core::agent::task::TaskFraming =
        serde_json::from_value(raw.clone()).map_err(|e| {
            format!(
                "framing argument is malformed — expected an object with `role`, `voice_constraints`, optional `domain_priors`, optional `forbidden_patterns`: {e}"
            )
        })?;
    framing.validate()?;
    Ok(Some(framing))
}

#[cfg(test)]
mod framing_parse_tests {
    //! Workstream H.2.b — planner framing validation.
    use super::*;
    use crate::core::agent::task::{DomainPriors, TaskFraming};

    #[test]
    fn absent_framing_parses_as_none() {
        assert!(parse_and_validate_framing(None).unwrap().is_none());
    }

    #[test]
    fn null_framing_parses_as_none() {
        let null = serde_json::Value::Null;
        assert!(parse_and_validate_framing(Some(&null)).unwrap().is_none());
    }

    #[test]
    fn valid_communication_framing_parses_and_persists_shape() {
        let json = serde_json::json!({
            "role": "senior communication professional",
            "voiceConstraints": [
                "one claim per paragraph",
                "no filler openers"
            ],
            "domainPriors": {"hints": ["press-release voice"]},
            "forbiddenPatterns": ["(?i)game[- ]changing"]
        });
        let framing = parse_and_validate_framing(Some(&json))
            .unwrap()
            .expect("framing should be parsed");
        assert_eq!(framing.role, "senior communication professional");
        assert_eq!(framing.voice_constraints.len(), 2);
        assert_eq!(
            framing.domain_priors,
            Some(DomainPriors {
                hints: vec!["press-release voice".to_string()]
            }),
        );
        assert_eq!(framing.forbidden_patterns.len(), 1);
    }

    #[test]
    fn framing_with_single_voice_constraint_is_rejected() {
        let json = serde_json::json!({
            "role": "writer",
            "voiceConstraints": ["professional and clear"]
        });
        let err = parse_and_validate_framing(Some(&json)).unwrap_err();
        assert!(err.contains("voice_constraints"));
    }

    #[test]
    fn framing_with_invalid_forbidden_regex_is_rejected() {
        let json = serde_json::json!({
            "role": "writer",
            "voiceConstraints": ["a", "b"],
            "forbiddenPatterns": ["[unclosed"]
        });
        let err = parse_and_validate_framing(Some(&json)).unwrap_err();
        assert!(err.contains("forbidden_patterns"));
    }

    #[test]
    fn framing_missing_required_fields_is_rejected_with_helpful_message() {
        // The planner supplied voice_constraints but omitted role.
        let json = serde_json::json!({
            "voiceConstraints": ["a", "b"]
        });
        let err = parse_and_validate_framing(Some(&json)).unwrap_err();
        assert!(
            err.contains("role") || err.contains("malformed"),
            "error message must point at the malformed shape: {err}",
        );
    }

    #[test]
    fn handler_branch_accepts_communication_framing_shape() {
        // Workstream H.2.b acceptance: the 2026-04-18 workflow's
        // launch-communication task produces a `communication` framing,
        // not a `code` one. The handler must parse and accept that
        // shape — this test stands in for the end-to-end re-run the
        // plan specifies (which cannot be driven from a pure test).
        let communication = serde_json::json!({
            "role": "senior communication professional",
            "voiceConstraints": [
                "one claim per paragraph",
                "no adjectives without a concrete number"
            ],
            "domainPriors": {"hints": ["press-release voice", "B2B SaaS launch copy"]},
            "forbiddenPatterns": [
                "(?i)pioneering",
                "(?i)game[- ]changing",
                "(?i)unprecedented"
            ]
        });
        let framing: TaskFraming = parse_and_validate_framing(Some(&communication))
            .expect("communication framing must validate")
            .expect("framing present");
        assert_eq!(framing.role, "senior communication professional");
        assert_eq!(framing.voice_constraints.len(), 2);
        assert_eq!(framing.forbidden_patterns.len(), 3);
    }
}

#[cfg(test)]
mod plan_quality_validation_tests {
    //! Workstream G.1 — plan-quality validation (overlap + no-op final).
    //!
    //! These tests run with the default build because they exercise
    //! validators inside `validate_plan_subtasks` that are on the
    //! hot path for every `agent_plan_subtasks` call.
    use super::*;

    // -------- overlap detection --------------------------------------------

    #[test]
    fn overlap_rejects_byte_identical_patterns_on_different_subtasks() {
        let subtasks = vec![
            serde_json::json!({
                "title": "Write social copy",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/communications/social/*.md", "kind": "file"}
                    ]
                }
            }),
            serde_json::json!({
                "title": "Write launch email",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/communications/social/*.md", "kind": "file"}
                    ]
                }
            }),
        ];
        let err = detect_overlapping_contracts(&subtasks)
            .expect_err("identical patterns must be rejected");
        assert!(err.contains("Write social copy"), "message: {err}");
        assert!(err.contains("Write launch email"), "message: {err}");
        assert!(err.contains("overlapping"), "message: {err}");
    }

    #[test]
    fn overlap_rejects_prefix_relationship_patterns_on_different_subtasks() {
        let subtasks = vec![
            serde_json::json!({
                "title": "Write all communications",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/communications/**/*.md", "kind": "file"}
                    ]
                }
            }),
            serde_json::json!({
                "title": "Write social copy",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/communications/social/*.md", "kind": "file"}
                    ]
                }
            }),
        ];
        let err = detect_overlapping_contracts(&subtasks)
            .expect_err("prefix-related patterns must be rejected");
        assert!(err.contains("overlapping"), "message: {err}");
    }

    #[test]
    fn overlap_accepts_distinct_neighbouring_patterns() {
        let subtasks = vec![
            serde_json::json!({
                "title": "Write tweets",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/communications/social/twitter-*.md", "kind": "file"}
                    ]
                }
            }),
            serde_json::json!({
                "title": "Write posts",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/communications/social/linkedin-*.md", "kind": "file"}
                    ]
                }
            }),
        ];
        assert!(
            detect_overlapping_contracts(&subtasks).is_ok(),
            "sibling-pattern plan must be accepted"
        );
    }

    #[test]
    fn overlap_accepts_plan_without_contracts() {
        let subtasks = vec![
            serde_json::json!({"title": "Analyze code"}),
            serde_json::json!({"title": "Implement feature"}),
            serde_json::json!({"title": "Verify tests"}),
        ];
        assert!(
            detect_overlapping_contracts(&subtasks).is_ok(),
            "legacy plans must not regress"
        );
    }

    #[test]
    fn overlap_permits_multiple_patterns_on_the_same_subtask() {
        // A subtask may legitimately declare two expected outputs;
        // they must not trigger the overlap detector against each
        // other.
        let subtasks = vec![serde_json::json!({
            "title": "Write release bundle",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/release/notes.md", "kind": "file"},
                    {"path_pattern": "docs/release/*.md", "kind": "file"}
                ]
            }
        })];
        assert!(
            detect_overlapping_contracts(&subtasks).is_ok(),
            "intra-subtask patterns must not collide"
        );
    }

    #[test]
    fn overlap_flags_literal_path_matched_by_other_glob() {
        // Literal `docs/social/twitter.md` falls under the glob
        // `docs/social/*.md` on a different subtask.
        let subtasks = vec![
            serde_json::json!({
                "title": "Write twitter copy",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/social/twitter.md", "kind": "file"}
                    ]
                }
            }),
            serde_json::json!({
                "title": "Write social copy",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/social/*.md", "kind": "file"}
                    ]
                }
            }),
        ];
        assert!(detect_overlapping_contracts(&subtasks).is_err());
    }

    #[test]
    fn overlap_helper_rejects_proper_prefix_alignment() {
        assert!(patterns_overlap(
            "docs/social/**/*.md",
            "docs/social/twitter/*.md"
        ));
    }

    #[test]
    fn overlap_helper_accepts_sibling_directories() {
        assert!(!patterns_overlap(
            "docs/a/*.md",
            "docs/alpha/*.md"
        ));
    }

    #[test]
    fn overlap_helper_accepts_patterns_with_no_literal_anchor() {
        // Two glob-only patterns with no literal prefix must not be
        // reported as overlapping (conservative default).
        assert!(!patterns_overlap("*.md", "*.txt"));
    }

    // -------- no-op final subtask -----------------------------------------

    #[test]
    fn noop_rejects_documentation_of_prior_work_with_empty_contract() {
        let subtasks = vec![
            serde_json::json!({
                "title": "Draft press release",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/communications/press-release.md", "kind": "file"}
                    ]
                }
            }),
            serde_json::json!({
                "title": "Generate documentation of prior subtasks in ./docs/communications/"
            }),
        ];
        let err = detect_noop_final_subtask(&subtasks)
            .expect_err("redundant documentation subtask must be rejected");
        assert!(err.contains("no-op"), "message: {err}");
        assert!(
            err.contains("documentation of"),
            "message must cite the matched trigger: {err}"
        );
    }

    #[test]
    fn noop_rejects_summary_subtask_with_no_contract() {
        let subtasks = vec![
            serde_json::json!({"title": "Draft release notes"}),
            serde_json::json!({"title": "Summarize launch communications"}),
        ];
        assert!(detect_noop_final_subtask(&subtasks).is_err());
    }

    #[test]
    fn noop_accepts_summary_subtask_when_contract_declares_output() {
        // The planner can declare a summary subtask if it commits to
        // a concrete artifact. The presence of the contract takes
        // the subtask out of the no-op set.
        let subtasks = vec![
            serde_json::json!({"title": "Draft release notes"}),
            serde_json::json!({
                "title": "Summarize launch communications",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/communications/INDEX.md", "kind": "file"}
                    ]
                }
            }),
        ];
        assert!(detect_noop_final_subtask(&subtasks).is_ok());
    }

    #[test]
    fn noop_accepts_single_subtask_plan() {
        let subtasks = vec![serde_json::json!({
            "title": "Summarize findings"
        })];
        assert!(detect_noop_final_subtask(&subtasks).is_ok());
    }

    #[test]
    fn noop_accepts_plan_whose_final_subtask_is_not_a_summary() {
        let subtasks = vec![
            serde_json::json!({"title": "Analyze requirements"}),
            serde_json::json!({"title": "Implement changes"}),
            serde_json::json!({"title": "Verify with tests"}),
        ];
        assert!(detect_noop_final_subtask(&subtasks).is_ok());
    }

    #[test]
    fn noop_trigger_is_case_insensitive() {
        let subtasks = vec![
            serde_json::json!({"title": "Draft press release"}),
            serde_json::json!({
                "title": "LIST OF FILES produced by prior subtasks"
            }),
        ];
        assert!(detect_noop_final_subtask(&subtasks).is_err());
    }

    #[test]
    fn noop_checks_description_field_too() {
        let subtasks = vec![
            serde_json::json!({"title": "Draft press release"}),
            serde_json::json!({
                "title": "Wrap up",
                "description": "Generate a recap of the work done in the prior subtasks."
            }),
        ];
        assert!(detect_noop_final_subtask(&subtasks).is_err());
    }

    // -------- integration through validate_plan_subtasks ------------------

    #[test]
    fn validate_plan_subtasks_rejects_overlapping_contracts_with_clear_message() {
        // Also satisfies the verification-step requirement so the
        // rejection can be pinned on the overlap detector alone.
        let subtasks = vec![
            serde_json::json!({
                "title": "Implement social copy",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/social/*.md", "kind": "file"}
                    ]
                }
            }),
            serde_json::json!({
                "title": "Implement email copy",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/social/*.md", "kind": "file"}
                    ]
                }
            }),
            serde_json::json!({"title": "Verify outputs"}),
        ];
        let err = validate_plan_subtasks(&subtasks, false)
            .expect_err("overlapping plans must fail validation");
        assert!(err.contains("overlapping"), "message: {err}");
    }

    #[test]
    fn validate_plan_subtasks_rejects_redundant_final_summary_subtask() {
        // Title uses an actionable verb ("Write") so the pre-existing
        // verb check is satisfied — the rejection reason must come
        // from the G.1 no-op heuristic.
        let subtasks = vec![
            serde_json::json!({"title": "Analyze existing copy"}),
            serde_json::json!({"title": "Implement new copy"}),
            serde_json::json!({
                "title": "Write documentation of what was done"
            }),
        ];
        let err = validate_plan_subtasks(&subtasks, false)
            .expect_err("redundant final summary must fail validation");
        assert!(err.contains("no-op"), "message: {err}");
    }

    #[test]
    fn validate_plan_subtasks_accepts_clean_contracted_plan() {
        let subtasks = vec![
            serde_json::json!({
                "title": "Implement tweet copy",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/social/twitter-*.md", "kind": "file"}
                    ]
                }
            }),
            serde_json::json!({
                "title": "Implement post copy",
                "contract": {
                    "expected_artifacts": [
                        {"path_pattern": "docs/social/linkedin-*.md", "kind": "file"}
                    ]
                }
            }),
        ];
        assert!(validate_plan_subtasks(&subtasks, false).is_ok());
    }

    #[test]
    fn validate_plan_subtasks_accepts_legacy_contract_free_plan() {
        let subtasks = vec![
            serde_json::json!({"title": "Analyze structure"}),
            serde_json::json!({"title": "Implement update"}),
            serde_json::json!({"title": "Verify behavior"}),
        ];
        assert!(validate_plan_subtasks(&subtasks, false).is_ok());
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

// ---------------------------------------------------------------------------
// Workstream F.1 — contract verification tests.
//
// Scoped to the pure helpers (`verify_contract_artifacts` +
// `ContractVerificationFailure::into_error_result`). End-to-end
// handler-level assertions would require spinning up the full
// ApprovalQueue / AppHandle / ToolRegistry, which the existing
// service_tests.rs covers. These tests exercise the decision logic.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod contract_verification_tests {
    use super::*;
    use crate::core::agent::session::{AgentSession, SessionId};
    use crate::core::agent::task::{ArtifactKind, ArtifactSpec, SubTaskContract};
    use crate::core::agent::types::AgentId;
    use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
    use crate::infrastructure::sqlite::repositories::agent::{
        SessionRepositoryError, TerminalLineEntry, ToolCallEntry, TouchedFileEntry,
    };
    use serde_json::Value;
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// Minimal telemetry double that only needs to answer
    /// `get_session_files`. Everything else is a no-op so the test
    /// focuses on the overlay-check path.
    struct OverlayTelemetry {
        files: Mutex<Vec<TouchedFileEntry>>,
    }

    impl OverlayTelemetry {
        fn new(files: Vec<(&str, &str)>) -> Self {
            let files = files
                .into_iter()
                .enumerate()
                .map(|(i, (path, action))| TouchedFileEntry {
                    id: format!("id-{i}"),
                    session_id: "session".to_string(),
                    file_path: path.to_string(),
                    action: action.to_string(),
                    diff: None,
                    timestamp: "2026-04-18T00:00:00Z".to_string(),
                })
                .collect();
            Self {
                files: Mutex::new(files),
            }
        }
    }

    impl AgentTelemetryRepository for OverlayTelemetry {
        fn create_execution_step(
            &self,
            _step: &crate::core::agent::task::ExecutionStep,
            _session_id: &str,
            _task_id: Option<&str>,
            _subtask_id: Option<&str>,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_session(
            &self,
            _session_id: &SessionId,
        ) -> Result<Option<AgentSession>, SessionRepositoryError> {
            Ok(None)
        }
        fn get_session_execution_steps(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<crate::core::agent::task::ExecutionStep>, SessionRepositoryError>
        {
            Ok(vec![])
        }
        fn add_file_touched(
            &self,
            _session_id: &SessionId,
            _file_path: &str,
            _action: &str,
            _diff: Option<&str>,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_session_files(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<TouchedFileEntry>, SessionRepositoryError> {
            Ok(self.files.lock().unwrap().clone())
        }
        fn add_terminal_line(
            &self,
            _session_id: &SessionId,
            _line_type: &str,
            _content: &str,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_session_terminal_output(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<TerminalLineEntry>, SessionRepositoryError> {
            Ok(vec![])
        }
        fn add_status_phrase(
            &self,
            _session_id: &SessionId,
            _phrase: &str,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_session_status_phrases(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<String>, SessionRepositoryError> {
            Ok(vec![])
        }
        fn clear_session_runtime_data(
            &self,
            _session_id: &SessionId,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn insert_tool_call(
            &self,
            _session_id: &SessionId,
            _tool_call_id: &str,
            _tool_name: &str,
            _arguments: &Value,
            _subtask_id: Option<&str>,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn update_tool_call_result(
            &self,
            _tool_call_id: &str,
            _result: &Value,
            _is_error: bool,
            _duration_ms: Option<i64>,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_tool_calls_for_session(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<ToolCallEntry>, SessionRepositoryError> {
            Ok(vec![])
        }
    }

    fn session_with_workspace(root: Option<PathBuf>) -> AgentSession {
        let mut s = AgentSession::new(
            AgentId::new("code-agent"),
            "Code".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            root,
            None,
        );
        s.id = SessionId("session".to_string());
        s
    }

    fn contract(pattern: &str, kind: ArtifactKind) -> SubTaskContract {
        SubTaskContract {
            expected_artifacts: vec![ArtifactSpec {
                path_pattern: pattern.to_string(),
                kind,
            }],
        }
    }

    #[test]
    fn contract_fails_when_neither_disk_nor_overlay_have_the_artifact() {
        // Replay of the 2026-04-18 subtask-3 situation: contract
        // promises social-media files; none exist; none were
        // recorded in files_touched. Advance must be rejected with
        // `contract_unsatisfied`.
        let temp = tempfile::tempdir().expect("temp");
        let session = session_with_workspace(Some(temp.path().to_path_buf()));
        let telemetry = OverlayTelemetry::new(vec![]);
        let c = contract("docs/communications/social/*.md", ArtifactKind::File);
        let err = verify_contract_artifacts(&c, &session, &telemetry)
            .expect_err("missing artifacts must fail verification");
        assert_eq!(err.missing_patterns.len(), 1);
        assert!(err.missing_patterns[0].contains("social"));
    }

    #[test]
    fn contract_passes_when_file_exists_on_disk() {
        let temp = tempfile::tempdir().expect("temp");
        let social = temp.path().join("docs/communications/social");
        std::fs::create_dir_all(&social).expect("mkdir");
        std::fs::write(social.join("twitter-launch.md"), "x").expect("write");

        let session = session_with_workspace(Some(temp.path().to_path_buf()));
        let telemetry = OverlayTelemetry::new(vec![]);
        let c = contract("docs/communications/social/*.md", ArtifactKind::File);
        verify_contract_artifacts(&c, &session, &telemetry)
            .expect("existing file on disk satisfies the pattern");
    }

    #[test]
    fn contract_passes_when_overlay_records_a_matching_write() {
        // No workspace root available — fallback: overlay entries win.
        let session = session_with_workspace(None);
        let telemetry = OverlayTelemetry::new(vec![
            ("docs/communications/social/twitter.md", "write"),
        ]);
        let c = contract("docs/communications/social/*.md", ArtifactKind::File);
        verify_contract_artifacts(&c, &session, &telemetry)
            .expect("overlay write matching the glob satisfies the pattern");
    }

    #[test]
    fn contract_rejection_includes_out_of_contract_touched_paths() {
        // Exact 2026-04-18 scope-drift shape: subtask declared
        // `docs/communications/social/*.md`; the model wrote only
        // emails. The rejection must list the out-of-contract paths
        // the model actually produced, so the correction message is
        // concrete.
        let session = session_with_workspace(None);
        let telemetry = OverlayTelemetry::new(vec![
            ("docs/communications/email/outreach.md", "write"),
            ("docs/communications/email/launch.md", "write"),
        ]);
        let c = contract("docs/communications/social/*.md", ArtifactKind::File);
        let err = verify_contract_artifacts(&c, &session, &telemetry)
            .expect_err("drifted writes do not satisfy the contract");
        assert_eq!(err.missing_patterns.len(), 1);
        assert!(err
            .concrete_missing_paths
            .iter()
            .any(|p| p.contains("email/outreach.md")));
        assert!(err
            .concrete_missing_paths
            .iter()
            .any(|p| p.contains("email/launch.md")));
    }

    #[test]
    fn verification_failure_serializes_with_contract_unsatisfied_reason() {
        let call = ToolCall {
            id: "call-1".to_string(),
            name: "agent_advance_to_next_subtask".to_string(),
            arguments: serde_json::json!({}),
        };
        let failure = ContractVerificationFailure {
            missing_patterns: vec!["docs/social/*.md".to_string()],
            concrete_missing_paths: vec!["docs/email/x.md".to_string()],
        };
        let result = failure.into_error_result(&call);
        assert!(result.is_error);
        assert_eq!(
            result.output.get("reason").and_then(|v| v.as_str()),
            Some("contract_unsatisfied"),
        );
        let patterns = result
            .output
            .get("missingPatterns")
            .and_then(|v| v.as_array())
            .expect("missingPatterns");
        assert_eq!(patterns.len(), 1);
        let concrete = result
            .output
            .get("concreteMissingPaths")
            .and_then(|v| v.as_array())
            .expect("concreteMissingPaths");
        assert_eq!(concrete.len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Workstream α.1 — tool-planner merge tests.
//
// The behavior under test is the replacement of the pre-α.1 early-return
// ("Subtasks already planned for this task. Continue with current
// subtask.") with `merge_model_plan_into_existing`. The merge dispatch
// lives inside `handle_plan_subtasks` — these tests call the merge
// helper directly so the assertions pin down its per-slot rules
// without dragging in the whole permissions pipeline.
//
// The task-repository fixture is a real in-memory SQLite repo so the
// contract/mode/title/description column writes go through the exact
// SQL path production uses. The Tauri `AppHandle` comes from the
// shared `test_support::mock_app_handle()`; the merge helper only
// uses it for best-effort event emission (`AgentEvent::Subtask*`),
// which is not asserted here.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod merge_tests {
    use super::*;
    use crate::core::agent::repositories::TaskRepository as _;
    use crate::core::agent::session::{AgentSession, SessionId};
    use crate::core::agent::task::{
        AgentRequest, AgentTask, ArtifactKind, ArtifactSpec, SubTask, SubTaskContract, SubTaskMode,
        TaskStatus,
    };
    use crate::core::agent::types::AgentId;
    use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;
    use serde_json::{json, Value};
    use std::sync::Arc;

    fn make_repo_with_task() -> (Arc<SqliteSessionRepository>, String) {
        let repo = Arc::new(
            SqliteSessionRepository::new(":memory:").expect("in-memory repo"),
        );
        let session = AgentSession::new(
            AgentId::new("agent"),
            "Agent".to_string(),
            "conv".to_string(),
            "thread".to_string(),
            None,
            None,
        );
        repo.as_ref()
            .create_session(&session)
            .expect("session create");
        let task = AgentTask::new(
            session.id.clone(),
            AgentRequest {
                content: "test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        let task_id = task.id.clone();
        <SqliteSessionRepository as TaskRepository>::create_task(repo.as_ref(), &task)
            .expect("task create");
        (repo, task_id)
    }

    fn persist_skeleton(
        repo: &SqliteSessionRepository,
        task_id: &str,
        titles: &[&str],
    ) -> Vec<SubTask> {
        let mut created = Vec::new();
        for (idx, title) in titles.iter().enumerate() {
            let subtask = SubTask::new(
                task_id.to_string(),
                (*title).to_string(),
                SubTaskMode::FullAccess,
                idx as i32,
            );
            <SqliteSessionRepository as TaskRepository>::create_subtask(repo, &subtask)
                .expect("subtask create");
            created.push(subtask);
        }
        created
    }

    fn make_session() -> AgentSession {
        AgentSession::new(
            AgentId::new("agent"),
            "Agent".to_string(),
            "conv".to_string(),
            "thread".to_string(),
            None,
            None,
        )
    }

    fn make_call(args: Value) -> ToolCall {
        ToolCall {
            id: "call-1".to_string(),
            name: "agent_plan_subtasks".to_string(),
            arguments: args,
        }
    }

    /// Build a contract JSON payload.
    ///
    /// [`SubTaskContract`] deserializes with `rename_all = "camelCase"`,
    /// so `from_value` requires `expectedArtifacts` + `pathPattern`
    /// even though the plan-validation helpers accept either spelling.
    /// These tests exercise the merge helper, which hits
    /// `from_value::<SubTaskContract>` directly.
    fn contract_for_path(path: &str) -> Value {
        json!({
            "expectedArtifacts": [
                {"pathPattern": path, "kind": "file"}
            ]
        })
    }

    #[test]
    fn merge_attaches_contract_and_refines_fields_on_pending_skeleton() {
        let (repo, task_id) = make_repo_with_task();
        persist_skeleton(repo.as_ref(), &task_id, &[
            "Investigate auth module",
            "Implement fix",
            "Verify behavior",
        ]);

        let existing =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        let session = make_session();
        let app_handle = crate::test_support::mock_app_handle();
        let call = make_call(json!({}));

        let model_plan = vec![
            json!({
                "title": "Investigate auth module",
                "description": "Read auth/*.rs and map the current flow",
                "contract": contract_for_path("docs/auth/*.md"),
                "order": 0
            }),
            json!({
                "title": "Implement session-handoff fix",
                "description": "Update auth/session.rs and add regression tests",
                "contract": contract_for_path("src-tauri/src/core/auth/**/*.rs"),
                "mode": "write",
                "order": 1
            }),
            json!({
                "title": "Verify behavior",
                "contract": contract_for_path("tests/auth/*.rs"),
                "order": 2
            }),
        ];

        let result = merge_model_plan_into_existing(
            &call,
            &session,
            &task_id,
            &model_plan,
            &existing,
            &app_handle,
            repo.as_ref(),
        );

        assert!(!result.is_error, "merge should succeed: {:?}", result.output);
        let refined = result.output.get("refined").and_then(Value::as_u64).unwrap();
        let appended = result.output.get("appended").and_then(Value::as_u64).unwrap();
        assert_eq!(refined, 3);
        assert_eq!(appended, 0);

        let after =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        assert_eq!(after.len(), 3);
        for st in &after {
            assert!(
                st.contract.is_some(),
                "order {} should have a contract after merge",
                st.order,
            );
        }
        let by_order: std::collections::HashMap<i32, &SubTask> =
            after.iter().map(|st| (st.order, st)).collect();
        assert_eq!(
            by_order[&1].title,
            "Implement session-handoff fix",
            "title should be updated on the pending slot",
        );
        assert_eq!(by_order[&1].mode, SubTaskMode::Write);
        assert_eq!(
            by_order[&0].description.as_deref(),
            Some("Read auth/*.rs and map the current flow"),
        );
    }

    #[test]
    fn merge_appends_when_model_declares_more_subtasks() {
        let (repo, task_id) = make_repo_with_task();
        persist_skeleton(repo.as_ref(), &task_id, &["Plan", "Execute"]);

        let existing =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        let session = make_session();
        let app_handle = crate::test_support::mock_app_handle();
        let call = make_call(json!({}));

        let model_plan = vec![
            json!({
                "title": "Plan",
                "contract": contract_for_path("docs/plan.md"),
                "order": 0
            }),
            json!({
                "title": "Execute",
                "contract": contract_for_path("src/*.rs"),
                "order": 1
            }),
            json!({
                "title": "Verify outputs",
                "contract": contract_for_path("tests/*.rs"),
                "order": 2
            }),
        ];

        let result = merge_model_plan_into_existing(
            &call,
            &session,
            &task_id,
            &model_plan,
            &existing,
            &app_handle,
            repo.as_ref(),
        );
        assert!(!result.is_error, "{:?}", result.output);
        assert_eq!(result.output["appended"], 1);
        assert_eq!(result.output["refined"], 2);

        let after =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        assert_eq!(after.len(), 3);
        assert_eq!(after[2].title, "Verify outputs");
        assert!(after[2].contract.is_some());
    }

    #[test]
    fn merge_ignores_extras_when_model_declares_fewer_subtasks() {
        let (repo, task_id) = make_repo_with_task();
        persist_skeleton(repo.as_ref(), &task_id, &["A", "B", "C"]);

        let existing =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        let session = make_session();
        let app_handle = crate::test_support::mock_app_handle();
        let call = make_call(json!({}));

        let model_plan = vec![json!({
            "title": "A",
            "contract": contract_for_path("a/*.md"),
            "order": 0
        })];

        let result = merge_model_plan_into_existing(
            &call,
            &session,
            &task_id,
            &model_plan,
            &existing,
            &app_handle,
            repo.as_ref(),
        );
        assert!(!result.is_error);

        let after =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        assert_eq!(after.len(), 3, "extra skeleton slots must be preserved");
        assert!(
            after[0].contract.is_some(),
            "the refined slot still receives its contract",
        );
        assert!(after[1].contract.is_none());
        assert!(after[2].contract.is_none());
    }

    #[test]
    fn merge_refuses_to_redefine_completed_subtask_with_requires_replan() {
        let (repo, task_id) = make_repo_with_task();
        let created =
            persist_skeleton(repo.as_ref(), &task_id, &["Investigate", "Implement"]);
        <SqliteSessionRepository as TaskRepository>::update_subtask_status(
            repo.as_ref(),
            &created[0].id,
            TaskStatus::Completed,
        )
        .unwrap();

        let existing =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        let session = make_session();
        let app_handle = crate::test_support::mock_app_handle();
        let call = make_call(json!({}));

        let model_plan = vec![
            json!({
                "title": "Investigate DIFFERENT scope",
                "description": "A materially different description",
                "contract": contract_for_path("x/*.md"),
                "order": 0
            }),
            json!({"title": "Implement", "order": 1}),
        ];

        let result = merge_model_plan_into_existing(
            &call,
            &session,
            &task_id,
            &model_plan,
            &existing,
            &app_handle,
            repo.as_ref(),
        );
        assert!(result.is_error);
        assert_eq!(result.output["requires_replan"], true);
        assert!(result
            .output
            .get("feedback")
            .and_then(Value::as_str)
            .unwrap()
            .contains("Completed"));
    }

    #[test]
    fn merge_refuses_to_append_when_a_completed_subtask_exists() {
        let (repo, task_id) = make_repo_with_task();
        let created =
            persist_skeleton(repo.as_ref(), &task_id, &["First", "Second"]);
        <SqliteSessionRepository as TaskRepository>::update_subtask_status(
            repo.as_ref(),
            &created[0].id,
            TaskStatus::Completed,
        )
        .unwrap();

        let existing =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        let session = make_session();
        let app_handle = crate::test_support::mock_app_handle();
        let call = make_call(json!({}));

        let model_plan = vec![
            json!({"title": "First", "order": 0}),
            json!({"title": "Second", "order": 1}),
            json!({"title": "Added later", "order": 2}),
        ];

        let result = merge_model_plan_into_existing(
            &call,
            &session,
            &task_id,
            &model_plan,
            &existing,
            &app_handle,
            repo.as_ref(),
        );
        assert!(result.is_error);
        assert_eq!(result.output["requires_replan"], true);
    }

    #[test]
    fn merge_does_not_mutate_completed_subtask_when_shape_matches() {
        let (repo, task_id) = make_repo_with_task();
        let created =
            persist_skeleton(repo.as_ref(), &task_id, &["First", "Second"]);
        <SqliteSessionRepository as TaskRepository>::update_subtask_status(
            repo.as_ref(),
            &created[0].id,
            TaskStatus::Completed,
        )
        .unwrap();

        let existing =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        let session = make_session();
        let app_handle = crate::test_support::mock_app_handle();
        let call = make_call(json!({}));

        // Model repeats the Completed subtask verbatim and refines the
        // pending one with a contract. No replan required.
        let model_plan = vec![
            json!({"title": "First", "order": 0}),
            json!({
                "title": "Second",
                "contract": contract_for_path("src/*.rs"),
                "order": 1
            }),
        ];

        let result = merge_model_plan_into_existing(
            &call,
            &session,
            &task_id,
            &model_plan,
            &existing,
            &app_handle,
            repo.as_ref(),
        );
        assert!(!result.is_error, "{:?}", result.output);

        let after =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        assert_eq!(after[0].status, TaskStatus::Completed);
        assert!(
            after[0].contract.is_none(),
            "completed slot must not gain a contract it did not already have",
        );
        assert!(after[1].contract.is_some());
    }

    #[test]
    fn merge_rejects_malformed_contract_with_structured_error() {
        let (repo, task_id) = make_repo_with_task();
        persist_skeleton(repo.as_ref(), &task_id, &["One"]);

        let existing =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        let session = make_session();
        let app_handle = crate::test_support::mock_app_handle();
        let call = make_call(json!({}));

        let model_plan = vec![json!({
            "title": "One",
            "contract": { "expected_artifacts": "not-an-array" },
            "order": 0
        })];

        let result = merge_model_plan_into_existing(
            &call,
            &session,
            &task_id,
            &model_plan,
            &existing,
            &app_handle,
            repo.as_ref(),
        );
        assert!(result.is_error);
        let err = result.output.get("error").and_then(Value::as_str).unwrap();
        assert!(err.contains("malformed"), "got: {err}");
    }

    /// Characterization replay — the 8311a601 forensic session had a
    /// skeleton of `N` auto-planned subtasks (contract-less) and the
    /// model's first `agent_plan_subtasks` call carried the same N
    /// titles with contracts attached. The old early-return dropped
    /// every contract. With α.1 in place, all N slots carry a contract
    /// after merge.
    ///
    /// Simplified replay: 7 model subtasks, 1:1 with the skeleton, each
    /// carrying a distinct `expected_artifacts` contract.
    #[test]
    fn merge_characterization_7_subtasks_all_gain_contract() {
        let (repo, task_id) = make_repo_with_task();
        let skeleton_titles = [
            "Plan architecture and tech stack",
            "Design database schema",
            "Scaffold project structure",
            "Implement auth endpoints",
            "Implement session persistence",
            "Wire up error middleware",
            "Run smoke tests",
        ];
        persist_skeleton(repo.as_ref(), &task_id, &skeleton_titles);

        let existing =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        let session = make_session();
        let app_handle = crate::test_support::mock_app_handle();
        let call = make_call(json!({}));

        let model_plan: Vec<Value> = skeleton_titles
            .iter()
            .enumerate()
            .map(|(idx, title)| {
                json!({
                    "title": title,
                    "contract": contract_for_path(&format!("docs/phase-{idx}/*.md")),
                    "order": idx
                })
            })
            .collect();

        let result = merge_model_plan_into_existing(
            &call,
            &session,
            &task_id,
            &model_plan,
            &existing,
            &app_handle,
            repo.as_ref(),
        );
        assert!(!result.is_error, "{:?}", result.output);
        assert_eq!(result.output["appended"], 0);
        assert_eq!(result.output["refined"], 7);

        let after =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        assert_eq!(after.len(), 7);
        for st in &after {
            let contract = st
                .contract
                .as_ref()
                .unwrap_or_else(|| panic!("order {} must gain a contract", st.order));
            assert!(
                !contract.expected_artifacts.is_empty(),
                "contract on order {} must not be empty",
                st.order,
            );
        }
    }

    /// Regression — when the model never issues `agent_plan_subtasks`
    /// (legacy path), the skeleton is untouched. This test protects the
    /// pre-α.1 behavior for non-refining flows: the merge helper is
    /// never entered, so contracts remain `None`.
    #[test]
    fn merge_not_invoked_leaves_skeleton_contracts_none() {
        let (repo, task_id) = make_repo_with_task();
        persist_skeleton(repo.as_ref(), &task_id, &["Only"]);

        let subtasks =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        assert_eq!(subtasks.len(), 1);
        assert!(
            subtasks[0].contract.is_none(),
            "skeleton subtask starts without contract",
        );
        assert_eq!(subtasks[0].mode, SubTaskMode::FullAccess);
    }

    /// SubTaskRefinement round-trip through the SQLite
    /// `TaskRepository::refine_subtask` implementation (Workstream α.1
    /// repository layer). Contract, title, description, and mode update
    /// together; the unset fields (here, none) would stay unchanged.
    #[test]
    fn refine_subtask_round_trip_updates_named_fields_only() {
        use crate::core::agent::repositories::SubTaskRefinement;

        let (repo, task_id) = make_repo_with_task();
        let created = persist_skeleton(repo.as_ref(), &task_id, &["slot"]);
        let slot = &created[0];

        let contract = SubTaskContract {
            expected_artifacts: vec![ArtifactSpec {
                path_pattern: "docs/x.md".to_string(),
                kind: ArtifactKind::File,
            }],
        };
        let refinement = SubTaskRefinement {
            title: Some("refined title".to_string()),
            description: Some("refined description".to_string()),
            mode: Some(SubTaskMode::Write),
            contract: Some(contract.clone()),
        };
        <SqliteSessionRepository as TaskRepository>::refine_subtask(
            repo.as_ref(),
            &slot.id,
            refinement,
        )
        .unwrap();

        let after =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].title, "refined title");
        assert_eq!(after[0].description.as_deref(), Some("refined description"));
        assert_eq!(after[0].mode, SubTaskMode::Write);
        assert_eq!(after[0].contract.as_ref(), Some(&contract));

        // Now a partial refinement that only sets `description` — title,
        // mode, contract must stay.
        let partial = SubTaskRefinement {
            description: Some("newer desc".to_string()),
            ..SubTaskRefinement::default()
        };
        <SqliteSessionRepository as TaskRepository>::refine_subtask(
            repo.as_ref(),
            &after[0].id,
            partial,
        )
        .unwrap();

        let after2 =
            <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
                .unwrap();
        assert_eq!(after2[0].title, "refined title");
        assert_eq!(after2[0].description.as_deref(), Some("newer desc"));
        assert_eq!(after2[0].mode, SubTaskMode::Write);
        assert_eq!(after2[0].contract.as_ref(), Some(&contract));

        // Empty refinement is a legal no-op.
        <SqliteSessionRepository as TaskRepository>::refine_subtask(
            repo.as_ref(),
            &after[0].id,
            SubTaskRefinement::default(),
        )
        .unwrap();
    }
}
