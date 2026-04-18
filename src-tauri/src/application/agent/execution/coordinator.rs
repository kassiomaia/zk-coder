//! Provides the application-owned execution coordinator for agents.
//!
//! This module contains the main `execute` function that orchestrates the
//! entire lifecycle of an agent's execution, from bootstrapping to completion.

use std::sync::Arc;
use tauri::AppHandle;

use crate::application::agent::harness::HarnessStateStore;
use crate::core::agent::permissions::PermissionResolver;
use crate::core::agent::execution::RuntimeTraceType;
use crate::core::agent::memory::MemoryRepository;
use crate::core::agent::repositories::{
    ExecutionRepository, RuntimeTraceRepository, SessionRepository, TaskRepository,
};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::{AgentExecutionResult, AgentRequest};
use crate::core::agent::types::TrustMode;
use crate::core::chat::services::ChatService;
use crate::core::llm::entities::{ChatMessage as Message, ToolChoiceMode, ToolDefinition};
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::runtime::AgentRuntimeError;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::ToolRegistry;

use super::bootstrap::start_execution;
use super::context_phase::resolve_context;
use super::deps::ExecutionDeps;
use super::llm_phase::call_llm;
use super::response_phase::{handle_response, ResponseDecision};
use super::step_gate::{begin_step, StepGateOutcome};
use super::tool_phase::{run_tools, ToolPhaseOutcome};
use super::yolo::ChatYoloModeProvider;

/// The main execution loop for an agent.
///
/// This function coordinates the entire execution process, which consists of the
/// following steps in a loop:
/// 1.  **Bootstrap**: Initializes the execution context, plans the task, and sets up the environment.
/// 2.  **Step Gate**: Checks for termination conditions before starting a new step.
/// 3.  **Context Resolution**: Prepares the context for the LLM call.
/// 4.  **LLM Call**: Makes a call to the Large Language Model.
/// 5.  **Response Handling**: Processes the LLM's response, which may lead to finishing the execution or calling tools.
/// 6.  **Tool Execution**: If necessary, executes the tools requested by the LLM.
///
/// # Arguments
///
/// * `session` - The agent session for which the execution is running.
/// * `request` - The user's request that initiated the execution.
/// * `agent_system_prompt` - The base system prompt for the agent.
/// * `trust_mode` - The trust mode for the execution.
/// * `app_handle` - A handle to the Tauri application.
/// * `session_repo` - The repository for managing agent sessions.
/// * `task_repo` - The repository for managing agent tasks.
/// * `telemetry_repo` - The repository for agent telemetry.
/// * `execution_repo` - The repository for execution checkpoints.
/// * `trace_repo` - The repository for runtime traces.
/// * `approval_queue` - The queue for handling pending approvals.
/// * `tool_registry` - The registry of available tools.
/// * `file_snapshot_registry` - The registry for file snapshots.
/// * `all_tool_definitions` - A slice of all available tool definitions.
///
/// # Returns
///
/// A `Result` containing the `AgentExecutionResult` on success, or an `AgentRuntimeError` on failure.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    session: &AgentSession,
    request: AgentRequest,
    agent_system_prompt: &str,
    trust_mode: TrustMode,
    app_handle: &AppHandle,
    session_repo: &dyn SessionRepository,
    chat_service: &dyn ChatService,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    execution_repo: &dyn ExecutionRepository,
    trace_repo: &dyn RuntimeTraceRepository,
    memory_repo: &dyn MemoryRepository,
    approval_queue: &Arc<ApprovalQueue>,
    tool_registry: &Arc<ToolRegistry>,
    file_snapshot_registry: &Arc<FileSnapshotRegistry>,
    harness_state_store: &Arc<dyn HarnessStateStore>,
    all_tool_definitions: &[ToolDefinition],
) -> Result<AgentExecutionResult, AgentRuntimeError> {
    let deps = ExecutionDeps {
        app_handle,
        session_repo,
        chat_service,
        task_repo,
        telemetry_repo,
        execution_repo,
        trace_repo,
        memory_repo,
        approval_queue,
        tool_registry,
        file_snapshot_registry,
        harness_state_store,
        all_tool_definitions,
    };

    let bootstrap = start_execution(&deps, session, &request, trust_mode).await?;
    let mut ctx = bootstrap.context;
    let client = bootstrap.client;
    let permission_resolver = PermissionResolver::new(trust_mode);
    let yolo_mode_provider = ChatYoloModeProvider::new(deps.chat_service);

    loop {
        match begin_step(&deps, &mut ctx)? {
            StepGateOutcome::Proceed => {}
            StepGateOutcome::Cancelled => return Err(AgentRuntimeError::Cancelled),
        }

        let resolved =
            resolve_context(&deps, &mut ctx, session, &request, agent_system_prompt, client.as_ref()).await?;

        // All subtasks are in a terminal state — ask the LLM for a finish
        // summary. Keep the session-visible tool catalog stable (M2.2) but
        // force `ToolChoiceMode::None` for this step (M2.3) so the model is
        // explicitly told not to call any tool. Also set the runtime-level
        // allow-list to empty so any call that slips through is rejected as
        // disallowed by mode.
        if resolved.all_subtasks_completed {
            log::info!(
                "[AgentExecution] All subtasks completed for task {} – requesting finish summary",
                ctx.task_id
            );
            ctx.llm_context.push(Message::user(
                "All subtasks have been completed. Provide a concise summary of what was accomplished, key findings, and any important results.",
            ));
            ctx.current_allowed_tool_names = Some(Vec::new());
            ctx.pending_tool_choice_override = Some(ToolChoiceMode::None);
            let _ = deps.tracer().append_trace(
                &ctx.session_id,
                &ctx.task_id,
                ctx.current_subtask_id.as_deref(),
                ctx.step_number,
                RuntimeTraceType::LlmResponseClassified,
                serde_json::json!({
                    "forcedToolChoiceMode": "none",
                    "reason": "all_subtasks_completed_final_summary",
                }),
            );
            let llm_result = call_llm(&deps, &mut ctx, &resolved, client.as_ref()).await?;
            match handle_response(&deps, &mut ctx, session, llm_result).await? {
                ResponseDecision::Finished(result) => return Ok(result),
                _ => continue,
            }
        }

        let llm_result = call_llm(&deps, &mut ctx, &resolved, client.as_ref()).await?;

        match handle_response(&deps, &mut ctx, session, llm_result).await? {
            ResponseDecision::Finished(result) => return Ok(result),
            ResponseDecision::ContinueWithNextSubtask => continue,
            ResponseDecision::ExecuteTools(response) => {
                match run_tools(
                    &deps,
                    &mut ctx,
                    session,
                    response,
                    &permission_resolver,
                    &yolo_mode_provider,
                )
                .await?
                {
                    ToolPhaseOutcome::Continue => {}
                    ToolPhaseOutcome::Cancelled => return Err(AgentRuntimeError::Cancelled),
                }
            }
        }
    }
}
