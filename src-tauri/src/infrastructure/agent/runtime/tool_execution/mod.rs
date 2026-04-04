//! Tool execution orchestration and approval handling.
//!
//! This module provides:
//! - Tool execution orchestration via `process_tool_calls_with_permissions`
//! - Core tool execution logic in `executor`
//! - Permission and approval flow handling in `approval`

pub mod approval;
pub mod executor;

use tauri::{AppHandle, Emitter};

use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::AgentSession;
use crate::core::agent::types::PermissionLevel;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::events::AgentEvent;
use crate::infrastructure::agent::events::AGENT_EVENT_NAME;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::runtime::infer_action_type;
use crate::infrastructure::agent::runtime::tool_feedback::{
    build_tool_call_feedback, build_tool_result_feedback,
};
use crate::infrastructure::agent::runtime::types::AgentRuntimeError;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;
use crate::infrastructure::tools::ToolRegistry;
use log::warn;

/// Process tool calls with permission checks, requesting approval as needed.
///
/// This is the main entry point for tool execution in the agent runtime.
/// It handles:
/// - Permission checking for each tool call
/// - Auto-execution for permitted tools
/// - Approval flow for tools requiring user consent
/// - Event emission for tool calls and results
/// - Persistence of tool calls and results
///
/// # Arguments
///
/// * `tool_calls` - The tool calls to execute
/// * `session` - The current agent session
/// * `task_id` - The current task ID
/// * `permission_resolver` - Permission checker for tool calls
/// * `app_handle` - Tauri app handle for event emission
/// * `approval_queue` - Queue for approval-required actions
/// * `tool_registry` - Registry of available tools
/// * `file_snapshot_registry` - Registry for file state snapshots
/// * `session_repo` - Session repository for state persistence
/// * `telemetry_repo` - Telemetry repository for logging
/// * `task_repo` - Task repository for task/subtask data
///
/// # Returns
///
/// A vector of tool results, one per input tool call.
pub async fn process_tool_calls_with_permissions(
    tool_calls: &[ToolCall],
    session: &AgentSession,
    task_id: &str,
    permission_resolver: &crate::core::agent::permissions::PermissionResolver,
    app_handle: &AppHandle,
    approval_queue: &ApprovalQueue,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    session_repo: &dyn SessionRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
) -> Result<Vec<OrchestratorToolResult>, AgentRuntimeError> {
    let mut results = Vec::with_capacity(tool_calls.len());

    for call in tool_calls {
        let action_type = infer_action_type(&call.name, &call.arguments);
        let permission = permission_resolver.check(&call.name, &action_type);

        // Generate unique ID, persist, and emit tool call event
        let tool_call_id = uuid::Uuid::new_v4().to_string();

        // Get current subtask_id for tool call linkage
        let current_subtask_id = if let Ok(subtasks) = task_repo.get_subtasks(task_id) {
            subtasks
                .iter()
                .find(|st| st.status == crate::core::agent::entities::TaskStatus::InProgress)
                .map(|st| st.id.clone())
        } else {
            None
        };

        if let Err(e) = telemetry_repo.insert_tool_call(
            &session.id,
            &tool_call_id,
            &call.name,
            &call.arguments,
            current_subtask_id.as_deref(),
        ) {
            warn!("[AgentRuntime] Failed to persist tool call: {e}");
        }

        if let Some(content) = build_tool_call_feedback(&call.name, &call.arguments) {
            crate::infrastructure::agent::runtime::permissions::emit_persisted_streaming_message(
                app_handle,
                session,
                format!("streaming-tool-call-{tool_call_id}"),
                content.clone(),
            );
        }

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ToolCall {
                session_id: session.id.0.clone(),
                tool_call_id: tool_call_id.clone(),
                tool: call.name.clone(),
                arguments: call.arguments.clone(),
                content: build_tool_call_feedback(&call.name, &call.arguments),
            },
        );

        let result = match permission {
            PermissionLevel::AutoRun => {
                executor::execute_tool(
                    call,
                    session,
                    task_id,
                    app_handle,
                    tool_registry,
                    file_snapshot_registry,
                    telemetry_repo,
                    task_repo,
                )
                .await
            }
            PermissionLevel::ApprovalRequired => {
                approval::execute_with_approval(
                    call,
                    session,
                    task_id,
                    app_handle,
                    approval_queue,
                    tool_registry,
                    file_snapshot_registry,
                    session_repo,
                    telemetry_repo,
                    task_repo,
                )
                .await?
            }
            PermissionLevel::Disabled => OrchestratorToolResult {
                call_id: call.id.clone(),
                name: call.name.clone(),
                output: serde_json::json!({
                    "error": "This tool is disabled by the agent's permission policy"
                }),
                is_error: true,
            },
        };

        // Persist result and emit tool result event
        if let Err(e) = telemetry_repo.update_tool_call_result(
            &tool_call_id,
            &result.output,
            result.is_error,
            None,
        ) {
            warn!("[AgentRuntime] Failed to persist tool result: {e}");
        }

        if let Some(content) =
            build_tool_result_feedback(&call.name, &result.output, result.is_error)
        {
            crate::infrastructure::agent::runtime::permissions::emit_persisted_streaming_message(
                app_handle,
                session,
                format!("streaming-tool-result-{tool_call_id}"),
                content.clone(),
            );
        }

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ToolResult {
                session_id: session.id.0.clone(),
                tool_call_id: tool_call_id.clone(),
                tool: call.name.clone(),
                result: result.output.clone(),
                is_error: result.is_error,
                content: build_tool_result_feedback(&call.name, &result.output, result.is_error),
            },
        );

        results.push(result);
    }

    Ok(results)
}
