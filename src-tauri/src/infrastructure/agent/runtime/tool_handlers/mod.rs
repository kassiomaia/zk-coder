//! Agent-specific tool handlers for agent runtime.
//!
//! This module contains handlers for agent-specific tools:
//! - `agent_plan_subtasks`: Plan and create subtasks for a task
//! - `agent_advance_to_next_subtask`: Advance to the next subtask in sequence
//! - `agent_mark_subtask_status`: Mark a subtask with a specific status
//! - `agent_narrate`: Emit agent narration events

use std::collections::HashSet;
use std::sync::Mutex;

use chrono::Utc;
use log::warn;
use tauri::{AppHandle, Emitter};

use crate::core::agent::repositories::TaskRepository;
use crate::core::agent::session::AgentSession;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;

use super::permissions::core_task_to_event_subtask;

mod subtask_helpers;
use subtask_helpers::{
    auto_decompose_failed_subtask, detect_code_change_intent, has_subtask_evidence,
    has_subtask_rationale, increment_subtask_failure_count, mark_subtask_decomposed_once,
    remember_subtask_rationale, validate_plan_subtasks, PLAN_RETRY_TRACKER,
};

pub use subtask_helpers::{DECOMPOSED_SUBTASKS, SUBTASK_FAILURE_COUNTS, SUBTASK_RATIONALES};

/// Main dispatcher for agent-specific tool calls.
///
/// Routes agent tool calls to their respective handlers. Returns `Some(result)` for
/// agent tools, `None` for tools that should be handled by the generic tool execution path.
pub fn handle_agent_tool_call(
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

/// Handler for the `agent_plan_subtasks` tool.
///
/// Creates subtasks for a task based on the provided plan. Validates that:
/// - Subtasks have unique, non-empty titles
/// - Each subtask title starts with an actionable verb
/// - Code-change tasks include a verification step
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
        let mut retries = retry_map.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
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
            .map(std::string::ToString::to_string);

        // Workstream B.2 — parse the optional `contract` field when
        // the planner declares one. A malformed contract is rejected
        // with a clear message so the model can fix the declaration
        // instead of silently losing the scope gate. Absence is fine —
        // legacy plans carry no contract and that path is preserved.
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

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::SubtaskCreated {
                session_id: session.id.0.clone(),
                subtask,
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

/// Handler for the `agent_advance_to_next_subtask` tool.
///
/// Completes the current in-progress subtask and advances to the next pending subtask.
/// Requires evidence of work (file changes, command execution, or explicit rationale) before advancing.
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

/// Handler for the `agent_mark_subtask_status` tool.
///
/// Marks a specific subtask with a new status. Supports automatic decomposition
/// of failed subtasks after 2 failures.
fn handle_mark_subtask_status(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
    _telemetry_repo: &dyn AgentTelemetryRepository,
) -> OrchestratorToolResult {
    let index = match call.arguments.get("index").and_then(serde_json::Value::as_i64) {
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

/// Handler for the `agent_narrate` tool.
///
/// Emits agent narration events to keep the user informed of progress.
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

/// Parses a status string into core and event task status tuples.
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
