//! Application-owned execution coordinator.

use std::sync::Arc;

use crate::core::agent::permissions::PermissionResolver;
use crate::core::agent::repositories::{
    ExecutionRepository, RuntimeTraceRepository, SessionRepository, TaskRepository,
};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::{AgentExecutionResult, AgentRequest};
use crate::core::agent::types::TrustMode;
use crate::core::llm::entities::ToolDefinition;
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::runtime::AgentRuntimeError;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::ToolRegistry;
use tauri::AppHandle;

use super::bootstrap::start_execution;
use super::context_phase::resolve_context;
use super::deps::ExecutionDeps;
use super::llm_phase::call_llm;
use super::response_phase::{handle_response, ResponseDecision};
use super::step_gate::{begin_step, StepGateOutcome};
use super::tool_phase::{run_tools, ToolPhaseOutcome};

#[allow(clippy::too_many_arguments)]
pub async fn execute(
    session: &AgentSession,
    request: AgentRequest,
    agent_system_prompt: &str,
    trust_mode: TrustMode,
    app_handle: &AppHandle,
    session_repo: &dyn SessionRepository,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    execution_repo: &dyn ExecutionRepository,
    trace_repo: &dyn RuntimeTraceRepository,
    approval_queue: &Arc<ApprovalQueue>,
    tool_registry: &Arc<ToolRegistry>,
    file_snapshot_registry: &Arc<FileSnapshotRegistry>,
    all_tool_definitions: &[ToolDefinition],
) -> Result<AgentExecutionResult, AgentRuntimeError> {
    let deps = ExecutionDeps {
        app_handle,
        session_repo,
        task_repo,
        telemetry_repo,
        execution_repo,
        trace_repo,
        approval_queue,
        tool_registry,
        file_snapshot_registry,
        all_tool_definitions,
    };

    let bootstrap = start_execution(&deps, session, &request, trust_mode).await?;
    let mut ctx = bootstrap.context;
    let client = bootstrap.client;
    let permission_resolver = PermissionResolver::new(trust_mode);

    loop {
        match begin_step(&deps, &mut ctx)? {
            StepGateOutcome::Proceed => {}
            StepGateOutcome::Cancelled => return Err(AgentRuntimeError::Cancelled),
        }

        let resolved = resolve_context(&deps, &mut ctx, session, &request, agent_system_prompt).await?;
        let llm_result = call_llm(&deps, &mut ctx, &resolved, client.as_ref()).await?;

        match handle_response(&deps, &mut ctx, session, llm_result)? {
            ResponseDecision::Finished(result) => return Ok(result),
            ResponseDecision::ExecuteTools(response) => {
                match run_tools(&deps, &mut ctx, session, response, &permission_resolver).await? {
                    ToolPhaseOutcome::Continue => {}
                    ToolPhaseOutcome::Cancelled => return Err(AgentRuntimeError::Cancelled),
                }
            }
        }
    }
}
