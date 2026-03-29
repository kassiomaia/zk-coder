//! Infrastructure entrypoint that delegates execution ownership to application services.

use std::sync::Arc;

use tauri::AppHandle;

use crate::core::agent::repositories::{
    ExecutionRepository, RuntimeTraceRepository, SessionRepository, TaskRepository,
};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::{AgentExecutionResult, AgentRequest};
use crate::core::agent::types::TrustMode;
use crate::core::llm::entities::ToolDefinition;
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::ToolRegistry;

use super::AgentRuntimeError;

/// Execute an agent request.
///
/// This is an infrastructure entrypoint that delegates execution ownership
/// to application services.
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
    crate::application::agent::execution::coordinator::execute(
        session,
        request,
        agent_system_prompt,
        trust_mode,
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
    )
    .await
}
