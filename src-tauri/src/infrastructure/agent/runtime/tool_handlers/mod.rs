//! Per-tool handler implementations for the agent permissions pipeline.
//!
//! Each handler module owns one `agent_*` meta-tool. The dispatcher
//! below picks the right handler by tool name and forwards arguments,
//! returning `None` when the call is for a regular (non-agent) tool
//! that the shared `ToolRegistry` should execute.
//!
//! The dispatcher exports the same shape the pre-refactor
//! `handle_agent_tool_call` used, preserving the public surface.

pub(crate) mod contract_verification;

mod advance_subtask;
mod mark_subtask_status;
mod narrate;
mod plan_subtasks;
pub(crate) mod shared;

use tauri::AppHandle;

use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::AgentSession;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::runtime::session_scope::RuntimeSessionScope;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;

/// Dispatches an agent meta-tool call to the matching handler module.
/// Returns `None` when the call is not for an agent meta-tool, letting
/// the caller fall through to the shared `ToolRegistry`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_agent_tool_call(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    current_subtask_id: Option<&str>,
    app_handle: &AppHandle,
    session_repo: &dyn SessionRepository,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    scope: &mut RuntimeSessionScope,
) -> Option<OrchestratorToolResult> {
    match call.name.as_str() {
        "agent_plan_subtasks" => Some(plan_subtasks::handle_plan_subtasks(
            call,
            session,
            task_id,
            app_handle,
            task_repo,
            telemetry_repo,
            scope,
        )),
        "agent_advance_to_next_subtask" => Some(advance_subtask::handle_advance_subtask(
            call,
            session,
            task_id,
            app_handle,
            task_repo,
            telemetry_repo,
            scope,
        )),
        "agent_mark_subtask_status" => Some(mark_subtask_status::handle_mark_subtask_status(
            call,
            session,
            task_id,
            app_handle,
            session_repo,
            task_repo,
            telemetry_repo,
            scope,
        )),
        "agent_narrate" => Some(narrate::handle_narrate(
            call,
            session,
            task_id,
            current_subtask_id,
            app_handle,
            telemetry_repo,
        )),
        _ => None,
    }
}
