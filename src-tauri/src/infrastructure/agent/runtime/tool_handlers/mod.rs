//! Per-tool handler implementations for the agent permissions pipeline.
//!
//! Each handler module owns one `agent_*` meta-tool and exposes both
//! the existing free-function entry point (unchanged, for direct
//! callers) and a zero-sized unit struct implementing
//! [`dispatcher::ToolHandler`]. The handler instances are held as
//! `&'static dyn ToolHandler` inside [`registry`] so
//! [`handle_agent_tool_call`] is a single hash-map lookup plus a
//! virtual call — no hardcoded `match` arm to edit when a new meta-tool
//! lands.
//!
//! Returns `None` when the call is for a regular (non-agent) tool that
//! the shared `ToolRegistry` should execute; callers rely on this
//! signal to fall through to the generic tool path, so the behaviour
//! matches the pre-refactor `match _ => None` arm exactly.

pub(crate) mod contract_verification;

mod advance_subtask;
mod dispatcher;
mod mark_subtask_status;
mod narrate;
mod plan_subtasks;
mod registry;
pub(crate) mod shared;

use tauri::AppHandle;

use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::AgentSession;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::runtime::session_scope::RuntimeSessionScope;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;

use dispatcher::ToolHandlerContext;

/// Dispatches an agent meta-tool call to the matching handler.
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
    let handler = registry::lookup(call.name.as_str())?;
    let mut ctx = ToolHandlerContext {
        session,
        task_id,
        current_subtask_id,
        app_handle,
        session_repo,
        task_repo,
        telemetry_repo,
        scope,
    };
    Some(handler.handle(call, &mut ctx))
}
