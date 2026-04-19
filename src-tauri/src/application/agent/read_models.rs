//! Application-facing read models for the agent backend.
//!
//! These structs aggregate runtime data needed by the Tauri command layer so
//! IPC handlers can depend on a stable application surface instead of reaching
//! directly into runtime and repository internals.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::application::agent::harness::HarnessStateOverrides;
use crate::core::agent::execution::{ExecutionSnapshot, RuntimeTrace};
use crate::core::agent::session::{AgentSession, PendingAction};
use crate::core::agent::session::SessionState;
use crate::core::agent::task::{AgentTask, SubTask, TaskStatus};
use crate::core::agent::types::Agent;

/// Long-running harness role for a session.
///
/// `AwaitingClarification` (Workstream E.2) captures a runtime-blocking
/// question the agent has posed to the user via `agent_narrate` with
/// `intent = "question"`. The main loop refuses to call the LLM while
/// the harness is in this mode; user replies clear it back to the prior
/// mode (typically `Worker`), and a timeout fails the current subtask
/// with `unanswered_clarification` (E.3).
///
/// The prompt + timestamp that accompany the awaiting variant travel
/// alongside the mode on [`AgentSessionHarnessState::awaiting_clarification`]
/// so the wire format remains a flat string mode for the existing
/// frontend comparisons.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HarnessMode {
    /// Session is bootstrapping and preparing an execution context.
    Initializer,
    /// Session is performing normal work within an active or completed run.
    Worker,
    /// Session is orienting itself for a resumed run after interruption or pause.
    ResumeVerifier,
    /// Session is paused waiting for the user to answer a clarification
    /// question emitted via `agent_narrate` with `intent = "question"`.
    /// Accompanying prompt + `asked_at` live on
    /// [`AgentSessionHarnessState::awaiting_clarification`].
    AwaitingClarification,
}

/// Accompaniment to [`HarnessMode::AwaitingClarification`] carrying the
/// exact question the agent emitted and the timestamp it was asked at.
///
/// Exposed on [`AgentSessionHarnessState`] so the AgentPanel can render
/// the question inline with a reply affordance (Workstream E.4) without
/// reshaping the existing flat `mode` string.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AwaitingClarificationDetails {
    /// Exact prompt the agent emitted.
    pub prompt: String,
    /// UTC timestamp at which the question was asked. The timeout
    /// fallback (E.3) computes elapsed wait against this.
    pub asked_at: DateTime<Utc>,
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
    /// Details for the awaiting-clarification mode (Workstream E.2/E.4).
    /// Present if and only if `mode == HarnessMode::AwaitingClarification`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub awaiting_clarification: Option<AwaitingClarificationDetails>,
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
///
/// `overrides`, when provided, replaces each corresponding derived field that
/// it has recorded. Fields the store has not recorded stay at the conservative
/// derived value, so callers without a store still get the same behavior.
#[must_use]
pub fn derive_harness_state(
    session: &AgentSession,
    tasks: &[AgentTask],
    overrides: Option<&HarnessStateOverrides>,
) -> AgentSessionHarnessState {
    let derived_mode = match session.state {
        SessionState::Launching => HarnessMode::Initializer,
        SessionState::Paused { .. } => HarnessMode::ResumeVerifier,
        SessionState::AwaitingClarification { .. } => HarnessMode::AwaitingClarification,
        _ => HarnessMode::Worker,
    };

    let derived_startup_verification = match &session.state {
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

    let derived_handoff = match &session.state {
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

    let derived_clean_exit = matches!(session.state, SessionState::Finished { .. });

    let derived_awaiting_clarification = match &session.state {
        SessionState::AwaitingClarification {
            prompt, asked_at, ..
        } => Some(AwaitingClarificationDetails {
            prompt: prompt.clone(),
            asked_at: *asked_at,
        }),
        _ => None,
    };

    let (mode, startup_verification, handoff, clean_exit_recorded, awaiting_clarification) =
        match overrides {
            Some(values) => (
                values.mode.clone().unwrap_or(derived_mode),
                values
                    .startup_verification
                    .clone()
                    .or(derived_startup_verification),
                values.handoff.clone().or(derived_handoff),
                values.clean_exit_recorded.unwrap_or(derived_clean_exit),
                values
                    .awaiting_clarification
                    .clone()
                    .or(derived_awaiting_clarification),
            ),
            None => (
                derived_mode,
                derived_startup_verification,
                derived_handoff,
                derived_clean_exit,
                derived_awaiting_clarification,
            ),
        };

    // Keep the details and the mode in lockstep: when the store
    // records a non-clarification mode, drop any stale awaiting details.
    let awaiting_clarification = match &mode {
        HarnessMode::AwaitingClarification => awaiting_clarification,
        _ => None,
    };

    AgentSessionHarnessState {
        mode,
        startup_verification,
        handoff,
        awaiting_clarification,
        clean_exit_recorded,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::agent::harness::HarnessStateOverrides;
    use crate::core::agent::session::{AgentSession, SessionState};
    use crate::core::agent::types::AgentId;
    use chrono::Utc;

    fn running_session() -> AgentSession {
        let mut session = AgentSession::new(
            AgentId::new("test-agent"),
            "Test Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        );
        session
            .set_state(SessionState::Running {
                current_step: 1,
                context_usage: 0.0,
            })
            .expect("running transition should be valid");
        session
    }

    fn finished_session() -> AgentSession {
        let mut session = running_session();
        session
            .set_state(SessionState::Finished {
                completed_at: Utc::now(),
            })
            .expect("finished transition should be valid");
        session
    }

    fn sample_startup_verification() -> StartupVerificationSummary {
        StartupVerificationSummary {
            state: StartupVerificationState::Passed,
            headline: "Recorded workspace check".to_string(),
            details: Some(vec!["cargo check ok".to_string()]),
            checked_at: Some("2026-04-17T12:00:00Z".to_string()),
        }
    }

    fn sample_handoff() -> SessionHandoffSummary {
        SessionHandoffSummary {
            last_completed_increment: "Recorded slice landed".to_string(),
            current_objective: "Expose handoff in session summary".to_string(),
            next_recommended_action: "Run validation".to_string(),
            open_issues: vec![],
            environment_health: EnvironmentHealth::Healthy,
            recorded_at: Some("2026-04-17T12:05:00Z".to_string()),
        }
    }

    #[test]
    fn derive_without_overrides_produces_derived_defaults_for_running_session() {
        let session = running_session();
        let state = derive_harness_state(&session, &[], None);
        assert_eq!(state.mode, HarnessMode::Worker);
        assert!(state.startup_verification.is_none());
        assert!(state.handoff.is_none());
        assert!(!state.clean_exit_recorded);
    }

    #[test]
    fn derive_without_overrides_marks_finished_session_as_clean_exit() {
        let session = finished_session();
        let state = derive_harness_state(&session, &[], None);
        assert!(state.clean_exit_recorded);
        assert!(state.handoff.is_some());
    }

    #[test]
    fn overrides_replace_each_recorded_field() {
        let session = running_session();
        let overrides = HarnessStateOverrides {
            mode: Some(HarnessMode::ResumeVerifier),
            startup_verification: Some(sample_startup_verification()),
            handoff: Some(sample_handoff()),
            clean_exit_recorded: Some(true),
            awaiting_clarification: None,
        };
        let state = derive_harness_state(&session, &[], Some(&overrides));
        assert_eq!(state.mode, HarnessMode::ResumeVerifier);
        assert_eq!(state.startup_verification, Some(sample_startup_verification()));
        assert_eq!(state.handoff, Some(sample_handoff()));
        assert!(state.clean_exit_recorded);
    }

    #[test]
    fn empty_overrides_preserve_derived_defaults() {
        let session = finished_session();
        let overrides = HarnessStateOverrides::default();
        let state = derive_harness_state(&session, &[], Some(&overrides));
        assert_eq!(state.mode, HarnessMode::Worker);
        assert!(state.clean_exit_recorded);
        assert!(state.handoff.is_some());
    }

    #[test]
    fn awaiting_clarification_override_flows_through_derive() {
        // Workstream E.2 read-model assertion: when the store records
        // the awaiting-clarification mode together with the details,
        // both travel to the outer read model exactly once.
        let session = running_session();
        let asked_at = chrono::DateTime::parse_from_rfc3339("2026-04-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let overrides = HarnessStateOverrides {
            mode: Some(HarnessMode::AwaitingClarification),
            startup_verification: None,
            handoff: None,
            clean_exit_recorded: None,
            awaiting_clarification: Some(AwaitingClarificationDetails {
                prompt: "What is the product name?".to_string(),
                asked_at,
            }),
        };
        let state = derive_harness_state(&session, &[], Some(&overrides));
        assert_eq!(state.mode, HarnessMode::AwaitingClarification);
        let details = state
            .awaiting_clarification
            .expect("awaiting details should flow through");
        assert_eq!(details.prompt, "What is the product name?");
    }

    #[test]
    fn session_state_awaiting_clarification_drives_mode_and_details() {
        // Phase 1 of the agent-clarification-resume plan: when the
        // session state itself is AwaitingClarification, derive_harness_state
        // must report both the mode and the prompt/asked_at details
        // without needing the override path.
        let mut session = running_session();
        let asked_at = chrono::DateTime::parse_from_rfc3339("2026-04-19T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        session
            .set_state(SessionState::AwaitingClarification {
                prompt: "Which workspace?".to_string(),
                asked_at,
                subtask_id: Some("subtask-1".to_string()),
            })
            .expect("running → awaiting_clarification is valid");
        let state = derive_harness_state(&session, &[], None);
        assert_eq!(state.mode, HarnessMode::AwaitingClarification);
        let details = state
            .awaiting_clarification
            .expect("details should be derived from session state");
        assert_eq!(details.prompt, "Which workspace?");
        assert_eq!(details.asked_at, asked_at);
    }

    #[test]
    fn awaiting_details_are_dropped_when_mode_is_not_awaiting() {
        // Stale overrides recorded during an earlier clarification must
        // not surface once the mode has moved back to Worker.
        let session = running_session();
        let asked_at = chrono::DateTime::parse_from_rfc3339("2026-04-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let overrides = HarnessStateOverrides {
            mode: Some(HarnessMode::Worker),
            startup_verification: None,
            handoff: None,
            clean_exit_recorded: None,
            awaiting_clarification: Some(AwaitingClarificationDetails {
                prompt: "stale".to_string(),
                asked_at,
            }),
        };
        let state = derive_harness_state(&session, &[], Some(&overrides));
        assert_eq!(state.mode, HarnessMode::Worker);
        assert!(state.awaiting_clarification.is_none());
    }

    #[test]
    fn partial_overrides_keep_unrecorded_fields_at_derived_values() {
        let session = finished_session();
        let overrides = HarnessStateOverrides {
            mode: None,
            startup_verification: Some(sample_startup_verification()),
            handoff: None,
            clean_exit_recorded: None,
            awaiting_clarification: None,
        };
        let state = derive_harness_state(&session, &[], Some(&overrides));
        // mode and clean_exit_recorded stay derived.
        assert_eq!(state.mode, HarnessMode::Worker);
        assert!(state.clean_exit_recorded);
        // startup_verification replaced.
        assert_eq!(state.startup_verification, Some(sample_startup_verification()));
        // handoff stays at the derived "finished" summary.
        assert!(
            state.handoff.is_some(),
            "handoff should keep the derived finished summary"
        );
    }
}
