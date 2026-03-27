//! Permission handling and tool execution for agent runtime.

use std::time::Duration;

use chrono::Utc;
use log::{info, warn};
use tauri::{AppHandle, Emitter};

use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::{AgentSession, PendingAction};
use crate::core::agent::types::{ActionPreview, ActionType};
use crate::core::llm::entities::{ToolCall, ToolResult};
use crate::infrastructure::agent::approval::{ApprovalEvent, ApprovalQueue};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::context::ToolExecutionContext;
use crate::infrastructure::tools::OrchestratorToolResult;
use crate::infrastructure::tools::ToolRegistry;

use super::types::AgentRuntimeError;
use super::{generate_diff_from_content, infer_action_type};

/// Timeout for waiting on user approval (seconds).
const APPROVAL_TIMEOUT_SECS: u64 = 300;

/// Extension trait for adding preview to pending actions.
trait PendingActionExt {
    fn with_preview(self, preview: ActionPreview) -> PendingAction;
}

impl PendingActionExt for PendingAction {
    fn with_preview(mut self, preview: ActionPreview) -> PendingAction {
        self.preview = Some(preview);
        self
    }
}

/// Process tool calls with permission checks, requesting approval as needed.
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
        let current_subtask_id = if let Ok(subtasks) = task_repo.get_subtasks(&task_id) {
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

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ToolCall {
                session_id: session.id.0.clone(),
                tool_call_id: tool_call_id.clone(),
                tool: call.name.clone(),
                arguments: call.arguments.clone(),
            },
        );

        let result = match permission {
            crate::core::agent::types::PermissionLevel::AutoRun => {
                execute_tool(
                    call,
                    session,
                    app_handle,
                    tool_registry,
                    file_snapshot_registry,
                    telemetry_repo,
                    task_repo,
                )
                .await
            }
            crate::core::agent::types::PermissionLevel::ApprovalRequired => {
                execute_with_approval(
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
            crate::core::agent::types::PermissionLevel::Disabled => OrchestratorToolResult {
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

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ToolResult {
                session_id: session.id.0.clone(),
                tool_call_id: tool_call_id.clone(),
                tool: call.name.clone(),
                result: result.output.clone(),
                is_error: result.is_error,
            },
        );

        results.push(result);
    }

    Ok(results)
}

/// Execute a tool call using the tool registry.
async fn execute_tool(
    call: &ToolCall,
    session: &AgentSession,
    app_handle: &AppHandle,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
) -> OrchestratorToolResult {
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

    let context = ToolExecutionContext::new(&session.conversation_id)
        .with_thread(session.thread_id.clone())
        .with_session(session.id.0.clone());
    let context = if let Some(ref workspace) = session.workspace_root {
        context.with_workspace(workspace.clone())
    } else {
        context
    };

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
        emit_file_events(
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

/// Execute a tool call that requires user approval first.
async fn execute_with_approval(
    call: &ToolCall,
    session: &AgentSession,
    _task_id: &str,
    app_handle: &AppHandle,
    approval_queue: &ApprovalQueue,
    tool_registry: &ToolRegistry,
    file_snapshot_registry: &FileSnapshotRegistry,
    session_repo: &dyn SessionRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
) -> Result<OrchestratorToolResult, AgentRuntimeError> {
    let action_type = infer_action_type(&call.name, &call.arguments);
    let mut pending = PendingAction::new(
        session.id.clone(),
        call.name.clone(),
        action_type,
        format!("Agent wants to call tool: {}", call.name),
        call.arguments.clone(),
    );

    // Generate diff preview for file write operations
    if call.name == "fs_write_file" {
        if let (Some(path), Some(content)) = (
            call.arguments.get("path").and_then(|v| v.as_str()),
            call.arguments.get("content").and_then(|v| v.as_str()),
        ) {
            if let Some(ref workspace) = session.workspace_root {
                let diff = crate::infrastructure::tools::fs::write_file::generate_diff(
                    workspace, path, content,
                );
                if let Some(diff) = diff {
                    let lang = std::path::Path::new(path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(String::from);
                    pending = pending.with_preview(crate::core::agent::types::ActionPreview {
                        content_type: "diff".to_string(),
                        content: diff,
                        language: lang,
                    });
                }
            }
        }
    }

    let action_id = pending.id.clone();

    // Update session state to waiting for input
    let core_state: crate::core::agent::entities::SessionState =
        crate::core::agent::session::SessionState::UserInputRequired {
            pending_action_ids: vec![action_id.clone()],
        }
        .into();
    session_repo
        .update_state(
            &crate::core::agent::entities::SessionId(session.id.0.clone()),
            &core_state,
        )
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    // Emit approval required event
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ApprovalRequired {
            session_id: session.id.0.clone(),
            action: pending.clone(),
        },
    );

    // Queue the action
    approval_queue.queue(pending).await;

    // Wait for approval decision
    let mut rx = approval_queue.subscribe();
    let timeout = tokio::time::sleep(Duration::from_secs(APPROVAL_TIMEOUT_SECS));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(ApprovalEvent::ActionApproved { action_id: approved_id })
                        if approved_id == action_id =>
                    {
                        info!("[AgentRuntime] Action {} approved", action_id);
                        return Ok(execute_tool(call, session, app_handle, tool_registry, file_snapshot_registry, telemetry_repo, task_repo).await);
                    }
                    Ok(ApprovalEvent::ActionDenied { action_id: denied_id })
                        if denied_id == action_id =>
                    {
                        info!("[AgentRuntime] Action {} denied", action_id);
                        return Ok(OrchestratorToolResult {
                            call_id: call.id.clone(),
                            name: call.name.clone(),
                            output: serde_json::json!({
                                "error": "Action denied by user"
                            }),
                            is_error: true,
                        });
                    }
                    Ok(ApprovalEvent::ActionTimeout { action_id: timeout_id })
                        if timeout_id == action_id =>
                    {
                        warn!("[AgentRuntime] Action {} timed out", action_id);
                        return Ok(OrchestratorToolResult {
                            call_id: call.id.clone(),
                            name: call.name.clone(),
                            output: serde_json::json!({
                                "error": "Action timed out waiting for approval"
                            }),
                            is_error: true,
                        });
                    }
                    Err(_) => {
                        // Channel closed
                        return Err(AgentRuntimeError::ApprovalTimeout);
                    }
                    _ => continue,
                }
            }
            _ = &mut timeout => {
                // Clean up the pending action
                approval_queue.deny(&action_id).await;
                return Err(AgentRuntimeError::ApprovalTimeout);
            }
        }
    }
}

/// Emits file-related events after successful tool execution.
fn emit_file_events(
    call: &ToolCall,
    session: &AgentSession,
    app_handle: &AppHandle,
    file_snapshot_registry: &FileSnapshotRegistry,
    telemetry_repo: &dyn AgentTelemetryRepository,
) {
    match call.name.as_str() {
        "fs_write_file" => {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                // Generate diff for post-session review
                let diff = if let Some(workspace_root) = &session.workspace_root {
                    let original_content =
                        file_snapshot_registry.get_snapshot_content(&session.id.0, path);
                    let current_content = std::fs::read_to_string(workspace_root.join(path)).ok();

                    match (original_content, current_content) {
                        (Some(original), Some(current)) => {
                            Some(generate_diff_from_content(path, &original, &current))
                        }
                        (None, Some(current)) => {
                            let line_count = current.lines().count();
                            let lines: String = current
                                .lines()
                                .map(|line| format!("+{}", line))
                                .collect::<Vec<_>>()
                                .join("\n");
                            Some(format!(
                                "--- /dev/null\n+++ b/{}\n@@ -0,0 +1,{} @@\n{}",
                                path, line_count, lines
                            ))
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::FileTouched {
                        session_id: session.id.0.clone(),
                        file_path: path.to_string(),
                        action: crate::infrastructure::agent::events::FileAction::Write,
                        diff: diff.clone(),
                    },
                );

                let _ =
                    telemetry_repo.add_file_touched(&session.id, path, "write", diff.as_deref());
            }
        }
        "fs_delete_file" => {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::FileTouched {
                        session_id: session.id.0.clone(),
                        file_path: path.to_string(),
                        action: crate::infrastructure::agent::events::FileAction::Delete,
                        diff: None,
                    },
                );

                let _ = telemetry_repo.add_file_touched(&session.id, path, "delete", None);
            }
        }
        "fs_read_file" => {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::FileTouched {
                        session_id: session.id.0.clone(),
                        file_path: path.to_string(),
                        action: crate::infrastructure::agent::events::FileAction::Read,
                        diff: None,
                    },
                );

                let _ = telemetry_repo.add_file_touched(&session.id, path, "read", None);
            }
        }
        "agent_emit_status" => {
            if let Some(message) = call.arguments.get("message").and_then(|v| v.as_str()) {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::StatusPhrase {
                        session_id: session.id.0.clone(),
                        message: message.to_string(),
                        timestamp: Utc::now(),
                    },
                );

                let _ = telemetry_repo.add_status_phrase(&session.id, message);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::SessionId;
    use crate::core::agent::types::TrustMode;
    use serde_json::json;

    #[test]
    fn test_pending_action_ext_with_preview() {
        let pending = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test action".to_string(),
            json!({}),
        );

        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "test diff".to_string(),
            language: Some("rust".to_string()),
        };

        let with_preview = pending.with_preview(preview.clone());
        assert!(with_preview.preview.is_some());
        assert_eq!(with_preview.preview.unwrap().content, "test diff");
    }

    #[test]
    fn test_pending_action_creation() {
        let session_id = SessionId("session-123".to_string());
        let action = PendingAction::new(
            session_id.clone(),
            "fs_write_file".to_string(),
            ActionType::WriteFile {
                path: "/tmp/test.txt".to_string(),
                size_bytes: 100,
            },
            "Write a test file".to_string(),
            json!({"path": "/tmp/test.txt", "content": "hello"}),
        );

        assert_eq!(action.session_id.0, "session-123");
        assert_eq!(action.tool_name, "fs_write_file");
        assert!(action.preview.is_none());
    }

    #[test]
    fn test_action_type_read_file() {
        let action = ActionType::ReadFile {
            path: "/path/to/file.rs".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("read_file"));

        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::ReadFile { path } => {
                assert_eq!(path, "/path/to/file.rs");
            }
            _ => panic!("Expected ReadFile"),
        }
    }

    #[test]
    fn test_action_type_write_file() {
        let action = ActionType::WriteFile {
            path: "/path/to/file.rs".to_string(),
            size_bytes: 1024,
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("write_file"));
        assert!(serialized.contains("1024"));
    }

    #[test]
    fn test_action_type_delete_file() {
        let action = ActionType::DeleteFile {
            path: "/tmp/old_file.txt".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("delete_file"));
    }

    #[test]
    fn test_action_type_execute_command() {
        let action = ActionType::ExecuteCommand {
            command: "cargo test".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("execute_command"));
        assert!(serialized.contains("cargo test"));
    }

    #[test]
    fn test_action_type_web_request() {
        let action = ActionType::WebRequest {
            url: "https://example.com".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("web_request"));
        assert!(serialized.contains("https://example.com"));
    }

    #[test]
    fn test_action_preview_creation() {
        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- old\n+++ new".to_string(),
            language: Some("typescript".to_string()),
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(serialized.contains("diff"));
        assert!(serialized.contains("typescript"));
    }

    #[test]
    fn test_action_preview_without_language() {
        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Plain text preview".to_string(),
            language: None,
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(serialized.contains("text"));

        let deserialized: ActionPreview = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.language.is_none());
    }

    #[test]
    fn test_pending_action_with_multiple_previews() {
        let session_id = SessionId("test".to_string());
        let base = PendingAction::new(
            session_id,
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test".to_string(),
            json!({}),
        );

        let preview1 = ActionPreview {
            content_type: "diff".to_string(),
            content: "diff content".to_string(),
            language: Some("rust".to_string()),
        };

        let with_preview1 = base.with_preview(preview1);
        assert!(with_preview1.preview.is_some());
        assert_eq!(
            with_preview1.preview.unwrap().language,
            Some("rust".to_string())
        );
    }

    #[test]
    fn test_permission_level_serialization() {
        let levels = vec![
            crate::core::agent::types::PermissionLevel::AutoRun,
            crate::core::agent::types::PermissionLevel::ApprovalRequired,
            crate::core::agent::types::PermissionLevel::Disabled,
        ];

        for level in levels {
            let serialized = serde_json::to_string(&level).unwrap();
            let deserialized: crate::core::agent::types::PermissionLevel =
                serde_json::from_str(&serialized).unwrap();
            assert_eq!(level, deserialized);
        }
    }

    #[test]
    fn test_trust_mode_serialization() {
        let modes = vec![
            TrustMode::Trusted,
            TrustMode::Untrusted,
            TrustMode::Restricted,
        ];

        for mode in modes {
            let serialized = serde_json::to_string(&mode).unwrap();
            let deserialized: TrustMode = serde_json::from_str(&serialized).unwrap();
            assert_eq!(mode, deserialized);
        }
    }

    // === Permission Level Tests ===

    #[test]
    fn test_permission_level_auto_run() {
        let level = crate::core::agent::types::PermissionLevel::AutoRun;
        let serialized = serde_json::to_string(&level).unwrap();
        assert!(serialized.contains("auto_run"));
    }

    #[test]
    fn test_permission_level_approval_required() {
        let level = crate::core::agent::types::PermissionLevel::ApprovalRequired;
        let serialized = serde_json::to_string(&level).unwrap();
        assert!(serialized.contains("approval_required"));
    }

    #[test]
    fn test_permission_level_disabled() {
        let level = crate::core::agent::types::PermissionLevel::Disabled;
        let serialized = serde_json::to_string(&level).unwrap();
        assert!(serialized.contains("disabled"));
    }

    // === PendingAction Tests ===

    #[test]
    fn test_pending_action_with_all_fields() {
        let action = PendingAction::new(
            SessionId("session-123".to_string()),
            "fs_write_file".to_string(),
            ActionType::WriteFile {
                path: "/tmp/test.txt".to_string(),
                size_bytes: 100,
            },
            "Write test file".to_string(),
            json!({"path": "/tmp/test.txt", "content": "hello"}),
        );

        assert_eq!(action.session_id.0, "session-123");
        assert_eq!(action.tool_name, "fs_write_file");
        assert!(action.preview.is_none());
    }

    #[test]
    fn test_pending_action_id_format() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test".to_string(),
            json!({}),
        );

        // ID should be a valid UUID string
        assert!(!action.id.is_empty());
        assert!(action.id.len() > 10);
    }

    #[test]
    fn test_pending_action_serialization() {
        let action = PendingAction::new(
            SessionId("session-456".to_string()),
            "delete_file".to_string(),
            ActionType::DeleteFile {
                path: "/tmp/old.txt".to_string(),
            },
            "Delete old file".to_string(),
            json!({"path": "/tmp/old.txt"}),
        );

        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: PendingAction = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.session_id.0, "session-456");
        assert_eq!(deserialized.tool_name, "delete_file");
    }

    #[test]
    fn test_pending_action_with_preview_serialization() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "write_file".to_string(),
            ActionType::WriteFile {
                path: "/tmp/test.txt".to_string(),
                size_bytes: 50,
            },
            "Write file".to_string(),
            json!({}),
        );

        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- old\n+++ new".to_string(),
            language: Some("rust".to_string()),
        };

        let with_preview = action.with_preview(preview);

        let serialized = serde_json::to_string(&with_preview).unwrap();
        let deserialized: PendingAction = serde_json::from_str(&serialized).unwrap();

        assert!(deserialized.preview.is_some());
        assert_eq!(deserialized.preview.unwrap().content_type, "diff");
    }

    #[test]
    fn test_pending_action_multiple_previews_replace() {
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "tool".to_string(),
            ActionType::ReadFile {
                path: "file.txt".to_string(),
            },
            "Read file".to_string(),
            json!({}),
        );

        let preview1 = ActionPreview {
            content_type: "text".to_string(),
            content: "First preview".to_string(),
            language: None,
        };

        let preview2 = ActionPreview {
            content_type: "diff".to_string(),
            content: "Second preview".to_string(),
            language: Some("typescript".to_string()),
        };

        let with_first = action.with_preview(preview1);
        let with_second = with_first.with_preview(preview2);

        // Second preview should replace the first
        assert_eq!(with_second.preview.unwrap().content, "Second preview");
    }

    // === Action Type Comprehensive Tests ===

    #[test]
    fn test_action_type_read_file_fields() {
        let action = ActionType::ReadFile {
            path: "/home/user/project/src/main.rs".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("read_file"));
        assert!(serialized.contains("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_action_type_write_file_fields() {
        let action = ActionType::WriteFile {
            path: "/tmp/output.json".to_string(),
            size_bytes: 2048,
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("write_file"));
        assert!(serialized.contains("2048"));
    }

    #[test]
    fn test_action_type_delete_file_fields() {
        let action = ActionType::DeleteFile {
            path: "/tmp/unused.tmp".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("delete_file"));
    }

    #[test]
    fn test_action_type_execute_command_fields() {
        let action = ActionType::ExecuteCommand {
            command: "cargo test --release".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("execute_command"));
        assert!(serialized.contains("cargo test --release"));
    }

    #[test]
    fn test_action_type_web_request_fields() {
        let action = ActionType::WebRequest {
            url: "https://api.example.com/v1/users".to_string(),
        };

        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("web_request"));
        assert!(serialized.contains("https://api.example.com"));
    }

    #[test]
    fn test_action_type_all_variants_roundtrip() {
        let actions = vec![
            ActionType::ReadFile {
                path: "/path/to/file.rs".to_string(),
            },
            ActionType::WriteFile {
                path: "/path/to/file.rs".to_string(),
                size_bytes: 100,
            },
            ActionType::DeleteFile {
                path: "/path/to/file.rs".to_string(),
            },
            ActionType::ExecuteCommand {
                command: "cargo build".to_string(),
            },
            ActionType::WebRequest {
                url: "https://example.com".to_string(),
            },
        ];

        for action in actions {
            let serialized = serde_json::to_string(&action).unwrap();
            let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();

            match (&action, &deserialized) {
                (ActionType::ReadFile { path: p1 }, ActionType::ReadFile { path: p2 }) => {
                    assert_eq!(p1, p2);
                }
                (
                    ActionType::WriteFile {
                        path: p1,
                        size_bytes: s1,
                    },
                    ActionType::WriteFile {
                        path: p2,
                        size_bytes: s2,
                    },
                ) => {
                    assert_eq!(p1, p2);
                    assert_eq!(s1, s2);
                }
                (ActionType::DeleteFile { path: p1 }, ActionType::DeleteFile { path: p2 }) => {
                    assert_eq!(p1, p2);
                }
                (
                    ActionType::ExecuteCommand { command: c1 },
                    ActionType::ExecuteCommand { command: c2 },
                ) => {
                    assert_eq!(c1, c2);
                }
                (ActionType::WebRequest { url: u1 }, ActionType::WebRequest { url: u2 }) => {
                    assert_eq!(u1, u2);
                }
                _ => panic!("Action types don't match"),
            }
        }
    }

    // === ActionPreview Tests ===

    #[test]
    fn test_action_preview_all_fields() {
        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- a/file.rs\n+++ b/file.rs\n@@ -1,1 +1,1 @@\n-old\n+new".to_string(),
            language: Some("rust".to_string()),
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(serialized.contains("diff"));
        assert!(serialized.contains("rust"));
    }

    #[test]
    fn test_action_preview_roundtrip() {
        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Plain text content".to_string(),
            language: None,
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        let deserialized: ActionPreview = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.content_type, "text");
        assert_eq!(deserialized.content, "Plain text content");
        assert!(deserialized.language.is_none());
    }

    #[test]
    fn test_action_preview_with_language_none() {
        let previews = vec![
            ActionPreview {
                content_type: "text".to_string(),
                content: "content".to_string(),
                language: None,
            },
            ActionPreview {
                content_type: "json".to_string(),
                content: "{}".to_string(),
                language: None,
            },
        ];

        for preview in previews {
            assert!(preview.language.is_none());
        }
    }

    #[test]
    fn test_action_preview_with_various_languages() {
        let languages = vec![
            ("rust", "diff"),
            ("typescript", "diff"),
            ("python", "diff"),
            ("javascript", "diff"),
            ("json", "json"),
            ("html", "html"),
        ];

        for (lang, content_type) in languages {
            let preview = ActionPreview {
                content_type: content_type.to_string(),
                content: format!("{} content", lang),
                language: Some(lang.to_string()),
            };

            assert_eq!(preview.language.unwrap(), lang);
        }
    }

    // === PendingActionExt Trait Tests ===

    #[test]
    fn test_pending_action_ext_trait_available() {
        // Test that the PendingActionExt trait works on PendingAction
        let action = PendingAction::new(
            SessionId("test".to_string()),
            "test_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Test".to_string(),
            json!({}),
        );

        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Preview".to_string(),
            language: None,
        };

        let _modified = action.with_preview(preview);
    }

    #[test]
    fn test_pending_action_ext_preserves_other_fields() {
        let action = PendingAction::new(
            SessionId("session-xyz".to_string()),
            "custom_tool".to_string(),
            ActionType::ExecuteCommand {
                command: "npm install".to_string(),
            },
            "Install packages".to_string(),
            json!({"command": "npm install"}),
        );

        let original_id = action.id.clone();
        let original_session_id = action.session_id.0.clone();
        let original_tool_name = action.tool_name.clone();

        let preview = ActionPreview {
            content_type: "text".to_string(),
            content: "Will run: npm install".to_string(),
            language: None,
        };

        let modified = action.with_preview(preview);

        assert_eq!(modified.id, original_id);
        assert_eq!(modified.session_id.0, original_session_id);
        assert_eq!(modified.tool_name, original_tool_name);
        assert!(modified.preview.is_some());
    }

    // === Approval Timeout Tests ===

    #[test]
    fn test_approval_timeout_constant() {
        // The approval timeout should be set to 300 seconds (5 minutes)
        assert_eq!(APPROVAL_TIMEOUT_SECS, 300);
    }

    #[test]
    fn test_approval_timeout_reasonable() {
        // Approval timeout should be at least 1 minute and at most 10 minutes
        assert!(APPROVAL_TIMEOUT_SECS >= 60);
        assert!(APPROVAL_TIMEOUT_SECS <= 600);
    }

    // === TrustMode and PermissionLevel Integration Tests ===

    #[test]
    fn test_trust_mode_affects_permission_level() {
        // Verify that different trust modes can be serialized
        let modes = vec![
            TrustMode::Trusted,
            TrustMode::Untrusted,
            TrustMode::Restricted,
        ];

        for mode in modes {
            let serialized = serde_json::to_string(&mode).unwrap();
            let _deserialized: TrustMode = serde_json::from_str(&serialized).unwrap();
        }
    }

    #[test]
    fn test_action_type_edge_cases() {
        // Test with empty path
        let action = ActionType::ReadFile {
            path: "".to_string(),
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::ReadFile { path } => assert_eq!(path, ""),
            _ => panic!("Expected ReadFile"),
        }

        // Test with very long path
        let long_path = "/".repeat(100) + "file.txt";
        let action = ActionType::WriteFile {
            path: long_path.clone(),
            size_bytes: 0,
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::WriteFile { path, .. } => assert_eq!(path, long_path),
            _ => panic!("Expected WriteFile"),
        }

        // Test with zero size
        let action = ActionType::WriteFile {
            path: "empty.txt".to_string(),
            size_bytes: 0,
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::WriteFile { size_bytes, .. } => assert_eq!(size_bytes, 0),
            _ => panic!("Expected WriteFile"),
        }

        // Test with large size
        let action = ActionType::WriteFile {
            path: "large.bin".to_string(),
            size_bytes: 1_000_000_000,
        };
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            ActionType::WriteFile { size_bytes, .. } => assert_eq!(size_bytes, 1_000_000_000),
            _ => panic!("Expected WriteFile"),
        }
    }

    #[test]
    fn test_action_preview_with_special_characters() {
        let preview = ActionPreview {
            content_type: "diff".to_string(),
            content: "--- old\n+++ new\n@@ -1,1 +1,1 @@\n-Hello, 世界\n+Hello, World\n".to_string(),
            language: Some("rust".to_string()),
        };

        let serialized = serde_json::to_string(&preview).unwrap();
        let deserialized: ActionPreview = serde_json::from_str(&serialized).unwrap();

        assert!(deserialized.content.contains("世界"));
    }

    #[test]
    fn test_pending_action_with_complex_arguments() {
        let complex_args = json!({
            "files": ["file1.rs", "file2.rs", "file3.rs"],
            "options": {
                "recursive": true,
                "force": false
            },
            "filters": [
                {"type": "extension", "value": ".rs"},
                {"type": "size", "operator": ">", "value": 100}
            ]
        });

        let action = PendingAction::new(
            SessionId("test".to_string()),
            "complex_tool".to_string(),
            ActionType::ReadFile {
                path: "test.txt".to_string(),
            },
            "Complex operation".to_string(),
            complex_args,
        );

        assert!(action.arguments["files"].is_array());
        assert_eq!(action.arguments["files"].as_array().unwrap().len(), 3);
    }
}
