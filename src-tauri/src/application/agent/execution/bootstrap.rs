//! Handles the initial setup and bootstrapping of an agent's execution.
//!
//! This module is responsible for preparing the execution environment when a new
//! request is received, including planning the task, creating the necessary
//! data structures, and initializing the execution context.

use crate::application::agent::harness::{
    passed_startup_verification, running_startup_verification,
};
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::{AgentSession, SessionState};
use crate::core::agent::task::{AgentRequest, AgentTask, TaskStatus};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::client::{build_llm_context, create_llm_client};
use crate::infrastructure::agent::runtime::AgentRuntimeError;
use chrono::Utc;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::harness_events::emit_harness_state_updated;
use super::state::ExecutionContext;
use super::termination::update_session_state;
use super::utils::truncate_text;
use crate::application::agent::service::planner::{plan_new_task, subtask_plan_to_agent_task};

/// The result of a successful bootstrap process.
pub struct BootstrapResult {
    /// The initialized execution context.
    pub context: ExecutionContext,
    /// The client for communicating with the LLM.
    pub client: Box<dyn crate::infrastructure::llm::LlmClient>,
}

/// Starts the execution of an agent for a given session and request.
///
/// This function performs the following steps:
/// 1. Updates the session state to `Running`.
/// 2. Emits an `ExecutionStarted` event.
/// 3. Uses a single-shot LLM call to plan the task, generating a title, description, and subtasks.
/// 4. Creates a root task with the generated title and description.
/// 5. Creates all subtasks, marking the first one as `InProgress`.
/// 6. Initializes the `ExecutionContext` and saves an initial snapshot.
/// 7. Emits `TaskCreated` and `SubtaskCreated` events to notify the frontend.
/// 8. Creates the LLM client and builds the initial LLM context.
/// 9. Updates the root task status to `InProgress` and emits a `TaskUpdated` event.
///
/// # Arguments
///
/// * `deps` - The execution dependencies, including repositories and application handles.
/// * `session` - The agent session for which the execution is starting.
/// * `request` - The user's request that initiated the execution.
/// * `trust_mode` - The trust mode for the execution, which determines permission levels.
///
/// # Returns
///
/// A `Result` containing a `BootstrapResult` on success, or an `AgentRuntimeError` on failure.
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

    // Step 1: Single-shot LLM planning to generate title, description, and subtasks
    let plan = plan_new_task(
        deps.app_handle,
        &request.content,
        session.model_id.as_deref(),
    )
    .await
    .map_err(|e| AgentRuntimeError::Repository(format!("Planning failed: {e}")))?;

    // Step 2: Create root task with LLM-generated title and description.
    // Store the generated title/description on the AgentRequest's context so
    // the task repo serializes them alongside the request content.
    let mut task = AgentTask::new(session.id.clone(), request.clone());
    task.title = Some(plan.title.clone());

    // Emit TaskPlanGenerated event so frontend immediately knows the LLM-generated title
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

    // Step 3: Create all subtasks at once if non-trivial
    let mut first_subtask_id: Option<String> = None;
    let mut collected_subtasks: Vec<crate::core::agent::task::SubTask> = Vec::new();
    if plan.is_non_trivial && !plan.subtasks.is_empty() {
        for subtask_plan in &plan.subtasks {
            let mut subtask =
                subtask_plan_to_agent_task(subtask_plan, &task.session_id.0, &task.id);

            // Mark first subtask as InProgress, others as Pending
            if subtask_plan.order == 0 {
                subtask.status = TaskStatus::InProgress;
                subtask.started_at = Some(Utc::now());
                first_subtask_id = Some(subtask.id.clone());
            }

            deps.task_repo
                .create_subtask(&subtask)
                .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

            // Collect for TaskCreated payload before emitting
            collected_subtasks.push(subtask.clone());

            // Emit SubtaskCreated for status-tracking purposes
            let _ = deps.app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::SubtaskCreated {
                    session_id: session_id.clone(),
                    subtask,
                },
            );
        }
    }

    // Set LLM-generated title and subtasks on the task before emitting TaskCreated
    if !collected_subtasks.is_empty() {
        task.subtasks = Some(collected_subtasks);
    }

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

    // Emit TaskUpdated event so frontend knows the root task is now InProgress
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: session_id.clone(),
            task_id: context.task_id.clone(),
            status: crate::core::agent::task::TaskStatus::InProgress,
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
