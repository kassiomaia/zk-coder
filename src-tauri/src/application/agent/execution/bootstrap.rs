use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::{AgentSession, SessionState};
use crate::core::agent::task::{AgentRequest, AgentTask};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::client::{build_llm_context, create_llm_client};
use crate::infrastructure::agent::runtime::AgentRuntimeError;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::state::ExecutionContext;
use super::termination::update_session_state;
use super::utils::{derive_task_title, truncate_text};

pub struct BootstrapResult {
    pub context: ExecutionContext,
    pub client: Box<dyn crate::infrastructure::llm::LlmClient>,
}

pub async fn start_execution(
    deps: &ExecutionDeps<'_>,
    session: &AgentSession,
    request: &AgentRequest,
    trust_mode: crate::core::agent::types::TrustMode,
) -> Result<BootstrapResult, AgentRuntimeError> {
    let session_id = session.id.0.clone();

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

    let task = AgentTask::new(session.id.clone(), request.clone());
    let core_task = crate::core::agent::entities::AgentTask {
        id: task.id.clone(),
        session_id: task.session_id.0.clone(),
        parent_task_id: None,
        title: derive_task_title(&task.request.content),
        description: Some(task.request.content.clone()),
        status: crate::core::agent::entities::TaskStatus::Pending,
        order: 0,
        mode: Some(crate::core::agent::entities::TaskMode::FullAccess),
    };
    deps.task_repo
        .create_task(&core_task)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

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
        current_subtask_id: None,
        last_injected_subtask_order: None,
        current_phase: ExecutionPhase::Bootstrapping,
        context_usage_percent: 0.0,
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
        }),
    );
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskCreated {
            session_id,
            task,
        },
    );

    let (client, model_name) = create_llm_client(deps.app_handle).await?;
    context.model_name = model_name;
    context.llm_context = build_llm_context(session, request, deps.app_handle).await?;

    deps.task_repo
        .update_task_status(
            &context.task_id,
            crate::core::agent::entities::TaskStatus::InProgress,
        )
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    Ok(BootstrapResult { context, client })
}
