//! Core tool execution logic.

use tauri::AppHandle;

use crate::core::agent::repositories::TaskRepository;
use crate::core::agent::session::AgentSession;
use crate::core::llm::entities::{ToolCall, ToolResult};
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::context::ToolExecutionContext;
use crate::infrastructure::tools::OrchestratorToolResult;
use crate::infrastructure::tools::ToolRegistry;
use log::info;

/// Execute a tool call using the tool registry.
///
/// This function handles:
/// - Agent-specific tool calls (delegated to handlers)
/// - File system operations (write/replace with snapshots)
/// - Workspace state capture for commands
/// - Generic tool execution via the tool registry
///
/// # Arguments
///
/// * `call` - The tool call to execute
/// * `session` - The current agent session
/// * `task_id` - The current task ID
/// * `app_handle` - Tauri app handle for event emission
/// * `tool_registry` - Registry of available tools
/// * `file_snapshot_registry` - Registry for file state snapshots
/// * `telemetry_repo` - Telemetry repository for logging
/// * `task_repo` - Task repository for task/subtask data
///
/// # Returns
///
/// The result of the tool execution.
pub(super) async fn execute_tool(
    call: &ToolCall,
    session: &AgentSession,
    task_id: &str,
    app_handle: &AppHandle,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
) -> OrchestratorToolResult {
    // Check for agent-specific tool calls first
    if let Some(result) = crate::infrastructure::agent::runtime::tool_handlers::handle_agent_tool_call(
        call,
        session,
        task_id,
        app_handle,
        task_repo,
        telemetry_repo,
    ) {
        return result;
    }

    // Capture workspace snapshot before command execution
    if call.name == "system_execute_command" {
        if let Some(ref workspace) = session.workspace_root {
            let stats =
                file_snapshot_registry.capture_workspace_best_effort(&session.id.0, workspace);
            info!(
                "[AgentRuntime] Captured workspace snapshot before command: scanned={}, captured={}, skipped_large={}, truncated={}",
                stats.scanned_files,
                stats.captured_files,
                stats.skipped_large_files,
                stats.truncated
            );
        }
    }

    // Capture file state before write operations for rollback
    if call.name == "fs_write_file" {
        if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
            if let Some(ref workspace) = session.workspace_root {
                let target = if std::path::Path::new(path).is_absolute() {
                    std::path::PathBuf::from(path)
                } else {
                    workspace.join(path)
                };
                file_snapshot_registry.capture(&session.id.0, &target);
            }
        }
    }

    // Build tool execution context
    let context = ToolExecutionContext::new(&session.conversation_id)
        .with_thread(session.thread_id.clone())
        .with_session(session.id.0.clone());
    let context = if let Some(ref workspace) = session.workspace_root {
        context.with_workspace(workspace.clone())
    } else {
        context
    };

    // Execute the tool
    let llm_call = ToolCall {
        id: call.id.clone(),
        name: call.name.clone(),
        arguments: call.arguments.clone(),
    };

    let result: ToolResult = tool_registry
        .execute(llm_call, context, app_handle.clone())
        .await;

    // Emit file touched event for successful file operations
    if !result.is_error {
        crate::infrastructure::agent::runtime::permissions::emit_file_events(
            call,
            session,
            app_handle,
            file_snapshot_registry,
            telemetry_repo,
        );
    }

    OrchestratorToolResult {
        call_id: result.call_id,
        name: call.name.clone(),
        output: result.output,
        is_error: result.is_error,
    }
}
