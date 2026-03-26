//! Execute shell commands within the workspace.
//!
//! Runs commands in the workspace root directory, streaming stdout/stderr
//! via Tauri events. Requires approval via the agent permission system.

use std::path::PathBuf;

use log::info;
use tauri::Manager;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::agents::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};

/// Maximum command output size (512 KB) to prevent memory issues.
const MAX_OUTPUT_BYTES: usize = 512 * 1024;

/// Maximum command execution time (60 seconds).
const COMMAND_TIMEOUT_SECS: u64 = 60;

/// Arguments for command execution.
#[derive(Debug, Deserialize)]
pub struct ExecuteCommandArgs {
    /// The command to execute (e.g., "cargo build").
    pub command: String,
    /// Optional working directory relative to workspace root.
    pub working_dir: Option<String>,
}

/// Tool to execute shell commands within the workspace.
pub struct ExecuteCommandTool;

impl Tool for ExecuteCommandTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("system", "execute_command"),
            description: "Execute a shell command in the workspace directory. Returns stdout, stderr, and exit code. Output is streamed in real-time.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute (e.g., 'cargo build', 'npm test')"
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Working directory relative to workspace root (optional, defaults to workspace root)"
                    }
                },
                "required": ["command"]
            }),
            requires_workspace: true,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        context: ToolExecutionContext,
        arguments: JsonValue,
        app_handle: AppHandle,
        workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let workspace = workspace_root.ok_or_else(|| {
                ToolError::WorkspaceRequired("No workspace configured".to_string())
            })?;

            let args: ExecuteCommandArgs =
                crate::tools::error::parse_args("system_execute_command", arguments)?;

            if args.command.trim().is_empty() {
                return Err(ToolError::invalid_args("Command cannot be empty"));
            }

            // Resolve working directory
            let cwd = if let Some(ref dir) = args.working_dir {
                let resolved = workspace.join(dir);
                if !resolved.starts_with(&workspace) {
                    return Err(ToolError::PathTraversal(format!(
                        "Working directory '{}' is outside workspace",
                        dir
                    )));
                }
                resolved
            } else {
                workspace.clone()
            };

            if !cwd.exists() {
                return Err(ToolError::failed(format!(
                    "Working directory does not exist: {}",
                    cwd.display()
                )));
            }

            let session_id = context.session_id.unwrap_or_default();

            info!(
                "[ExecuteCommand] Running '{}' in '{}'",
                args.command,
                cwd.display()
            );

            // Get the session store BEFORE spawning (must be on main thread)
            let store: Option<std::sync::Arc<crate::agents::store::SessionStore>> =
                app_handle.try_state::<crate::commands::AgentState>()
                    .and_then(|s| Some(s.service.session_store().clone()));

            // Spawn the command
            let mut child = Command::new("sh")
                .arg("-c")
                .arg(&args.command)
                .current_dir(&cwd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| {
                    ToolError::failed(format!("Failed to spawn command: {}", e))
                })?;

            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            // Stream stdout and stderr concurrently
            let stdout_handle = {
                let app = app_handle.clone();
                let sid = session_id.clone();
                let store_clone = store.clone();
                tokio::spawn(async move {
                    let mut output = String::new();
                    if let Some(stdout) = stdout {
                        let mut reader = BufReader::new(stdout).lines();
                        while let Ok(Some(line)) = reader.next_line().await {
                            let _ = app.emit(
                                AGENT_EVENT_NAME,
                                AgentEvent::CommandOutput {
                                    session_id: sid.clone(),
                                    stream: "stdout".to_string(),
                                    content: line.clone(),
                                },
                            );
                            // Persist to database if we have a session store
                            if let Some(ref store) = store_clone {
                                let _ = store.add_terminal_line(&crate::agents::session::SessionId(sid.clone()), "stdout", &line);
                            }
                            output.push_str(&line);
                            output.push('\n');
                        }
                    }
                    output
                })
            };

            let stderr_handle = {
                let app = app_handle.clone();
                let sid = session_id.clone();
                tokio::spawn(async move {
                    let mut output = String::new();
                    if let Some(stderr) = stderr {
                        let mut reader = BufReader::new(stderr).lines();
                        while let Ok(Some(line)) = reader.next_line().await {
                            let _ = app.emit(
                                AGENT_EVENT_NAME,
                                AgentEvent::CommandOutput {
                                    session_id: sid.clone(),
                                    stream: "stderr".to_string(),
                                    content: line.clone(),
                                },
                            );
                            // Persist to database if we have a session store
                            if let Some(ref store) = store {
                                let _ = store.add_terminal_line(&crate::agents::session::SessionId(sid.clone()), "stderr", &line);
                            }
                            output.push_str(&line);
                            output.push('\n');
                        }
                    }
                    output
                })
            };

            // Wait for completion with timeout
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(COMMAND_TIMEOUT_SECS),
                async {
                    let (stdout_result, stderr_result, status) = tokio::join!(
                        stdout_handle,
                        stderr_handle,
                        child.wait()
                    );

                    let stdout_out = stdout_result.unwrap_or_default();
                    let stderr_out = stderr_result.unwrap_or_default();
                    let exit_status = status.map_err(|e| {
                        ToolError::failed(format!("Failed to wait for command: {}", e))
                    })?;

                    Ok::<_, ToolError>((stdout_out, stderr_out, exit_status))
                },
            )
            .await;

            match result {
                Ok(Ok((mut stdout_out, mut stderr_out, exit_status))) => {
                    let total_bytes = stdout_out.len() + stderr_out.len();

                    // Truncate if too large
                    if total_bytes > MAX_OUTPUT_BYTES {
                        let half = MAX_OUTPUT_BYTES / 2;
                        if stdout_out.len() > half {
                            stdout_out.truncate(half);
                            stdout_out.push_str("\n... (truncated)");
                        }
                        if stderr_out.len() > half {
                            stderr_out.truncate(half);
                            stderr_out.push_str("\n... (truncated)");
                        }
                    }

                    let exit_code = exit_status.code();

                    // Emit command finished event
                    let _ = app_handle.emit(
                        AGENT_EVENT_NAME,
                        AgentEvent::CommandFinished {
                            session_id: session_id.clone(),
                            exit_code,
                        },
                    );

                    let success = exit_code == Some(0);

                    Ok(serde_json::json!({
                        "command": args.command,
                        "exit_code": exit_code,
                        "stdout": stdout_out,
                        "stderr": stderr_out,
                        "success": success,
                        "truncated": total_bytes > MAX_OUTPUT_BYTES,
                    }))
                }
                Ok(Err(e)) => Err(e),
                Err(_) => {
                    // Timeout - kill the process
                    let _ = child.kill().await;

                    let _ = app_handle.emit(
                        AGENT_EVENT_NAME,
                        AgentEvent::CommandFinished {
                            session_id,
                            exit_code: None,
                        },
                    );

                    Err(ToolError::failed(format!(
                        "Command timed out after {} seconds",
                        COMMAND_TIMEOUT_SECS
                    )))
                }
            }
        })
    }
}

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;
    use std::fs;

    // === Tool metadata tests ===

    #[test]
    fn test_execute_command_meta() {
        let tool = ExecuteCommandTool;
        let meta = tool.meta();
        assert_eq!(meta.id.full_name(), "system_execute_command");
        assert!(meta.requires_workspace);
    }

    #[test]
    fn test_tool_id_format() {
        let tool = ExecuteCommandTool;
        let meta = tool.meta();

        assert_eq!(meta.id.category, "system");
        assert_eq!(meta.id.name, "execute_command");
    }

    #[test]
    fn test_input_schema() {
        let tool = ExecuteCommandTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");
        assert!(schema["additionalProperties"] == false);

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("command")));
    }

    // === Arguments parsing tests ===

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "command": "echo hello",
            "working_dir": "src"
        });

        let parsed: ExecuteCommandArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.command, "echo hello");
        assert_eq!(parsed.working_dir, Some("src".to_string()));
    }

    #[test]
    fn test_parse_args_without_working_dir() {
        let args = serde_json::json!({
            "command": "ls -la"
        });

        let parsed: ExecuteCommandArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert!(parsed.working_dir.is_none());
    }

    #[test]
    fn test_parse_args_missing_command() {
        let args = serde_json::json!({
            "working_dir": "src"
        });

        let parsed: Result<ExecuteCommandArgs, _> = serde_json::from_value(args);
        assert!(parsed.is_err());
    }

    // === Integration tests with temporary directory ===

    #[tokio::test]
    async fn test_execute_simple_command() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo test output"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["command"], "echo test output");
        assert_eq!(output["success"], true);
        assert_eq!(output["exit_code"], 0);
        assert!(output["stdout"].as_str().unwrap().contains("test output"));
    }

    #[tokio::test]
    async fn test_execute_command_with_nonzero_exit_code() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "exit 42"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["success"], false);
        assert_eq!(output["exit_code"], 42);
    }

    #[tokio::test]
    async fn test_execute_command_with_working_dir() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("test.txt"), b"content").unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "cat test.txt",
            "working_dir": "subdir"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["stdout"].as_str().unwrap().contains("content"));
    }

    #[tokio::test]
    async fn test_execute_command_empty_command_fails() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "   "
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn test_execute_command_no_workspace_fails() {
        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo test"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("workspace"));
    }

    #[tokio::test]
    async fn test_execute_command_working_dir_outside_workspace_fails() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo test",
            "working_dir": "../../../etc"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("outside workspace"));
    }

    #[tokio::test]
    async fn test_execute_command_nonexistent_working_dir_fails() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo test",
            "working_dir": "nonexistent_dir"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_execute_command_stderr_captured() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "sh -c 'echo error >&2'"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["stderr"].as_str().unwrap().contains("error"));
    }

    #[tokio::test]
    async fn test_execute_command_with_pipe() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo hello world | grep hello"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["stdout"].as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_execute_command_list_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), b"content1").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), b"content2").unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "ls"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        let stdout = output["stdout"].as_str().unwrap();
        assert!(stdout.contains("file1.txt") || stdout.contains("file2.txt"));
    }

    #[tokio::test]
    async fn test_execute_command_multiline_output() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo line1 && echo line2 && echo line3"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        let stdout = output["stdout"].as_str().unwrap();
        assert!(stdout.contains("line1"));
        assert!(stdout.contains("line2"));
        assert!(stdout.contains("line3"));
    }

    #[tokio::test]
    async fn test_execute_command_environment_variable() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo $HOME"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        // HOME should be set
        let stdout = output["stdout"].as_str().unwrap();
        assert!(!stdout.trim().is_empty());
    }

    // === Constants tests ===

    #[test]
    fn test_max_output_bytes() {
        assert_eq!(MAX_OUTPUT_BYTES, 512 * 1024);
    }

    #[test]
    fn test_command_timeout_secs() {
        assert_eq!(COMMAND_TIMEOUT_SECS, 60);
    }

    // === Output structure tests ===

    #[tokio::test]
    async fn test_execute_command_output_structure() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo test"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();

        // Verify all expected fields
        assert!(output.get("command").is_some());
        assert!(output.get("exit_code").is_some());
        assert!(output.get("stdout").is_some());
        assert!(output.get("stderr").is_some());
        assert!(output.get("success").is_some());
        assert!(output.get("truncated").is_some());

        // Verify types
        assert!(output["command"].is_string());
        assert!(output["exit_code"].is_number());
        assert!(output["stdout"].is_string());
        assert!(output["stderr"].is_string());
        assert!(output["success"].is_boolean());
        assert!(output["truncated"].is_boolean());
    }

    #[tokio::test]
    async fn test_execute_command_output_is_json_serializable() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo test"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();

        // Verify we can serialize back to JSON string
        let json_string = serde_json::to_string(&output);
        assert!(json_string.is_ok());
    }

    // === Context tests ===

    #[tokio::test]
    async fn test_execute_with_session_context() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation")
            .with_session("test-session");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo test"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_thread_context() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ExecuteCommandTool;
        let context = ToolExecutionContext::new("test-conversation")
            .with_thread("test-thread");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "command": "echo test"
        });

        let result = tool
            .execute(context, args, app_handle, Some(temp_dir.path().to_path_buf()))
            .await;

        assert!(result.is_ok());
    }
}
