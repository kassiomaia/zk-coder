//! Execute shell commands within the workspace.
//!
//! Runs commands in the workspace root directory, streaming stdout/stderr
//! via Tauri events. Requires approval via the agent permission system.

use std::path::PathBuf;

use log::info;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_command_meta() {
        let tool = ExecuteCommandTool;
        let meta = tool.meta();
        assert_eq!(meta.id.full_name(), "system_execute_command");
        assert!(meta.requires_workspace);
    }
}
