//! `agent_plan_subtasks` tool handler + the α.1 merge path for refining
//! an auto-planned skeleton with a model-declared plan.
//!
//! Extracted from `permissions.rs` in Phase 5 of the permissions-
//! decomposition refactor.

use chrono::Utc;
use log::warn;
use tauri::{AppHandle, Emitter};

use super::dispatcher::{ToolHandler, ToolHandlerContext};
use super::shared::{
    core_task_to_event_subtask, detect_code_change_intent, parse_and_validate_framing,
};
use crate::core::agent::planning::plan_validation::validate_plan_subtasks;
use crate::core::agent::repositories::TaskRepository;
use crate::core::agent::session::AgentSession;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::session_scope::RuntimeSessionScope;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;

/// Zero-sized marker for the `agent_plan_subtasks` meta-tool. Held as a
/// `'static` reference by the tool-handler registry.
pub(super) struct PlanSubtasksHandler;

/// Static instance the registry references so lookups stay allocation-free.
pub(super) static PLAN_SUBTASKS: PlanSubtasksHandler = PlanSubtasksHandler;

impl ToolHandler for PlanSubtasksHandler {
    fn name(&self) -> &'static str {
        "agent_plan_subtasks"
    }

    fn handle(
        &self,
        call: &ToolCall,
        ctx: &mut ToolHandlerContext<'_>,
    ) -> OrchestratorToolResult {
        handle_plan_subtasks(
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
pub(super) fn handle_plan_subtasks<R: tauri::Runtime>(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle<R>,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    scope: &mut RuntimeSessionScope,
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
        let retry_key = format!("{}:{}", session.id.0, task_id);
        let first_retry = scope.mark_plan_retry(&retry_key);
        if first_retry {
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
    // plan into it by order rather than early-returning.
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
/// See the module-level doc and the in-method comments for the full
/// per-slot rules; in short:
/// - Pending/InProgress slot → `refine_subtask` (title, description,
///   mode, contract when differing and non-empty)
/// - Completed slot with material differences → `requires_replan: true`
/// - Model declares more slots than exist → append if no Completed
///   slot exists, otherwise `requires_replan`
/// - Model declares fewer slots → ignore; skeleton slots remain
pub(crate) fn merge_model_plan_into_existing<R: tauri::Runtime>(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    model_plan: &[serde_json::Value],
    existing: &[crate::core::agent::task::SubTask],
    app_handle: &AppHandle<R>,
    task_repo: &dyn TaskRepository,
) -> OrchestratorToolResult {
    use crate::core::agent::repositories::SubTaskRefinement;
    use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};

    let any_completed = existing.iter().any(|st| st.status == TaskStatus::Completed);

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

        let model_mode: Option<SubTaskMode> = item
            .get("mode")
            .and_then(|v| serde_json::from_value::<SubTaskMode>(v.clone()).ok());

        let target_order = item
            .get("order")
            .and_then(serde_json::Value::as_i64)
            .map_or(idx as i32, |v| v as i32);

        let existing_slot = existing.iter().find(|st| st.order == target_order);

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
            }
            Some(slot) => {
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
                    warn!("[AgentRuntime] Failed to append subtask during merge: {e}");
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

// ---------------------------------------------------------------------------
// Tests migrated from `permissions.rs` (Workstream α.1 merge tests).
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "plan_subtasks_tests.rs"]
mod merge_tests;
