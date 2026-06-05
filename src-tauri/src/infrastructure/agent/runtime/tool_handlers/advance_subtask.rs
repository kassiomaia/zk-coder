//! `agent_advance_to_next_subtask` tool handler.
//!
//! Gates subtask completion on evidence (recorded tool calls, explicit
//! rationale) and on contract-artifact verification when a contract is
//! declared. Extracted from `permissions.rs` in Phase 5.

use chrono::Utc;
use log::warn;
use tauri::{AppHandle, Emitter};

use super::contract_verification::{has_subtask_evidence, verify_contract_artifacts};
use super::dispatcher::{ToolHandler, ToolHandlerContext};
use super::shared::core_task_to_event_subtask;
use crate::core::agent::repositories::TaskRepository;
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::TaskStatus;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::session_scope::RuntimeSessionScope;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;

/// Zero-sized marker for the `agent_advance_to_next_subtask` meta-tool.
pub(super) struct AdvanceSubtaskHandler;

/// Static instance the registry references so lookups stay allocation-free.
pub(super) static ADVANCE_SUBTASK: AdvanceSubtaskHandler = AdvanceSubtaskHandler;

impl ToolHandler for AdvanceSubtaskHandler {
    fn name(&self) -> &'static str {
        "agent_advance_to_next_subtask"
    }

    fn handle(&self, call: &ToolCall, ctx: &mut ToolHandlerContext<'_>) -> OrchestratorToolResult {
        handle_advance_subtask(
            call,
            ctx.session,
            ctx.task_id,
            ctx.app_handle,
            ctx.task_repo,
            ctx.telemetry_repo,
            ctx.scope,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_advance_subtask(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    scope: &mut RuntimeSessionScope,
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
        .position(|st| st.status == TaskStatus::InProgress);

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
        || scope.has_rationale(&current.id);
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
            if let Err(failure) = verify_contract_artifacts(contract, session, telemetry_repo) {
                return failure.into_error_result(call);
            }
        }
    }

    if let Err(e) = task_repo.update_subtask_status(&current.id, TaskStatus::Completed) {
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
    completed_event.status = TaskStatus::Completed;
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
        .find(|st| st.status == TaskStatus::Pending)
        .cloned();

    if let Some(next) = next {
        if let Err(e) = task_repo.update_subtask_status(&next.id, TaskStatus::InProgress) {
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
        in_progress_event.status = TaskStatus::InProgress;
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

    let _ = task_repo.update_task_status(task_id, TaskStatus::Completed);
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: session.id.0.clone(),
            task_id: task_id.to_string(),
            status: TaskStatus::Completed,
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
