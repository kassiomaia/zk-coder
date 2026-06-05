//! `agent_narrate` tool handler.
//!
//! Emits a narration event (or a clarification question) on behalf of
//! the agent. Extracted from `permissions.rs` in Phase 5 of the
//! permissions-decomposition refactor.

use chrono::Utc;
use tauri::{AppHandle, Emitter};

use super::dispatcher::{ToolHandler, ToolHandlerContext};
use crate::core::agent::session::AgentSession;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;

/// Zero-sized marker for the `agent_narrate` meta-tool.
pub(super) struct NarrateHandler;

/// Static instance the registry references so lookups stay allocation-free.
pub(super) static NARRATE: NarrateHandler = NarrateHandler;

impl ToolHandler for NarrateHandler {
    fn name(&self) -> &'static str {
        "agent_narrate"
    }

    fn handle(&self, call: &ToolCall, ctx: &mut ToolHandlerContext<'_>) -> OrchestratorToolResult {
        handle_narrate(
            call,
            ctx.session,
            ctx.task_id,
            ctx.current_subtask_id,
            ctx.app_handle,
            ctx.telemetry_repo,
        )
    }
}

/// Handles an `agent_narrate` tool call.
pub(super) fn handle_narrate(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    current_subtask_id: Option<&str>,
    app_handle: &AppHandle,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> OrchestratorToolResult {
    let message = call
        .arguments
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Working...");
    let intent = crate::infrastructure::agent::events::NarrationIntent::from_arg(
        call.arguments.get("intent").and_then(|v| v.as_str()),
    );

    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::AgentNarration {
            session_id: session.id.0.clone(),
            conversation_id: session.conversation_id.clone(),
            thread_id: session.thread_id.clone(),
            content: message.to_string(),
            intent,
            timestamp: Utc::now(),
        },
    );

    let _ = telemetry_repo.add_status_phrase(&session.id, message);

    // agent-clarification-resume plan — a `question` intent writes an
    // authoritative DB row plus a live reply channel. The coordinator's
    // park stage picks the row up after this tool's writeback completes
    // and parks on the channel until the user replies or the timeout
    // elapses.
    if matches!(
        intent,
        crate::infrastructure::agent::events::NarrationIntent::Question
    ) {
        if let Err(err) =
            crate::application::agent::clarification::record_pending_clarification_persistent(
                &session.id.0,
                task_id,
                current_subtask_id,
                message,
            )
        {
            log::warn!(
                "[AgentNarrate] Failed to record pending clarification for session {}: {err}",
                session.id.0
            );
        }
    }

    OrchestratorToolResult {
        call_id: call.id.clone(),
        name: call.name.clone(),
        output: serde_json::json!({
            "success": true,
            "message": "Update sent",
            "intent": match intent {
                crate::infrastructure::agent::events::NarrationIntent::Narration => "narration",
                crate::infrastructure::agent::events::NarrationIntent::Question => "question",
            }
        }),
        is_error: false,
    }
}
