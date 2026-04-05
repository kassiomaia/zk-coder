//! Execute shell commands within the workspace.
//!
//! Runs commands in the workspace root directory, streaming stdout/stderr
//! via Tauri events. The execution path is backed by the shared spawn service,
//! which enforces policy, cancellation, and telemetry controls.

use std::path::PathBuf;

use log::info;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use tauri::AppHandle;
use tauri::Manager;

use crate::infrastructure::agent::runtime::{
    SpawnEngine, SpawnError, SpawnInvocation, SpawnOutcome, SpawnRequest,
};
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};

/// Maximum command output size (512 KB) to prevent memory issues.
#[cfg(test)]
const MAX_OUTPUT_BYTES: usize =
    crate::infrastructure::agent::runtime::spawn::DEFAULT_MAX_OUTPUT_BYTES;

/// Maximum command execution time (60 seconds).
#[cfg(test)]
const COMMAND_TIMEOUT_SECS: u64 =
    crate::infrastructure::agent::runtime::spawn::DEFAULT_COMMAND_TIMEOUT_SECS;

/// Arguments for command execution.
#[derive(Debug, Deserialize)]
pub struct ExecuteCommandArgs {
    /// The command to execute (e.g., "cargo build").
    pub command: String,
    /// Optional executable to launch directly without a shell.
    pub program: Option<String>,
    /// Optional arguments for the direct executable launch.
    #[serde(default)]
    pub args: Vec<String>,
    /// Optional working directory relative to workspace root.
    pub working_dir: Option<String>,
}

/// Tool to execute shell commands within the workspace.
pub struct ExecuteCommandTool;

impl Tool for ExecuteCommandTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("system", "execute_command"),
            description: "Execute a command in the workspace directory. Prefer program + args for argv-first launches; use command for shell features. Returns stdout, stderr, and exit code. Output is streamed in real-time.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute when shell features are needed (e.g., 'cargo build', 'npm test')"
                    },
                    "program": {
                        "type": "string",
                        "description": "Executable to run directly without a shell"
                    },
                    "args": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "Arguments for the executable when program is provided"
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

            let args: ExecuteCommandArgs = crate::infrastructure::tools::error::parse_args(
                "system_execute_command",
                arguments,
            )?;

            if args.command.trim().is_empty()
                && args.program.as_deref().unwrap_or("").trim().is_empty()
            {
                return Err(ToolError::invalid_args("Command cannot be empty"));
            }

            // Resolve working directory
            let cwd = if let Some(ref dir) = args.working_dir {
                let resolved = workspace.join(dir);
                if !resolved.starts_with(&workspace) {
                    return Err(ToolError::PathTraversal(format!(
                        "Working directory '{dir}' is outside workspace"
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

            let session_id = context
                .session_id
                .clone()
                .unwrap_or_else(|| context.conversation_id.clone());

            info!(
                "[ExecuteCommand] Running '{}' in '{}'",
                args.program.as_deref().unwrap_or(&args.command),
                cwd.display()
            );

            let request = SpawnRequest {
                session_id: session_id.clone(),
                invocation: if let Some(program) = args
                    .program
                    .as_deref()
                    .map(str::trim)
                    .filter(|program| !program.is_empty())
                {
                    SpawnInvocation::Program {
                        program: program.to_string(),
                        args: args.args.clone(),
                    }
                } else {
                    SpawnInvocation::Shell {
                        command: args.command.trim().to_string(),
                    }
                },
                working_dir: args.working_dir.as_ref().map(PathBuf::from),
                workspace_root: Some(workspace.clone()),
            };

            let outcome = match app_handle.try_state::<crate::commands::AgentState>() {
                Some(state) => state
                    .service
                    .spawn_command(request, &app_handle)
                    .await
                    .map_err(map_spawn_error)?,
                None => SpawnEngine::shared()
                    .execute(request, &app_handle, None)
                    .await
                    .map_err(map_spawn_error)?,
            };

            Ok(spawn_outcome_to_value(&args, outcome))
        })
    }
}

fn map_spawn_error(err: SpawnError) -> ToolError {
    match err {
        SpawnError::InvalidArguments(message) => ToolError::invalid_args(message),
        SpawnError::WorkspaceRequired(message) => ToolError::WorkspaceRequired(message),
        SpawnError::PathTraversal(message) => ToolError::PathTraversal(message),
        SpawnError::WorkingDirectoryMissing(message) => ToolError::failed(message),
        SpawnError::Denied(message) => ToolError::failed(message),
        SpawnError::Busy(message) => ToolError::failed(message),
        SpawnError::SpawnFailed(message) => ToolError::failed(message),
        SpawnError::ExecutionFailed(message) => ToolError::failed(message),
        SpawnError::TimedOut(seconds) => {
            ToolError::failed(format!("Command timed out after {seconds} seconds"))
        }
        SpawnError::Cancelled => ToolError::failed("Command cancelled"),
        SpawnError::Io(err) => ToolError::Io(err),
    }
}

fn spawn_outcome_to_value(args: &ExecuteCommandArgs, outcome: SpawnOutcome) -> JsonValue {
    serde_json::json!({
        "command": args.command.clone(),
        "program": args.program.clone(),
        "args": args.args.clone(),
        "working_dir": args.working_dir.clone(),
        "spawn_id": outcome.spawn_id,
        "mode": outcome.mode,
        "pid": outcome.pid,
        "exit_code": outcome.exit_code,
        "stdout": outcome.stdout,
        "stderr": outcome.stderr,
        "success": outcome.success,
        "truncated": outcome.truncated,
        "duration_ms": outcome.duration_ms,
        "resolved_working_dir": outcome.working_dir,
    })
}

#[cfg(all(test, feature = "integration-test"))]
#[path = "execute_command_tests.rs"]
mod execute_command_tests;
