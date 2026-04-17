//! Application-facing read models for the agent backend.
//!
//! These structs aggregate runtime data needed by the Tauri command layer so
//! IPC handlers can depend on a stable application surface instead of reaching
//! directly into runtime and repository internals.

use serde::Serialize;

use crate::core::agent::execution::{ExecutionSnapshot, RuntimeTrace};
use crate::core::agent::session::{AgentSession, PendingAction};
use crate::core::agent::session::SessionState;
use crate::core::agent::task::{AgentTask, SubTask, TaskStatus};
use crate::core::agent::types::Agent;

/// Long-running harness role for a session.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HarnessMode {
    /// Session is bootstrapping and preparing an execution context.
    Initializer,
    /// Session is performing normal work within an active or completed run.
    Worker,
    /// Session is orienting itself for a resumed run after interruption or pause.
    ResumeVerifier,
}

/// Environment health summary for session handoff.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentHealth {
    /// No health assessment has been recorded yet.
    Unknown,
    /// Environment is healthy for continued work.
    Healthy,
    /// Environment is usable but needs attention.
    Degraded,
    /// Environment is blocked and should not resume unchanged.
    Blocked,
}

/// Startup verification lifecycle summary.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StartupVerificationState {
    /// No verification has run yet.
    Idle,
    /// Verification is currently in progress.
    Running,
    /// Verification completed successfully.
    Passed,
    /// Verification completed with a blocking or notable failure.
    Failed,
}

/// Session-facing summary of startup verification.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupVerificationSummary {
    /// Current verification state.
    pub state: StartupVerificationState,
    /// Short user-facing headline.
    pub headline: String,
    /// Optional detail lines explaining the result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<String>>,
    /// Timestamp of the latest completed check.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<String>,
}

/// Session-facing handoff summary for long-running work.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionHandoffSummary {
    /// Most recent completed unit of work.
    pub last_completed_increment: String,
    /// Current durable objective.
    pub current_objective: String,
    /// Recommended next action when resuming.
    pub next_recommended_action: String,
    /// Open issues or blockers.
    pub open_issues: Vec<String>,
    /// Health of the environment at handoff time.
    pub environment_health: EnvironmentHealth,
    /// Timestamp when the handoff was recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<String>,
}

/// Aggregate long-running harness state exposed to the frontend.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionHarnessState {
    /// Current harness role for the active session window.
    pub mode: HarnessMode,
    /// Summary of startup verification if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_verification: Option<StartupVerificationSummary>,
    /// Summary of the latest durable handoff if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff: Option<SessionHandoffSummary>,
    /// Whether a clean exit summary has been recorded.
    pub clean_exit_recorded: bool,
}

fn most_relevant_subtask(tasks: &[AgentTask]) -> Option<&SubTask> {
    tasks.iter()
        .flat_map(|task| task.subtasks.iter().flat_map(|subtasks| subtasks.iter()))
        .find(|subtask| subtask.status == TaskStatus::InProgress)
        .or_else(|| {
            tasks.iter()
                .flat_map(|task| task.subtasks.iter().flat_map(|subtasks| subtasks.iter()))
                .filter(|subtask| subtask.status == TaskStatus::Completed)
                .last()
        })
}

fn latest_completed_title(tasks: &[AgentTask]) -> Option<String> {
    tasks.iter()
        .flat_map(|task| {
            task.subtasks
                .iter()
                .flat_map(|subtasks| subtasks.iter())
                .filter(|subtask| subtask.status == TaskStatus::Completed)
                .map(|subtask| subtask.title.clone())
        })
        .last()
        .or_else(|| {
            tasks.iter()
                .rev()
                .find(|task| task.status == TaskStatus::Completed)
                .and_then(|task| task.title.clone())
        })
}

/// Derive additive long-running harness state from the current session and task snapshot.
#[must_use]
pub fn derive_harness_state(
    session: &AgentSession,
    tasks: &[AgentTask],
) -> AgentSessionHarnessState {
    let mode = match session.state {
        SessionState::Launching => HarnessMode::Initializer,
        SessionState::Paused { .. } => HarnessMode::ResumeVerifier,
        _ => HarnessMode::Worker,
    };

    let startup_verification = match &session.state {
        SessionState::Launching => Some(StartupVerificationSummary {
            state: StartupVerificationState::Running,
            headline: "Preparing session".to_string(),
            details: None,
            checked_at: None,
        }),
        SessionState::Paused { paused_at, .. } => Some(StartupVerificationSummary {
            state: StartupVerificationState::Idle,
            headline: "Workspace should be verified before resuming".to_string(),
            details: None,
            checked_at: Some(paused_at.to_rfc3339()),
        }),
        SessionState::Error {
            message,
            recoverable: true,
        } => Some(StartupVerificationSummary {
            state: StartupVerificationState::Failed,
            headline: "Session needs verification before resuming".to_string(),
            details: Some(vec![message.clone()]),
            checked_at: Some(session.updated_at.to_rfc3339()),
        }),
        _ => None,
    };

    let handoff = match &session.state {
        SessionState::Paused {
            paused_at, message, ..
        } => {
            let current_subtask = most_relevant_subtask(tasks);
            Some(SessionHandoffSummary {
                last_completed_increment: latest_completed_title(tasks)
                    .unwrap_or_else(|| "No completed increment recorded yet".to_string()),
                current_objective: current_subtask
                    .map(|subtask| subtask.title.clone())
                    .unwrap_or_else(|| message.clone()),
                next_recommended_action: "Verify workspace state, then resume or replan"
                    .to_string(),
                open_issues: vec![message.clone()],
                environment_health: EnvironmentHealth::Unknown,
                recorded_at: Some(paused_at.to_rfc3339()),
            })
        }
        SessionState::Finished { completed_at } => Some(SessionHandoffSummary {
            last_completed_increment: latest_completed_title(tasks)
                .unwrap_or_else(|| "Latest run completed".to_string()),
            current_objective: "Awaiting the next request".to_string(),
            next_recommended_action: "Start the next bounded task".to_string(),
            open_issues: vec![],
            environment_health: EnvironmentHealth::Healthy,
            recorded_at: Some(completed_at.to_rfc3339()),
        }),
        _ => None,
    };

    AgentSessionHarnessState {
        mode,
        startup_verification,
        handoff,
        clean_exit_recorded: matches!(session.state, SessionState::Finished { .. }),
    }
}

/// Rich runtime view of a session for frontend inspection.
#[derive(Debug, Clone)]
pub struct AgentSessionRuntimeDetails {
    /// The agent session data.
    pub session: AgentSession,
    /// Files that were touched during the session.
    pub files_touched: Vec<FileTouchedView>,
    /// Terminal output lines captured during execution.
    pub terminal_output: Vec<TerminalLineView>,
    /// Execution steps recorded during the session.
    pub execution_steps: Vec<serde_json::Value>,
    /// Status phrases reported during execution.
    pub status_phrases: Vec<String>,
    /// Tasks associated with this session.
    pub tasks: Vec<AgentTask>,
    /// Tool calls made during the session.
    pub tool_calls: Vec<ToolCallView>,
    /// Long-running harness session metadata for UI workflow alignment.
    pub harness_state: Option<AgentSessionHarnessState>,
}

/// Deep execution debug data used by the development trace window.
#[derive(Debug, Clone)]
pub struct AgentExecutionDebugDetails {
    /// Snapshot of the execution state at a point in time.
    pub snapshot: Option<ExecutionSnapshot>,
    /// Runtime traces collected during execution.
    pub traces: Vec<RuntimeTrace>,
}

/// Prepared execution context for an agent request.
#[derive(Debug, Clone)]
pub struct AgentExecutionContext {
    /// The agent session.
    pub session: AgentSession,
    /// The system prompt for the agent.
    pub system_prompt: String,
    /// The trust mode for execution.
    pub trust_mode: crate::core::agent::types::TrustMode,
}

/// Runtime read model used by approval commands.
#[derive(Debug, Clone)]
pub struct PendingActionsView {
    /// List of pending actions awaiting approval.
    pub actions: Vec<PendingAction>,
}

/// Runtime read model used by registry commands.
#[derive(Debug, Clone)]
pub struct AgentCatalog {
    /// List of available agents.
    pub agents: Vec<Agent>,
}

/// Public view of a touched file in a session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTouchedView {
    /// Path to the file.
    pub path: String,
    /// Whether the file was modified (vs just read).
    pub modified: bool,
    /// The action performed on the file.
    pub action: String,
    /// The diff of changes made (if available).
    pub diff: Option<String>,
}

/// Public view of a terminal output line in a session.
#[derive(Debug, Clone, Serialize)]
pub struct TerminalLineView {
    /// Unique identifier for this line.
    pub id: String,
    /// The type of line (e.g., "stdout", "stderr").
    #[serde(rename = "type")]
    pub line_type: String,
    /// The content of the line.
    pub content: String,
    /// Timestamp when this line was generated.
    pub timestamp: String,
}

/// Public view of a recorded tool call in a session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallView {
    /// Unique identifier for this tool call.
    pub id: String,
    /// The session this tool call belongs to.
    pub session_id: String,
    /// The name of the tool that was called.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub arguments: serde_json::Value,
    /// Result returned by the tool (if completed).
    pub result: Option<serde_json::Value>,
    /// Whether the tool call resulted in an error.
    pub is_error: bool,
    /// Duration of the tool call in milliseconds.
    pub duration_ms: Option<i64>,
    /// The subtask ID this tool call belongs to (if any).
    pub subtask_id: Option<String>,
    /// Timestamp when this tool call was recorded.
    pub timestamp: String,
}
