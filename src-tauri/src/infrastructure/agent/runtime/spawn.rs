//! Shared spawn engine for agent-driven process execution.
//!
//! This module owns process creation, concurrency limits, output streaming,
//! cancellation checks, and termination behavior. Higher layers can attach
//! telemetry and runtime traces through the `SpawnObserver` trait.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use log::info;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex, Semaphore};

use crate::core::agent::execution::RuntimeTraceType;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};

/// Maximum command output size, in bytes.
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 512 * 1024;

/// Default command execution timeout, in seconds.
pub const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 60;

/// How often the engine polls for process cancellation, in milliseconds.
pub const DEFAULT_CANCELLATION_POLL_MS: u64 = 250;

/// Result of a spawn invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpawnOutcome {
    /// Spawn identifier used for trace correlation.
    pub spawn_id: String,
    /// Launch mode used for the process.
    pub mode: SpawnMode,
    /// Executable used for direct launches.
    pub program: Option<String>,
    /// Arguments used for direct launches.
    pub args: Vec<String>,
    /// Shell command used for shell launches.
    pub command: Option<String>,
    /// Process id, if available.
    pub pid: Option<u32>,
    /// Exit code, if the process terminated normally.
    pub exit_code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Whether the process exited successfully.
    pub success: bool,
    /// Whether output was truncated.
    pub truncated: bool,
    /// Total execution duration in milliseconds.
    pub duration_ms: u64,
    /// Working directory used for the spawn.
    pub working_dir: String,
}

/// Launch mode for a process spawn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpawnMode {
    /// Launch a program directly with argv.
    Program,
    /// Launch through the shell.
    Shell,
}

/// How the process should be launched.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SpawnInvocation {
    /// Launch a program directly with arguments.
    Program {
        /// Executable to launch directly.
        program: String,
        /// Arguments passed to the executable.
        args: Vec<String>,
    },
    /// Launch a shell command string.
    Shell {
        /// Shell command string to execute.
        command: String,
    },
}

/// Request for a process spawn.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpawnRequest {
    /// Session identifier used for cancellation and telemetry correlation.
    pub session_id: String,
    /// Launch details.
    pub invocation: SpawnInvocation,
    /// Working directory relative to the workspace root.
    pub working_dir: Option<PathBuf>,
    /// Workspace root for safety checks.
    pub workspace_root: Option<PathBuf>,
}

/// Structured policy for process spawning.
#[derive(Debug, Clone)]
pub struct SpawnPolicy {
    /// Whether shell execution is allowed.
    pub allow_shell: bool,
    /// Maximum concurrent spawns across the backend.
    pub max_concurrent_spawns: usize,
    /// Whether a single session may run more than one spawn concurrently.
    pub allow_concurrent_spawns_per_session: bool,
    /// Maximum output size retained per spawn.
    pub max_output_bytes: usize,
    /// Timeout for the whole spawn operation.
    pub command_timeout: Duration,
    /// Poll interval for cancellation checks.
    pub cancellation_poll: Duration,
    /// Optional allowlist of direct executable names.
    pub allowed_programs: Option<HashSet<String>>,
}

impl Default for SpawnPolicy {
    fn default() -> Self {
        Self {
            allow_shell: true,
            max_concurrent_spawns: 4,
            allow_concurrent_spawns_per_session: false,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            command_timeout: Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS),
            cancellation_poll: Duration::from_millis(DEFAULT_CANCELLATION_POLL_MS),
            allowed_programs: None,
        }
    }
}

/// Errors that can occur while spawning a process.
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    /// Invalid arguments were provided.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    /// Workspace access was required but not available.
    #[error("Workspace access required: {0}")]
    WorkspaceRequired(String),
    /// Requested working directory escapes the workspace boundary.
    #[error("Working directory escapes workspace: {0}")]
    PathTraversal(String),
    /// Requested working directory does not exist.
    #[error("Working directory does not exist: {0}")]
    WorkingDirectoryMissing(String),
    /// Spawn was denied by policy.
    #[error("Spawn denied by policy: {0}")]
    Denied(String),
    /// A spawn is already active for the session.
    #[error("A spawn is already active for session {0}")]
    Busy(String),
    /// The process could not be started.
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),
    /// The process execution failed.
    #[error("Process execution failed: {0}")]
    ExecutionFailed(String),
    /// The process timed out.
    #[error("Process timed out after {0} seconds")]
    TimedOut(u64),
    /// The process was cancelled.
    #[error("Process cancelled")]
    Cancelled,
    /// Unexpected I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl SpawnError {
    /// Convert the spawn error to a tool-safe error string.
    #[must_use]
    pub fn to_message(&self) -> String {
        self.to_string()
    }
}

/// Observer interface for telemetry and runtime traces.
pub trait SpawnObserver: Send + Sync {
    /// Record a runtime trace event for spawn lifecycle telemetry.
    fn record_trace(&self, trace_type: RuntimeTraceType, payload: JsonValue);

    /// Record a streamed terminal line.
    fn record_terminal_line(&self, line_type: &str, content: &str);

    /// Returns true if the session should be cancelled.
    fn is_cancelled(&self) -> bool {
        false
    }
}

enum CommandStreamEvent {
    Line {
        stream: &'static str,
        content: String,
    },
    StreamClosed {
        stream: &'static str,
    },
}

struct SpawnLease {
    active_sessions: Arc<Mutex<HashSet<String>>>,
    session_id: String,
}

impl Drop for SpawnLease {
    fn drop(&mut self) {
        let active_sessions = self.active_sessions.clone();
        let session_id = self.session_id.clone();
        let _ = tokio::runtime::Handle::try_current().map(|handle| {
            handle.spawn(async move {
                let mut guard = active_sessions.lock().await;
                guard.remove(&session_id);
            });
        });
    }
}

/// Shared process spawn engine.
pub struct SpawnEngine {
    policy: SpawnPolicy,
    active_sessions: Arc<Mutex<HashSet<String>>>,
    global_permits: Arc<Semaphore>,
}

impl SpawnEngine {
    /// Creates a new engine with the provided policy.
    #[must_use]
    pub fn new(policy: SpawnPolicy) -> Self {
        Self {
            global_permits: Arc::new(Semaphore::new(policy.max_concurrent_spawns)),
            active_sessions: Arc::new(Mutex::new(HashSet::new())),
            policy,
        }
    }

    /// Returns the shared engine used by agent-driven process execution.
    #[must_use]
    pub fn shared() -> Arc<Self> {
        static INSTANCE: OnceLock<Arc<SpawnEngine>> = OnceLock::new();
        INSTANCE
            .get_or_init(|| Arc::new(Self::new(SpawnPolicy::default())))
            .clone()
    }

    /// Executes a spawn request.
    pub async fn execute<R: tauri::Runtime>(
        &self,
        request: SpawnRequest,
        app_handle: &AppHandle<R>,
        observer: Option<&dyn SpawnObserver>,
    ) -> Result<SpawnOutcome, SpawnError> {
        let start = Instant::now();
        let spawn_id = uuid::Uuid::new_v4().to_string();
        let session_id = request.session_id.clone();

        if request.session_id.trim().is_empty() {
            return Err(SpawnError::InvalidArguments(
                "Session id is required for spawn execution".to_string(),
            ));
        }

        let _permit = self
            .global_permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| SpawnError::Busy(session_id.clone()))?;

        let _session_lease = if self.policy.allow_concurrent_spawns_per_session {
            None
        } else {
            let mut guard = self.active_sessions.lock().await;
            if guard.contains(&session_id) {
                if let Some(observer) = observer {
                    observer.record_trace(
                        RuntimeTraceType::SpawnBusy,
                        json!({
                            "spawnId": spawn_id,
                            "sessionId": session_id,
                            "reason": "session_already_running",
                        }),
                    );
                }
                return Err(SpawnError::Busy(session_id));
            }
            guard.insert(session_id.clone());
            Some(SpawnLease {
                active_sessions: self.active_sessions.clone(),
                session_id: session_id.clone(),
            })
        };

        let resolved = match self.resolve_launch(request) {
            Ok(resolved) => resolved,
            Err(err) => {
                if let Some(observer) = observer {
                    let trace_type = match err {
                        SpawnError::Denied(_) | SpawnError::PathTraversal(_) => {
                            RuntimeTraceType::SpawnDenied
                        }
                        _ => RuntimeTraceType::SpawnFailed,
                    };
                    observer.record_trace(
                        trace_type,
                        json!({
                            "spawnId": spawn_id,
                            "sessionId": session_id,
                            "error": err.to_message(),
                        }),
                    );
                }
                return Err(err);
            }
        };
        let cwd = match self
            .resolve_working_dir(&resolved.workspace_root, resolved.working_dir.as_deref())
        {
            Ok(cwd) => cwd,
            Err(err) => {
                if let Some(observer) = observer {
                    let trace_type = match err {
                        SpawnError::Denied(_) | SpawnError::PathTraversal(_) => {
                            RuntimeTraceType::SpawnDenied
                        }
                        _ => RuntimeTraceType::SpawnFailed,
                    };
                    observer.record_trace(
                        trace_type,
                        json!({
                            "spawnId": spawn_id,
                            "sessionId": session_id,
                            "error": err.to_message(),
                        }),
                    );
                }
                return Err(err);
            }
        };

        if let Some(observer) = observer {
            observer.record_trace(
                RuntimeTraceType::SpawnStarted,
                json!({
                    "spawnId": spawn_id,
                    "sessionId": session_id,
                    "mode": resolved.mode,
                    "program": resolved.program,
                    "argsCount": resolved.args.len(),
                    "commandLength": resolved.command.as_ref().map(|command| command.len()),
                    "workingDir": cwd.to_string_lossy(),
                    "workspaceRoot": resolved.workspace_root.as_ref().map(|path| path.to_string_lossy().to_string()),
                    "maxOutputBytes": self.policy.max_output_bytes,
                    "timeoutSeconds": self.policy.command_timeout.as_secs(),
                    "allowShell": self.policy.allow_shell,
                }),
            );
        }

        info!(
            "[SpawnEngine] Starting {:?} spawn for session {} in {}",
            resolved.mode,
            session_id,
            cwd.display()
        );

        let mut command = self.build_command(&resolved, &cwd)?;

        let pid;
        let mut child = command
            .spawn()
            .map_err(|e| SpawnError::SpawnFailed(e.to_string()))?;
        pid = child.id();

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<CommandStreamEvent>();

        let stdout_handle = {
            let tx = stream_tx.clone();
            tokio::spawn(async move {
                if let Some(stdout) = stdout {
                    let mut reader = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        if tx
                            .send(CommandStreamEvent::Line {
                                stream: "stdout",
                                content: line,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                let _ = tx.send(CommandStreamEvent::StreamClosed { stream: "stdout" });
            })
        };

        let stderr_handle = {
            let tx = stream_tx.clone();
            tokio::spawn(async move {
                if let Some(stderr) = stderr {
                    let mut reader = BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        if tx
                            .send(CommandStreamEvent::Line {
                                stream: "stderr",
                                content: line,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                let _ = tx.send(CommandStreamEvent::StreamClosed { stream: "stderr" });
            })
        };
        drop(stream_tx);

        let result = tokio::time::timeout(self.policy.command_timeout, async {
            let mut stdout_out = String::new();
            let mut stderr_out = String::new();
            let mut stdout_closed = false;
            let mut stderr_closed = false;
            let mut exit_status = None;
            let mut last_progress = Instant::now();

            loop {
                if exit_status.is_some() && stdout_closed && stderr_closed {
                    break;
                }

                tokio::select! {
                    maybe_event = stream_rx.recv(), if !(stdout_closed && stderr_closed) => {
                        match maybe_event {
                            Some(CommandStreamEvent::Line { stream, content }) => {
                                let _ = app_handle.emit(
                                    AGENT_EVENT_NAME,
                                    AgentEvent::CommandOutput {
                                        session_id: session_id.clone(),
                                        stream: stream.to_string(),
                                        content: content.clone(),
                                    },
                                );

                                if let Some(observer) = observer {
                                    observer.record_terminal_line(stream, &content);
                                }

                                let target = if stream == "stdout" {
                                    &mut stdout_out
                                } else {
                                    &mut stderr_out
                                };
                                target.push_str(&content);
                                target.push('\n');
                                last_progress = Instant::now();
                            }
                            Some(CommandStreamEvent::StreamClosed { stream }) => {
                                if stream == "stdout" {
                                    stdout_closed = true;
                                } else {
                                    stderr_closed = true;
                                }
                                last_progress = Instant::now();
                            }
                            None => {
                                stdout_closed = true;
                                stderr_closed = true;
                            }
                        }
                    }
                    () = tokio::time::sleep(self.policy.cancellation_poll) => {
                        if exit_status.is_none() {
                            exit_status = child.try_wait().map_err(|e| {
                                SpawnError::ExecutionFailed(format!("Failed to wait for process: {e}"))
                            })?;
                        }

                        if let Some(observer) = observer {
                            if observer.is_cancelled() {
                                let _ = child.kill().await;
                                let _ = child.wait().await;
                                observer.record_trace(
                                    RuntimeTraceType::SpawnCancelled,
                                    json!({
                                        "spawnId": spawn_id,
                                        "sessionId": session_id,
                                        "pid": pid,
                                        "elapsedMs": start.elapsed().as_millis() as u64,
                                    }),
                                );
                                observer.record_trace(
                                    RuntimeTraceType::SpawnKilled,
                                    json!({
                                        "spawnId": spawn_id,
                                        "sessionId": session_id,
                                        "pid": pid,
                                        "reason": "session_cancelled",
                                    }),
                                );

                                let _ = app_handle.emit(
                                    AGENT_EVENT_NAME,
                                    AgentEvent::CommandFinished {
                                        session_id: session_id.clone(),
                                        exit_code: None,
                                    },
                                );

                                return Err(SpawnError::Cancelled);
                            }
                        }

                        if exit_status.is_none() && last_progress.elapsed() > self.policy.command_timeout {
                            let _ = child.kill().await;
                            let _ = child.wait().await;
                            if let Some(observer) = observer {
                                observer.record_trace(
                                    RuntimeTraceType::SpawnTimedOut,
                                    json!({
                                        "spawnId": spawn_id,
                                        "sessionId": session_id,
                                        "pid": pid,
                                        "elapsedMs": start.elapsed().as_millis() as u64,
                                    }),
                                );
                                observer.record_trace(
                                    RuntimeTraceType::SpawnKilled,
                                    json!({
                                        "spawnId": spawn_id,
                                        "sessionId": session_id,
                                        "pid": pid,
                                        "reason": "timeout",
                                    }),
                                );
                            }
                            let _ = app_handle.emit(
                                AGENT_EVENT_NAME,
                                AgentEvent::CommandFinished {
                                    session_id: session_id.clone(),
                                    exit_code: None,
                                },
                            );
                            return Err(SpawnError::TimedOut(self.policy.command_timeout.as_secs()));
                        }
                    }
                }
            }

            let _ = stdout_handle.await;
            let _ = stderr_handle.await;

            let exit_status = exit_status.ok_or_else(|| {
                SpawnError::ExecutionFailed(
                    "Process finished without a final exit status".to_string(),
                )
            })?;

            Ok::<_, SpawnError>((stdout_out, stderr_out, exit_status))
        })
        .await;

        match result {
            Ok(Ok((mut stdout_out, mut stderr_out, exit_status))) => {
                let total_bytes = stdout_out.len() + stderr_out.len();
                let truncated = total_bytes > self.policy.max_output_bytes;

                if truncated {
                    let half = self.policy.max_output_bytes / 2;
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
                let success = exit_code == Some(0);

                if let Some(observer) = observer {
                    observer.record_trace(
                        RuntimeTraceType::SpawnFinished,
                        json!({
                            "spawnId": spawn_id,
                            "sessionId": session_id,
                            "pid": pid,
                            "exitCode": exit_code,
                            "success": success,
                            "durationMs": start.elapsed().as_millis() as u64,
                            "stdoutBytes": stdout_out.len(),
                            "stderrBytes": stderr_out.len(),
                            "truncated": truncated,
                        }),
                    );
                }

                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::CommandFinished {
                        session_id: session_id.clone(),
                        exit_code,
                    },
                );

                Ok(SpawnOutcome {
                    spawn_id,
                    mode: resolved.mode,
                    program: resolved.program,
                    args: resolved.args,
                    command: resolved.command,
                    pid,
                    exit_code,
                    stdout: stdout_out,
                    stderr: stderr_out,
                    success,
                    truncated,
                    duration_ms: start.elapsed().as_millis() as u64,
                    working_dir: cwd.to_string_lossy().to_string(),
                })
            }
            Ok(Err(err)) => {
                if let Some(observer) = observer {
                    observer.record_trace(
                        RuntimeTraceType::SpawnFailed,
                        json!({
                            "spawnId": spawn_id,
                            "sessionId": session_id,
                            "error": err.to_message(),
                        }),
                    );
                }
                // Return outcome with success=false instead of error
                Ok(SpawnOutcome {
                    spawn_id,
                    mode: resolved.mode,
                    program: resolved.program,
                    args: resolved.args,
                    command: resolved.command,
                    pid,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: err.to_message(),
                    success: false,
                    truncated: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    working_dir: cwd.to_string_lossy().to_string(),
                })
            }
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                if let Some(observer) = observer {
                    observer.record_trace(
                        RuntimeTraceType::SpawnTimedOut,
                        json!({
                            "spawnId": spawn_id,
                            "sessionId": session_id,
                            "pid": pid,
                            "elapsedMs": start.elapsed().as_millis() as u64,
                            "reason": "overall_timeout",
                        }),
                    );
                }
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::CommandFinished {
                        session_id,
                        exit_code: None,
                    },
                );
                // Return outcome with success=false instead of error
                Ok(SpawnOutcome {
                    spawn_id,
                    mode: resolved.mode,
                    program: resolved.program,
                    args: resolved.args,
                    command: resolved.command,
                    pid,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: format!(
                        "Command timed out after {} seconds",
                        self.policy.command_timeout.as_secs()
                    ),
                    success: false,
                    truncated: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    working_dir: cwd.to_string_lossy().to_string(),
                })
            }
        }
    }

    fn resolve_launch(&self, request: SpawnRequest) -> Result<ResolvedLaunch, SpawnError> {
        let SpawnRequest {
            session_id: _,
            invocation,
            working_dir,
            workspace_root,
        } = request;

        match invocation {
            SpawnInvocation::Program { program, args } => {
                let program = program.trim().to_string();
                if program.is_empty() {
                    return Err(SpawnError::InvalidArguments(
                        "Program cannot be empty".to_string(),
                    ));
                }

                if let Some(allowed) = &self.policy.allowed_programs {
                    if !allowed.contains(&program) {
                        return Err(SpawnError::Denied(format!(
                            "Program '{program}' is not allowed by policy"
                        )));
                    }
                }

                Ok(ResolvedLaunch {
                    mode: SpawnMode::Program,
                    program: Some(program),
                    args,
                    command: None,
                    working_dir,
                    workspace_root,
                })
            }
            SpawnInvocation::Shell { command } => {
                let command = command.trim().to_string();
                if command.is_empty() {
                    return Err(SpawnError::InvalidArguments(
                        "Command cannot be empty".to_string(),
                    ));
                }

                if !self.policy.allow_shell {
                    return Err(SpawnError::Denied(
                        "Shell execution is disabled by policy".to_string(),
                    ));
                }

                Ok(ResolvedLaunch {
                    mode: SpawnMode::Shell,
                    program: None,
                    args: Vec::new(),
                    command: Some(command),
                    working_dir,
                    workspace_root,
                })
            }
        }
    }

    fn resolve_working_dir(
        &self,
        workspace_root: &Option<PathBuf>,
        working_dir: Option<&Path>,
    ) -> Result<PathBuf, SpawnError> {
        let workspace = workspace_root
            .as_ref()
            .ok_or_else(|| SpawnError::WorkspaceRequired("No workspace configured".to_string()))?;

        let cwd = match working_dir {
            Some(dir) => {
                // Reject parent-directory components before joining. `Path::starts_with`
                // is lexical, so `/workspace/../etc` would still start with `/workspace`
                // and pass the prefix check even though it resolves outside the workspace.
                if dir
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    return Err(SpawnError::PathTraversal(format!(
                        "Working directory '{}' contains parent-directory components",
                        dir.display()
                    )));
                }
                let resolved = workspace.join(dir);
                if !resolved.starts_with(workspace) {
                    return Err(SpawnError::PathTraversal(format!(
                        "Working directory '{}' is outside workspace",
                        dir.display()
                    )));
                }
                resolved
            }
            None => workspace.clone(),
        };

        if !cwd.exists() {
            return Err(SpawnError::WorkingDirectoryMissing(
                cwd.display().to_string(),
            ));
        }

        Ok(cwd)
    }

    fn build_command(&self, launch: &ResolvedLaunch, cwd: &Path) -> Result<Command, SpawnError> {
        let mut command = match launch.mode {
            SpawnMode::Program => {
                let program = launch
                    .program
                    .as_ref()
                    .ok_or_else(|| SpawnError::ExecutionFailed("Missing program".to_string()))?;
                let mut command = Command::new(program);
                command.args(&launch.args);
                command
            }
            SpawnMode::Shell => {
                let shell_command = launch.command.as_ref().ok_or_else(|| {
                    SpawnError::ExecutionFailed("Missing shell command".to_string())
                })?;
                let mut command = Command::new("sh");
                command.arg("-c").arg(shell_command);
                command
            }
        };

        command
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        Ok(command)
    }
}

#[derive(Debug)]
struct ResolvedLaunch {
    mode: SpawnMode,
    program: Option<String>,
    args: Vec<String>,
    command: Option<String>,
    working_dir: Option<PathBuf>,
    workspace_root: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };
    use tempfile::TempDir;

    #[derive(Default)]
    struct RecordingObserver {
        traces: Mutex<Vec<RuntimeTraceType>>,
        lines: Mutex<Vec<(String, String)>>,
        cancelled: AtomicBool,
    }

    impl RecordingObserver {
        fn new(cancelled: bool) -> Self {
            Self {
                traces: Mutex::new(Vec::new()),
                lines: Mutex::new(Vec::new()),
                cancelled: AtomicBool::new(cancelled),
            }
        }

        fn trace_types(&self) -> Vec<RuntimeTraceType> {
            self.traces
                .lock()
                .expect("traces lock")
                .iter()
                .cloned()
                .collect()
        }

        fn terminal_lines(&self) -> Vec<(String, String)> {
            self.lines.lock().expect("lines lock").clone()
        }
    }

    impl SpawnObserver for RecordingObserver {
        fn record_trace(&self, trace_type: RuntimeTraceType, _payload: JsonValue) {
            self.traces.lock().expect("traces lock").push(trace_type);
        }

        fn record_terminal_line(&self, line_type: &str, content: &str) {
            self.lines
                .lock()
                .expect("lines lock")
                .push((line_type.to_string(), content.to_string()));
        }

        fn is_cancelled(&self) -> bool {
            self.cancelled.load(Ordering::Relaxed)
        }
    }

    #[test]
    fn test_default_policy_values() {
        let policy = SpawnPolicy::default();
        assert!(policy.allow_shell);
        assert_eq!(policy.max_concurrent_spawns, 4);
        assert!(!policy.allow_concurrent_spawns_per_session);
        assert_eq!(policy.max_output_bytes, DEFAULT_MAX_OUTPUT_BYTES);
    }

    #[test]
    fn test_request_serialization_round_trip() {
        let request = SpawnRequest {
            session_id: "session-1".to_string(),
            invocation: SpawnInvocation::Program {
                program: "cargo".to_string(),
                args: vec!["test".to_string()],
            },
            working_dir: Some(PathBuf::from("src-tauri")),
            workspace_root: Some(PathBuf::from("/workspace")),
        };

        let json = serde_json::to_value(&request).expect("serialize request");
        let deserialized: SpawnRequest = serde_json::from_value(json).expect("deserialize request");
        assert_eq!(deserialized.session_id, "session-1");
        assert!(matches!(
            deserialized.invocation,
            SpawnInvocation::Program { .. }
        ));
    }

    #[test]
    fn test_outcome_serialization_round_trip() {
        let outcome = SpawnOutcome {
            spawn_id: "spawn-1".to_string(),
            mode: SpawnMode::Program,
            program: Some("cargo".to_string()),
            args: vec!["test".to_string()],
            command: None,
            pid: Some(1234),
            exit_code: Some(0),
            stdout: "ok".to_string(),
            stderr: String::new(),
            success: true,
            truncated: false,
            duration_ms: 12,
            working_dir: "/workspace".to_string(),
        };

        let json = serde_json::to_value(&outcome).expect("serialize outcome");
        let deserialized: SpawnOutcome = serde_json::from_value(json).expect("deserialize outcome");
        assert_eq!(deserialized.spawn_id, "spawn-1");
        assert!(deserialized.success);
    }

    #[test]
    fn test_resolve_launch_program_respects_allowlist() {
        let mut allowed_programs = HashSet::new();
        allowed_programs.insert("cargo".to_string());

        let engine = SpawnEngine::new(SpawnPolicy {
            allowed_programs: Some(allowed_programs),
            ..SpawnPolicy::default()
        });

        let request = SpawnRequest {
            session_id: "session-1".to_string(),
            invocation: SpawnInvocation::Program {
                program: "git".to_string(),
                args: vec!["status".to_string()],
            },
            working_dir: None,
            workspace_root: Some(PathBuf::from("/workspace")),
        };

        let err = engine
            .resolve_launch(request)
            .expect_err("policy should deny");
        assert!(matches!(err, SpawnError::Denied(_)));
    }

    #[test]
    fn test_resolve_launch_shell_respects_policy() {
        let engine = SpawnEngine::new(SpawnPolicy {
            allow_shell: false,
            ..SpawnPolicy::default()
        });

        let request = SpawnRequest {
            session_id: "session-1".to_string(),
            invocation: SpawnInvocation::Shell {
                command: "echo hi".to_string(),
            },
            working_dir: None,
            workspace_root: Some(PathBuf::from("/workspace")),
        };

        let err = engine
            .resolve_launch(request)
            .expect_err("policy should deny");
        assert!(matches!(err, SpawnError::Denied(_)));
    }

    #[test]
    fn test_resolve_working_dir_rejects_path_traversal() {
        let engine = SpawnEngine::new(SpawnPolicy::default());
        let workspace = PathBuf::from("/workspace");

        let err = engine
            .resolve_working_dir(&Some(workspace), Some(Path::new("../etc")))
            .expect_err("path traversal should fail");
        assert!(matches!(err, SpawnError::PathTraversal(_)));
    }

    #[tokio::test]
    async fn test_execute_program_spawn_records_trace_and_output() {
        let temp_dir = TempDir::new().expect("temp dir");
        let app_handle = crate::test_support::mock_app_handle();
        let engine = SpawnEngine::new(SpawnPolicy {
            command_timeout: Duration::from_secs(5),
            cancellation_poll: Duration::from_millis(5),
            ..SpawnPolicy::default()
        });
        let observer = RecordingObserver::new(false);

        let outcome = engine
            .execute(
                SpawnRequest {
                    session_id: "session-1".to_string(),
                    invocation: SpawnInvocation::Program {
                        program: "echo".to_string(),
                        args: vec!["hello".to_string()],
                    },
                    working_dir: None,
                    workspace_root: Some(temp_dir.path().to_path_buf()),
                },
                &app_handle,
                Some(&observer),
            )
            .await
            .expect("spawn should succeed");

        assert_eq!(outcome.mode, SpawnMode::Program);
        assert!(outcome.success);
        assert!(outcome.stdout.contains("hello"));
        assert!(observer
            .trace_types()
            .contains(&RuntimeTraceType::SpawnStarted));
        assert!(observer
            .trace_types()
            .contains(&RuntimeTraceType::SpawnFinished));

        let terminal_lines = observer.terminal_lines();
        assert!(terminal_lines
            .iter()
            .any(|(line_type, content)| line_type == "stdout" && content.contains("hello")));
    }

    #[tokio::test]
    async fn test_execute_cancelled_spawn_records_kill_trace() {
        let temp_dir = TempDir::new().expect("temp dir");
        let app_handle = crate::test_support::mock_app_handle();
        let engine = SpawnEngine::new(SpawnPolicy {
            command_timeout: Duration::from_secs(5),
            cancellation_poll: Duration::from_millis(5),
            ..SpawnPolicy::default()
        });
        let observer = RecordingObserver::new(true);

        let outcome = engine
            .execute(
                SpawnRequest {
                    session_id: "session-cancel".to_string(),
                    invocation: SpawnInvocation::Shell {
                        command: "sleep 5".to_string(),
                    },
                    working_dir: None,
                    workspace_root: Some(temp_dir.path().to_path_buf()),
                },
                &app_handle,
                Some(&observer),
            )
            .await
            .expect("spawn should return outcome");

        // Cancellation returns outcome with success=false instead of error
        assert!(!outcome.success);
        let trace_types = observer.trace_types();
        assert!(trace_types.contains(&RuntimeTraceType::SpawnCancelled));
        assert!(trace_types.contains(&RuntimeTraceType::SpawnKilled));
    }
}
