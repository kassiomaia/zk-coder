//! `agent_mark_subtask_status` tool handler + `auto_decompose_failed_subtask`.
//!
//! Extracted from `permissions.rs` in Phase 5 of the permissions-
//! decomposition refactor. The auto-decompose idempotency guard is now
//! expressed against the task repository (by looking for existing
//! children whose description matches the canonical
//! "Auto-decomposed from failed subtask '...'" pattern) rather than an
//! in-memory flag set.

use chrono::Utc;
use log::warn;
use tauri::{AppHandle, Emitter};

use super::contract_verification::verify_contract_artifacts;
use super::shared::{core_task_to_event_subtask, parse_status};
use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::session_scope::RuntimeSessionScope;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;

/// Prefix used on the `description` field of auto-decomposed children
/// so the repository-based idempotency guard can recognise them.
const AUTO_DECOMPOSE_DESCRIPTION_PREFIX: &str = "Auto-decomposed from failed subtask '";

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_mark_subtask_status(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    session_repo: &dyn SessionRepository,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    scope: &mut RuntimeSessionScope,
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
    if matches!(core_status, TaskStatus::Completed) {
        if let Some(ref contract) = current.contract {
            if !contract.is_empty() {
                if let Err(failure) = verify_contract_artifacts(contract, session, telemetry_repo) {
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

    // Workstream F.2 — flip the session's `needs_review` flag on Failed.
    if matches!(event_status, TaskStatus::Failed) {
        if let Err(e) = session_repo.set_needs_review(&session.id, true) {
            warn!("[AgentRuntime] Failed to set needs_review on session: {e}");
        }
    }

    let mut updated = core_task_to_event_subtask(current, task_id);
    updated.status = event_status.clone();
    if matches!(event_status, TaskStatus::InProgress) {
        updated.started_at = Some(Utc::now());
    }
    if matches!(event_status, TaskStatus::Completed | TaskStatus::Failed) {
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
        scope.record_rationale(&updated_id, &reason);
    }

    if matches!(event_status, TaskStatus::Failed) {
        let fail_count = scope.increment_failure(&updated_id);
        if fail_count >= 2
            && !already_auto_decomposed(task_repo, task_id, &current_for_decompose.id)
        {
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

/// Task-repository-based replacement for the old in-memory
/// `DECOMPOSED_SUBTASKS` set. Returns `true` when the task already
/// carries a child subtask whose description names the failed subtask
/// id using the canonical "Auto-decomposed from failed subtask '...'"
/// prefix.
fn already_auto_decomposed(
    task_repo: &dyn TaskRepository,
    task_id: &str,
    failed_subtask_id: &str,
) -> bool {
    let needle = format!("{id}')", id = failed_subtask_id);
    task_repo
        .get_subtasks(task_id)
        .ok()
        .map(|subtasks| {
            subtasks.iter().any(|st| {
                st.description
                    .as_deref()
                    .is_some_and(|d| d.starts_with(AUTO_DECOMPOSE_DESCRIPTION_PREFIX) && d.contains(&needle))
            })
        })
        .unwrap_or(false)
}

fn auto_decompose_failed_subtask(
    session: &AgentSession,
    task_id: &str,
    failed_subtask: &SubTask,
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
        let mut subtask = SubTask::new(
            task_id.to_string(),
            title.clone(),
            SubTaskMode::FullAccess,
            base_order + idx as i32,
        );
        subtask.description = Some(format!(
            "{prefix}{t}' ({id})",
            prefix = AUTO_DECOMPOSE_DESCRIPTION_PREFIX,
            t = failed_subtask.title,
            id = failed_subtask.id,
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
        let _ = task_repo.update_subtask_status(first_id, TaskStatus::InProgress);
        let refreshed = task_repo.get_subtasks(task_id).ok().unwrap_or_default();
        if let Some(first) = refreshed.iter().find(|s| &s.id == first_id).cloned() {
            let mut event = core_task_to_event_subtask(first, task_id);
            event.status = TaskStatus::InProgress;
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
