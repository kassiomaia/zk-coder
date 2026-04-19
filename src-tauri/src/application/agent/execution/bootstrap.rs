//! Handles the initial setup and bootstrapping of an agent's execution.
//!
//! This module is responsible for preparing the execution environment when a new
//! request is received, including planning the task, creating the necessary
//! data structures, and initializing the execution context.

use crate::application::agent::harness::{
    passed_startup_verification, running_startup_verification,
};
use crate::application::agent::read_models::SessionHandoffSummary;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::memory::SessionMemory;
use crate::core::agent::session::{AgentSession, SessionState};
use crate::core::agent::task::{AgentRequest, AgentTask, SubTask, TaskStatus};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::client::{build_llm_context, create_llm_client};
use crate::infrastructure::agent::runtime::AgentRuntimeError;
use chrono::Utc;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::harness_events::emit_harness_state_updated;
use super::state::ExecutionContext;
use super::termination::update_session_state;
use super::utils::{max_execution_steps, truncate_text};
use crate::application::agent::service::planner::{
    plan_or_continue, subtask_plan_to_agent_task, ExecutionState, PlanAction, TaskPlan,
    UserIntentHint,
};

/// The result of a successful bootstrap process.
pub struct BootstrapResult {
    /// The initialized execution context.
    pub context: ExecutionContext,
    /// The client for communicating with the LLM.
    pub client: Box<dyn crate::infrastructure::llm::LlmClient>,
}

/// Bundle of prior execution state for the resume branch of
/// [`start_execution`]. Built from the task/subtask repositories and
/// the harness state store whenever a prior task exists; `None` means
/// this is a brand-new session with no persisted plan.
struct PriorExecutionState {
    task: AgentTask,
    subtasks: Vec<SubTask>,
    handoff: Option<SessionHandoffSummary>,
    memory: Option<SessionMemory>,
}

/// Loads any prior task + subtasks + handoff + memory for `session_id`.
/// Returns `None` when the session has no persisted task (fresh session).
fn load_prior_execution_state(
    deps: &ExecutionDeps<'_>,
    session_id: &str,
) -> Result<Option<PriorExecutionState>, AgentRuntimeError> {
    let mut tasks = deps
        .task_repo
        .get_session_tasks(session_id)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    let Some(task) = tasks.pop() else {
        return Ok(None);
    };
    let subtasks = deps
        .task_repo
        .get_subtasks(&task.id)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
    let handoff = deps.harness_state_store.get(session_id).handoff;
    let memory = deps
        .memory_repo
        .get_memory(&crate::core::agent::session::SessionId(
            session_id.to_string(),
        ))
        .ok()
        .flatten();
    Ok(Some(PriorExecutionState {
        task,
        subtasks,
        handoff,
        memory,
    }))
}

/// Starts the execution of an agent for a given session and request.
///
/// This is the single entry point for every spawn — fresh starts and
/// resumes both flow through here. The branching happens inside, driven
/// by the state-aware [`plan_or_continue`] planner call:
///
/// 1. Transitions the session to `Running` and emits `ExecutionStarted`.
/// 2. Loads any prior execution state (task, subtasks, handoff, memory).
/// 3. Consumes the pending [`UserIntentHint`] from the harness store
///    (set by the service layer before spawning — Continue / Replan /
///    Fresh). When no state is found, this is effectively a Fresh run.
/// 4. Calls `plan_or_continue` with the bundled state and user message;
///    the planner returns a [`PlanAction`].
/// 5. Dispatches to [`bootstrap_fresh`] or [`bootstrap_continue`]. A
///    `Replan` decision is treated as a Fresh bootstrap of the revised
///    plan — old task rows remain in history.
pub async fn start_execution(
    deps: &ExecutionDeps<'_>,
    session: &AgentSession,
    request: &AgentRequest,
    trust_mode: crate::core::agent::types::TrustMode,
) -> Result<BootstrapResult, AgentRuntimeError> {
    let session_id = session.id.0.clone();

    deps.harness_state_store.record_startup_verification(
        &session_id,
        running_startup_verification("Bootstrapping session"),
    );

    update_session_state(
        deps,
        &session.id,
        SessionState::Running {
            current_step: 0,
            context_usage: 0.0,
        },
    )?;

    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ExecutionStarted {
            session_id: session_id.clone(),
            request_id: uuid::Uuid::new_v4().to_string(),
        },
    );

    let prior = load_prior_execution_state(deps, &session_id)?;
    let intent = deps
        .harness_state_store
        .take_pending_intent(&session_id)
        .unwrap_or(UserIntentHint::Fresh);

    let decision = {
        let state_ref = prior.as_ref().map(|p| ExecutionState {
            task: &p.task,
            subtasks: p.subtasks.as_slice(),
            handoff: p.handoff.as_ref(),
            memory: p.memory.as_ref(),
            user_intent_hint: intent,
        });
        plan_or_continue(
            deps.app_handle,
            &request.content,
            state_ref,
            session.model_id.as_deref(),
        )
        .await
        .map_err(|e| AgentRuntimeError::Repository(format!("Planning failed: {e}")))?
    };

    match decision.action {
        PlanAction::Fresh => {
            let plan = decision.plan.ok_or_else(|| {
                AgentRuntimeError::Repository(
                    "Fresh planning decision is missing a plan".to_string(),
                )
            })?;
            bootstrap_fresh(deps, session, request, trust_mode, plan).await
        }
        PlanAction::Replan => {
            // For this slice, Replan follows the same path as Fresh:
            // we create a brand-new task row from the revised plan.
            // The prior task remains in history so completed work is
            // not discarded from the session record.
            let plan = decision.plan.ok_or_else(|| {
                AgentRuntimeError::Repository(
                    "Replan planning decision is missing a plan".to_string(),
                )
            })?;
            bootstrap_fresh(deps, session, request, trust_mode, plan).await
        }
        PlanAction::Continue => {
            let prior = prior.ok_or_else(|| {
                AgentRuntimeError::Repository(
                    "Continue planning decision but no prior execution state loaded".to_string(),
                )
            })?;
            let next_order = decision.next_subtask_order.unwrap_or(0);
            bootstrap_continue(deps, session, request, trust_mode, prior, next_order).await
        }
    }
}

/// Creates all the plan/task/subtask rows for a brand-new execution
/// window and assembles the `BootstrapResult`. Preserves every side
/// effect the pre-refactor `start_execution` performed (events, tracer
/// snapshots, memory upsert, startup verification completion).
async fn bootstrap_fresh(
    deps: &ExecutionDeps<'_>,
    session: &AgentSession,
    request: &AgentRequest,
    trust_mode: crate::core::agent::types::TrustMode,
    plan: TaskPlan,
) -> Result<BootstrapResult, AgentRuntimeError> {
    let session_id = session.id.0.clone();

    let mut task = AgentTask::new(session.id.clone(), request.clone());
    task.title = Some(plan.title.clone());

    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskPlanGenerated {
            session_id: session_id.clone(),
            task_id: task.id.clone(),
            title: plan.title.clone(),
            description: plan.description.clone(),
            is_non_trivial: plan.is_non_trivial,
            subtasks: plan.subtasks.clone(),
        },
    );

    deps.task_repo
        .create_task(&task)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    let mut first_subtask_id: Option<String> = None;
    let mut collected_subtasks: Vec<SubTask> = Vec::new();
    if plan.is_non_trivial && !plan.subtasks.is_empty() {
        for subtask_plan in &plan.subtasks {
            let mut subtask =
                subtask_plan_to_agent_task(subtask_plan, &task.session_id.0, &task.id);

            if subtask_plan.order == 0 {
                subtask.status = TaskStatus::InProgress;
                subtask.started_at = Some(Utc::now());
                first_subtask_id = Some(subtask.id.clone());
            }

            deps.task_repo
                .create_subtask(&subtask)
                .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

            collected_subtasks.push(subtask.clone());

            let _ = deps.app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::SubtaskCreated {
                    session_id: session_id.clone(),
                    subtask,
                },
            );
        }
    }

    let initial_subtask_count = collected_subtasks.len() as u32;

    if !collected_subtasks.is_empty() {
        task.subtasks = Some(collected_subtasks);
    }

    // M3.1: initialize the session's durable working-memory artifact when
    // the task is non-trivial. Trivial tasks skip the upsert.
    if plan.is_non_trivial && !plan.subtasks.is_empty() {
        let active_plan_summary = plan
            .subtasks
            .iter()
            .map(|s| s.title.clone())
            .collect::<Vec<_>>()
            .join("; ");
        let memory = SessionMemory::new(
            session.id.clone(),
            task.id.clone(),
            plan.description.clone(),
            active_plan_summary,
        );
        if let Err(err) = deps.memory_repo.upsert_memory(&memory) {
            log::warn!(
                "[AgentRuntime] Failed to initialize session memory for {session_id}: {err}",
                session_id = session_id,
            );
        } else {
            log::info!(
                "[AgentRuntime] Initialized session memory for {session_id} (task={task_id}, subtasks={count})",
                session_id = session_id,
                task_id = task.id,
                count = plan.subtasks.len(),
            );
        }
    }

    let initial_max_steps = max_execution_steps(initial_subtask_count);

    let mut context = ExecutionContext {
        session_id: session_id.clone(),
        task_id: task.id.clone(),
        model_name: String::new(),
        llm_context: Vec::new(),
        steps: Vec::new(),
        total_input_tokens: 0,
        total_output_tokens: 0,
        step_number: 0,
        recent_tool_calls: Vec::new(),
        current_subtask_id: first_subtask_id,
        last_injected_subtask_order: if plan.is_non_trivial { Some(0) } else { None },
        current_phase: ExecutionPhase::Bootstrapping,
        context_usage_percent: 0.0,
        consecutive_empty_tool_iterations: 0,
        current_allowed_tool_names: None,
        pending_tool_choice_override: None,
        retrieval_count_in_current_subtask: 0,
        last_completed_subtask_for_memory: None,
        memorized_failures: Vec::new(),
        max_execution_steps: initial_max_steps,
        recent_replace_failures: Vec::new(),
    };

    let tracer = deps.tracer();
    let _ = tracer.save_snapshot(&context.snapshot(ExecutionPhase::Bootstrapping, None, None));
    let _ = tracer.append_trace(
        &context.session_id,
        &context.task_id,
        None,
        0,
        RuntimeTraceType::ExecutionStarted,
        serde_json::json!({
            "requestContent": truncate_text(&request.content, 2000),
            "trustMode": trust_mode,
            "generatedTitle": plan.title,
            "generatedDescription": plan.description,
            "subtaskCount": plan.subtasks.len(),
        }),
    );
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskCreated {
            session_id: session_id.clone(),
            task,
        },
    );

    let (client, model_name) = create_llm_client(deps.app_handle, session.model_id.clone()).await?;
    context.model_name = model_name;
    context.llm_context = build_llm_context(session, request, deps.app_handle).await?;

    deps.task_repo
        .update_task_status(&context.task_id, TaskStatus::InProgress)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: session_id.clone(),
            task_id: context.task_id.clone(),
            status: TaskStatus::InProgress,
        },
    );

    let passed_details = (!plan.subtasks.is_empty()).then(|| {
        vec![format!(
            "Plan generated with {} subtask{}",
            plan.subtasks.len(),
            if plan.subtasks.len() == 1 { "" } else { "s" },
        )]
    });
    deps.harness_state_store.record_startup_verification(
        &session_id,
        passed_startup_verification("Session ready", passed_details),
    );
    let _ = emit_harness_state_updated(deps, &session.id);

    Ok(BootstrapResult { context, client })
}

/// Resumes an existing task without calling the planner or recreating
/// any task/subtask rows. Transitions the target subtask (selected by
/// `next_subtask_order`) to `InProgress`, rebuilds the `ExecutionContext`
/// around the existing task, and emits `ExecutionResumed` so the UI
/// knows to refresh rather than expect a fresh plan payload.
async fn bootstrap_continue(
    deps: &ExecutionDeps<'_>,
    session: &AgentSession,
    request: &AgentRequest,
    trust_mode: crate::core::agent::types::TrustMode,
    prior: PriorExecutionState,
    next_subtask_order: i32,
) -> Result<BootstrapResult, AgentRuntimeError> {
    let session_id = session.id.0.clone();
    let task = prior.task;
    let mut subtasks = prior.subtasks;
    subtasks.sort_by_key(|s| s.order);

    // Find the subtask to resume on. Prefer the explicit order from the
    // planner decision; fall back to the first non-completed subtask so
    // a slightly stale `next_subtask_order` still lands somewhere sane.
    let resume_subtask = subtasks
        .iter()
        .find(|s| s.order == next_subtask_order)
        .or_else(|| {
            subtasks
                .iter()
                .find(|s| s.status != TaskStatus::Completed)
        })
        .cloned();

    let Some(mut resume_subtask) = resume_subtask else {
        return Err(AgentRuntimeError::Repository(
            "Continue decision but no non-completed subtask to resume on".to_string(),
        ));
    };

    // Transition the resume subtask to InProgress if it isn't already
    // (recovery preserves InProgress, so this is usually a no-op; we
    // still call it so a Pending subtask promotes cleanly).
    if resume_subtask.status != TaskStatus::InProgress {
        deps.task_repo
            .update_subtask_status(&resume_subtask.id, TaskStatus::InProgress)
            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
        resume_subtask.status = TaskStatus::InProgress;
        resume_subtask.started_at.get_or_insert_with(Utc::now);
        let _ = deps.app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::SubtaskUpdated {
                session_id: session_id.clone(),
                subtask: resume_subtask.clone(),
            },
        );
    }

    // Make sure the root task shows InProgress too. Recovery leaves the
    // task untouched, so this is typically a no-op — but on an older
    // session that predates the cursor-preserving recovery path, the
    // task might still be marked Failed.
    if task.status != TaskStatus::InProgress {
        deps.task_repo
            .update_task_status(&task.id, TaskStatus::InProgress)
            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;
        let _ = deps.app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::TaskUpdated {
                session_id: session_id.clone(),
                task_id: task.id.clone(),
                status: TaskStatus::InProgress,
            },
        );
    }

    let initial_max_steps = max_execution_steps(subtasks.len() as u32);

    let mut context = ExecutionContext {
        session_id: session_id.clone(),
        task_id: task.id.clone(),
        model_name: String::new(),
        llm_context: Vec::new(),
        steps: Vec::new(),
        total_input_tokens: 0,
        total_output_tokens: 0,
        step_number: 0,
        recent_tool_calls: Vec::new(),
        current_subtask_id: Some(resume_subtask.id.clone()),
        last_injected_subtask_order: None,
        current_phase: ExecutionPhase::Bootstrapping,
        context_usage_percent: 0.0,
        consecutive_empty_tool_iterations: 0,
        current_allowed_tool_names: None,
        pending_tool_choice_override: None,
        retrieval_count_in_current_subtask: 0,
        last_completed_subtask_for_memory: None,
        memorized_failures: Vec::new(),
        max_execution_steps: initial_max_steps,
        recent_replace_failures: Vec::new(),
    };

    let tracer = deps.tracer();
    let _ = tracer.save_snapshot(&context.snapshot(ExecutionPhase::Bootstrapping, None, None));
    let _ = tracer.append_trace(
        &context.session_id,
        &context.task_id,
        Some(&resume_subtask.id),
        0,
        RuntimeTraceType::ExecutionStarted,
        serde_json::json!({
            "requestContent": truncate_text(&request.content, 2000),
            "trustMode": trust_mode,
            "resumed": true,
            "subtaskCount": subtasks.len(),
            "nextSubtaskOrder": next_subtask_order,
        }),
    );

    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ExecutionResumed {
            session_id: session_id.clone(),
            task_id: task.id.clone(),
            next_subtask_id: Some(resume_subtask.id.clone()),
        },
    );

    let (client, model_name) = create_llm_client(deps.app_handle, session.model_id.clone()).await?;
    context.model_name = model_name;
    context.llm_context = build_llm_context(session, request, deps.app_handle).await?;

    let passed_details = (!subtasks.is_empty()).then(|| {
        vec![format!(
            "Resumed plan with {} subtask{}",
            subtasks.len(),
            if subtasks.len() == 1 { "" } else { "s" },
        )]
    });
    deps.harness_state_store.record_startup_verification(
        &session_id,
        passed_startup_verification("Session ready", passed_details),
    );
    let _ = emit_harness_state_updated(deps, &session.id);

    Ok(BootstrapResult { context, client })
}
