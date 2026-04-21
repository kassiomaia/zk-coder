//! Trait + context used by the static tool-handler registry.
//!
//! Every agent meta-tool implements [`ToolHandler`] on a zero-sized unit
//! struct so instances can be held as `&'static dyn ToolHandler` inside
//! the registry without any runtime allocation. [`ToolHandlerContext`]
//! bundles every dependency any handler might need; individual
//! implementations pick the subset they actually use.
//!
//! Placement note: the trait lives in `infrastructure/` rather than in
//! `core/` because `ToolHandlerContext` references framework-level
//! types (`tauri::AppHandle`, `RuntimeSessionScope`) that the domain
//! layer must not depend on. The OCP benefit — new tool = new file +
//! one registry line — is realised regardless of which layer the trait
//! lives in.

use tauri::AppHandle;

use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::AgentSession;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::runtime::session_scope::RuntimeSessionScope;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;

/// Carries every dependency the four agent meta-tool handlers need.
/// Each concrete handler picks the fields it uses; the dispatcher fills
/// every field on every call so behaviour is identical to the former
/// hardcoded `match`.
pub(super) struct ToolHandlerContext<'a> {
    pub session: &'a AgentSession,
    pub task_id: &'a str,
    pub current_subtask_id: Option<&'a str>,
    pub app_handle: &'a AppHandle,
    pub session_repo: &'a dyn SessionRepository,
    pub task_repo: &'a dyn TaskRepository,
    pub telemetry_repo: &'a dyn AgentTelemetryRepository,
    pub scope: &'a mut RuntimeSessionScope,
}

/// Contract for a single agent meta-tool handler.
///
/// The registry looks tools up by [`ToolHandler::name`], so each impl
/// must return a `'static` tool identifier that matches exactly the
/// `call.name` emitted by the model.
pub(super) trait ToolHandler: Send + Sync {
    /// The `call.name` value this handler claims. Must be `'static`
    /// because the registry stores these alongside the handler refs.
    fn name(&self) -> &'static str;

    /// Executes the tool call and returns its orchestrator-level
    /// result. Handlers are free to mutate repositories, emit events,
    /// or modify `ctx.scope` as the former free-function
    /// implementations did.
    fn handle(
        &self,
        call: &ToolCall,
        ctx: &mut ToolHandlerContext<'_>,
    ) -> OrchestratorToolResult;
}
