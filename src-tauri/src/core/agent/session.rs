//! Defines the core types for agent sessions.
//!
//! A session represents a single, stateful instance of an agent's execution,
//! tied to a specific conversation and thread. It is responsible for tracking
//! the agent's lifecycle state and managing any actions that require user approval.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use super::types::{ActionPreview, ActionType, AgentId};

/// A unique identifier for an agent session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    /// Creates a new, randomly generated `SessionId`.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Represents a running, stateful agent session.
///
/// This struct is ephemeral and holds all the runtime state for an agent's
/// execution, including its current lifecycle state, workspace, and relationship
/// to other sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    /// The unique identifier for this session.
    pub id: SessionId,
    /// The ID of the agent type being used in this session.
    pub agent_id: AgentId,
    /// The display name of the agent.
    pub agent_name: String,
    /// The ID of the conversation this session belongs to.
    pub conversation_id: String,
    /// The ID of the thread this session is associated with.
    pub thread_id: String,
    /// The current lifecycle state of the session.
    pub state: SessionState,
    /// The workspace root directory for file operations.
    pub workspace_root: Option<PathBuf>,
    /// The ID of the parent session, if this is a sub-session.
    pub parent_session_id: Option<SessionId>,
    /// The cached model ID from the conversation settings.
    pub model_id: Option<String>,
    /// Workstream F.2 — flipped to `true` when any subtask is marked
    /// `Failed` through `agent_mark_subtask_status`. Persists across
    /// reload so the UI can surface a "needs review" affordance for
    /// sessions that ended with at least one unsatisfied contract or
    /// explicitly failed subtask.
    #[serde(default)]
    pub needs_review: bool,
    /// The timestamp of when the session was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of when the session was last updated.
    pub updated_at: DateTime<Utc>,
}

impl AgentSession {
    /// Creates a new `AgentSession` for a given agent and conversation.
    #[must_use]
    pub fn new(
        agent_id: AgentId,
        agent_name: String,
        conversation_id: String,
        thread_id: String,
        workspace_root: Option<PathBuf>,
        model_id: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            agent_id,
            agent_name,
            conversation_id,
            thread_id,
            state: SessionState::Launching,
            workspace_root,
            parent_session_id: None,
            model_id,
            needs_review: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Flags the session for post-run review (Workstream F.2).
    ///
    /// Called when a subtask is explicitly marked `Failed` through
    /// `agent_mark_subtask_status`. The flag is persisted through the
    /// session repository so the UI still surfaces it after reload.
    pub fn mark_needs_review(&mut self) {
        self.needs_review = true;
        self.updated_at = Utc::now();
    }

    /// Transitions the session to a new state and updates the `updated_at` timestamp.
    ///
    /// Returns `Err` with a description if the transition is not valid according
    /// to the session state machine. Terminal states (`Finished`, `Cancelled`,
    /// unrecoverable `Error`) cannot transition to any other state.
    pub fn set_state(&mut self, state: SessionState) -> Result<(), String> {
        if !self.state.can_transition_to(&state) {
            return Err(format!(
                "Invalid session state transition: {:?} -> {:?}",
                self.state.variant_name(),
                state.variant_name(),
            ));
        }
        self.state = state;
        self.updated_at = Utc::now();
        Ok(())
    }
}

/// Represents the lifecycle state of an agent session.
///
/// This enum defines a state machine that governs the behavior of an `AgentSession`
/// as it progresses through its execution.
///
/// # Serialization
///
/// Serializes with `#[serde(tag = "type")]` (canonical form). Deserialization
/// accepts both the canonical `"type"` tag and the legacy `"state"` tag used by
/// pre-consolidation code paths; see the manual `Deserialize` impl below.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionState {
    /// The session is being initialized.
    Launching,
    /// The agent is actively executing a task.
    Running {
        /// The current step number in the execution process.
        #[serde(rename = "currentStep")]
        current_step: u32,
        /// The current context usage as a percentage of the context window.
        #[serde(rename = "contextUsage")]
        context_usage: f64,
    },
    /// The agent has finished a request and is waiting for the next one.
    WaitForRequest {
        /// The timestamp of the last action.
        #[serde(rename = "lastActionAt")]
        last_action_at: DateTime<Utc>,
    },
    /// The agent is blocked, waiting for user approval on one or more actions.
    UserInputRequired {
        /// A list of IDs for the pending actions.
        #[serde(rename = "pendingActionIds")]
        pending_action_ids: Vec<String>,
    },
    /// The session has completed successfully.
    Finished {
        /// The timestamp of when the session was completed.
        #[serde(rename = "completedAt")]
        completed_at: DateTime<Utc>,
    },
    /// The session was cancelled by the user.
    Cancelled {
        /// The reason for the cancellation.
        reason: String,
    },
    /// The session has encountered an error.
    Error {
        /// The error message.
        message: String,
        /// A flag indicating whether the error is recoverable.
        recoverable: bool,
    },
    /// The session has been paused and can be resumed later.
    Paused {
        /// The timestamp of when the session was paused.
        #[serde(rename = "pausedAt")]
        paused_at: DateTime<Utc>,
        /// A message describing the reason for pausing.
        message: String,
        /// The ID of the current subtask, if any.
        #[serde(rename = "currentSubtaskId", skip_serializing_if = "Option::is_none")]
        current_subtask_id: Option<String>,
    },
}

impl SessionState {
    /// Returns the short variant name for diagnostics.
    #[must_use]
    pub const fn variant_name(&self) -> &'static str {
        match self {
            Self::Launching => "Launching",
            Self::Running { .. } => "Running",
            Self::WaitForRequest { .. } => "WaitForRequest",
            Self::UserInputRequired { .. } => "UserInputRequired",
            Self::Finished { .. } => "Finished",
            Self::Cancelled { .. } => "Cancelled",
            Self::Error { .. } => "Error",
            Self::Paused { .. } => "Paused",
        }
    }

    /// Checks whether transitioning from `self` to `target` is valid.
    ///
    /// Terminal states (`Finished`, `Cancelled`, unrecoverable `Error`) cannot
    /// transition further. Recoverable errors may transition back to `Running`
    /// or to `Cancelled`.
    #[must_use]
    pub fn can_transition_to(&self, target: &Self) -> bool {
        match self {
            // Launching can proceed to Running, or terminate early
            Self::Launching => matches!(
                target,
                Self::Running { .. } | Self::Error { .. } | Self::Cancelled { .. }
            ),
            // Running can go to many states
            Self::Running { .. } => matches!(
                target,
                Self::WaitForRequest { .. }
                    | Self::UserInputRequired { .. }
                    | Self::Finished { .. }
                    | Self::Error { .. }
                    | Self::Cancelled { .. }
                    | Self::Paused { .. }
            ),
            // WaitForRequest can resume or terminate
            Self::WaitForRequest { .. } => matches!(
                target,
                Self::Running { .. }
                    | Self::Finished { .. }
                    | Self::Error { .. }
                    | Self::Cancelled { .. }
                    | Self::Paused { .. }
            ),
            // UserInputRequired resolves to Running or terminates
            Self::UserInputRequired { .. } => matches!(
                target,
                Self::Running { .. } | Self::Error { .. } | Self::Cancelled { .. }
            ),
            // Paused can resume or terminate
            Self::Paused { .. } => matches!(
                target,
                Self::Running { .. } | Self::Error { .. } | Self::Cancelled { .. }
            ),
            // Recoverable errors can retry or be cancelled
            Self::Error { recoverable, .. } => {
                if *recoverable {
                    matches!(target, Self::Running { .. } | Self::Cancelled { .. })
                } else {
                    false
                }
            }
            // Terminal states
            Self::Finished { .. } | Self::Cancelled { .. } => false,
        }
    }
}

/// Shadow of [`SessionState`] used only to reuse serde's derived tag-based
/// deserializer for the canonical `"type"`-tagged form. This is the private
/// half of the compatibility deserializer below.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SessionStateCanonical {
    Launching,
    Running {
        #[serde(rename = "currentStep")]
        current_step: u32,
        #[serde(rename = "contextUsage")]
        context_usage: f64,
    },
    WaitForRequest {
        #[serde(rename = "lastActionAt")]
        last_action_at: DateTime<Utc>,
    },
    UserInputRequired {
        #[serde(rename = "pendingActionIds")]
        pending_action_ids: Vec<String>,
    },
    Finished {
        #[serde(rename = "completedAt")]
        completed_at: DateTime<Utc>,
    },
    Cancelled {
        reason: String,
    },
    Error {
        message: String,
        recoverable: bool,
    },
    Paused {
        #[serde(rename = "pausedAt")]
        paused_at: DateTime<Utc>,
        message: String,
        #[serde(rename = "currentSubtaskId", default)]
        current_subtask_id: Option<String>,
    },
}

impl From<SessionStateCanonical> for SessionState {
    fn from(value: SessionStateCanonical) -> Self {
        match value {
            SessionStateCanonical::Launching => Self::Launching,
            SessionStateCanonical::Running {
                current_step,
                context_usage,
            } => Self::Running {
                current_step,
                context_usage,
            },
            SessionStateCanonical::WaitForRequest { last_action_at } => {
                Self::WaitForRequest { last_action_at }
            }
            SessionStateCanonical::UserInputRequired { pending_action_ids } => {
                Self::UserInputRequired { pending_action_ids }
            }
            SessionStateCanonical::Finished { completed_at } => Self::Finished { completed_at },
            SessionStateCanonical::Cancelled { reason } => Self::Cancelled { reason },
            SessionStateCanonical::Error {
                message,
                recoverable,
            } => Self::Error {
                message,
                recoverable,
            },
            SessionStateCanonical::Paused {
                paused_at,
                message,
                current_subtask_id,
            } => Self::Paused {
                paused_at,
                message,
                current_subtask_id,
            },
        }
    }
}

/// Compatibility prefix used by the service layer to tag recoverable errors.
/// Mirrored here (rather than imported from the application layer) so the core
/// module stays free of upward dependencies. Keep in sync with
/// `application::agent::service::AgentServiceImpl::is_recoverable_error_message`.
const RECOVERABLE_ERROR_PREFIX: &str = "[recoverable]";

fn parse_legacy_error(error: &str) -> (bool, String) {
    if let Some(rest) = error.strip_prefix(RECOVERABLE_ERROR_PREFIX) {
        (true, rest.trim_start().to_string())
    } else {
        (false, error.to_string())
    }
}

/// Deserializes the historical legacy `#[serde(tag = "state")]` shape into the
/// canonical [`SessionState`]. Only the variants that appeared in the pre-
/// consolidation `entities.rs` enum are handled; anything else is an error.
fn deserialize_legacy_state<E: serde::de::Error>(
    value: &serde_json::Value,
) -> Result<SessionState, E> {
    let obj = value
        .as_object()
        .ok_or_else(|| E::custom("legacy SessionState must be a JSON object"))?;
    let variant = obj
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or_else(|| E::custom("legacy SessionState missing `state` discriminator"))?;

    // Helpers used across arms.
    let parse_dt = |key: &str| -> Result<DateTime<Utc>, E> {
        let v = obj
            .get(key)
            .ok_or_else(|| E::custom(format!("legacy SessionState missing `{key}`")))?;
        serde_json::from_value::<DateTime<Utc>>(v.clone())
            .map_err(|err| E::custom(format!("legacy SessionState invalid `{key}`: {err}")))
    };
    let get_str = |key: &str| -> String {
        obj.get(key)
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default()
    };

    match variant {
        "processing" => Ok(SessionState::Running {
            current_step: 0,
            context_usage: 0.0,
        }),
        "awaiting_approval" => {
            let action_id = get_str("action_id");
            let ids = if action_id.is_empty() {
                Vec::new()
            } else {
                vec![action_id]
            };
            Ok(SessionState::UserInputRequired {
                pending_action_ids: ids,
            })
        }
        "wait_for_request" => Ok(SessionState::WaitForRequest {
            last_action_at: parse_dt("last_action_at")?,
        }),
        "finished" => Ok(SessionState::Finished {
            completed_at: parse_dt("completed_at")?,
        }),
        "cancelled" => Ok(SessionState::Cancelled {
            reason: get_str("reason"),
        }),
        "error" => {
            let error = get_str("error");
            let (recoverable, message) = parse_legacy_error(&error);
            Ok(SessionState::Error {
                message,
                recoverable,
            })
        }
        "paused" => Ok(SessionState::Paused {
            paused_at: parse_dt("paused_at")?,
            message: get_str("message"),
            current_subtask_id: None,
        }),
        other => Err(E::custom(format!(
            "unknown legacy SessionState variant: {other}"
        ))),
    }
}

impl<'de> Deserialize<'de> for SessionState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let is_legacy = value
            .as_object()
            .map(|obj| !obj.contains_key("type") && obj.contains_key("state"))
            .unwrap_or(false);
        if is_legacy {
            deserialize_legacy_state::<D::Error>(&value)
        } else {
            SessionStateCanonical::deserialize(value)
                .map(Into::into)
                .map_err(serde::de::Error::custom)
        }
    }
}

/// Represents a pending action that is awaiting user approval.
///
/// When an agent needs to perform a sensitive action, it creates a `PendingAction`
/// and waits for the user to approve or deny it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingAction {
    /// The unique identifier for this action.
    pub id: String,
    /// The ID of the session this action belongs to.
    pub session_id: SessionId,
    /// The name of the tool that generated this action.
    pub tool_name: String,
    /// The type of the action to be performed.
    pub action_type: ActionType,
    /// A user-friendly description of the action.
    pub description: String,
    /// An optional preview of the action's effects.
    pub preview: Option<ActionPreview>,
    /// The arguments for the tool call.
    pub arguments: serde_json::Value,
    /// The timestamp of when the action was created.
    pub created_at: DateTime<Utc>,
}

impl PendingAction {
    /// Creates a new `PendingAction` from a tool call.
    #[must_use]
    pub fn new(
        session_id: SessionId,
        tool_name: String,
        action_type: ActionType,
        description: String,
        arguments: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            tool_name,
            action_type,
            description,
            preview: None,
            arguments,
            created_at: Utc::now(),
        }
    }

    /// Attaches a preview to this pending action.
    #[must_use]
    pub fn with_preview(mut self, preview: ActionPreview) -> Self {
        self.preview = Some(preview);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_session(state: SessionState) -> AgentSession {
        let mut session = AgentSession::new(
            AgentId::new("test"),
            "Test Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            Some(PathBuf::from("/workspace")),
            None,
        );
        // Force the initial state for testing.
        session.state = state;
        session
    }

    fn running() -> SessionState {
        SessionState::Running {
            current_step: 1,
            context_usage: 0.5,
        }
    }

    fn wait_for_request() -> SessionState {
        SessionState::WaitForRequest {
            last_action_at: Utc::now(),
        }
    }

    fn finished() -> SessionState {
        SessionState::Finished {
            completed_at: Utc::now(),
        }
    }

    fn cancelled() -> SessionState {
        SessionState::Cancelled {
            reason: "user cancelled".to_string(),
        }
    }

    fn recoverable_error() -> SessionState {
        SessionState::Error {
            message: "timeout".to_string(),
            recoverable: true,
        }
    }

    fn unrecoverable_error() -> SessionState {
        SessionState::Error {
            message: "fatal".to_string(),
            recoverable: false,
        }
    }

    fn paused() -> SessionState {
        SessionState::Paused {
            paused_at: Utc::now(),
            message: "paused".to_string(),
            current_subtask_id: None,
        }
    }

    fn user_input_required() -> SessionState {
        SessionState::UserInputRequired {
            pending_action_ids: vec!["action-1".to_string()],
        }
    }

    // ---- Valid transitions ----

    #[test]
    fn launching_to_running_is_valid() {
        let mut session = make_session(SessionState::Launching);
        assert!(session.set_state(running()).is_ok());
    }

    #[test]
    fn launching_to_error_is_valid() {
        let mut session = make_session(SessionState::Launching);
        assert!(session.set_state(unrecoverable_error()).is_ok());
    }

    #[test]
    fn running_to_finished_is_valid() {
        let mut session = make_session(running());
        assert!(session.set_state(finished()).is_ok());
    }

    #[test]
    fn running_to_paused_is_valid() {
        let mut session = make_session(running());
        assert!(session.set_state(paused()).is_ok());
    }

    #[test]
    fn running_to_user_input_required_is_valid() {
        let mut session = make_session(running());
        assert!(session.set_state(user_input_required()).is_ok());
    }

    #[test]
    fn wait_for_request_to_running_is_valid() {
        let mut session = make_session(wait_for_request());
        assert!(session.set_state(running()).is_ok());
    }

    #[test]
    fn paused_to_running_is_valid() {
        let mut session = make_session(paused());
        assert!(session.set_state(running()).is_ok());
    }

    #[test]
    fn recoverable_error_to_running_is_valid() {
        let mut session = make_session(recoverable_error());
        assert!(session.set_state(running()).is_ok());
    }

    #[test]
    fn recoverable_error_to_cancelled_is_valid() {
        let mut session = make_session(recoverable_error());
        assert!(session.set_state(cancelled()).is_ok());
    }

    #[test]
    fn user_input_required_to_running_is_valid() {
        let mut session = make_session(user_input_required());
        assert!(session.set_state(running()).is_ok());
    }

    // ---- Invalid transitions ----

    #[test]
    fn finished_to_running_is_invalid() {
        let mut session = make_session(finished());
        assert!(session.set_state(running()).is_err());
    }

    #[test]
    fn cancelled_to_running_is_invalid() {
        let mut session = make_session(cancelled());
        assert!(session.set_state(running()).is_err());
    }

    #[test]
    fn unrecoverable_error_to_running_is_invalid() {
        let mut session = make_session(unrecoverable_error());
        assert!(session.set_state(running()).is_err());
    }

    #[test]
    fn launching_to_finished_is_invalid() {
        let mut session = make_session(SessionState::Launching);
        assert!(session.set_state(finished()).is_err());
    }

    #[test]
    fn launching_to_paused_is_invalid() {
        let mut session = make_session(SessionState::Launching);
        assert!(session.set_state(paused()).is_err());
    }

    // ---- variant_name ----

    #[test]
    fn variant_name_returns_correct_names() {
        assert_eq!(SessionState::Launching.variant_name(), "Launching");
        assert_eq!(running().variant_name(), "Running");
        assert_eq!(finished().variant_name(), "Finished");
        assert_eq!(cancelled().variant_name(), "Cancelled");
        assert_eq!(paused().variant_name(), "Paused");
    }

    // ---- Legacy serde compatibility ----

    #[test]
    fn deserializes_canonical_running() {
        let json = serde_json::json!({
            "type": "running",
            "currentStep": 3,
            "contextUsage": 0.42,
        });
        let state: SessionState = serde_json::from_value(json).expect("canonical running");
        match state {
            SessionState::Running {
                current_step,
                context_usage,
            } => {
                assert_eq!(current_step, 3);
                assert!((context_usage - 0.42).abs() < f64::EPSILON);
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn legacy_processing_deserializes_to_running() {
        let json = serde_json::json!({
            "state": "processing",
            "started_at": "2026-04-16T12:00:00Z",
        });
        let state: SessionState = serde_json::from_value(json).expect("legacy processing");
        match state {
            SessionState::Running {
                current_step,
                context_usage,
            } => {
                assert_eq!(current_step, 0);
                assert!((context_usage).abs() < f64::EPSILON);
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn legacy_awaiting_approval_deserializes_to_user_input_required() {
        let json = serde_json::json!({
            "state": "awaiting_approval",
            "action_id": "act-1",
            "requested_at": "2026-04-16T12:00:00Z",
        });
        let state: SessionState = serde_json::from_value(json).expect("legacy awaiting_approval");
        match state {
            SessionState::UserInputRequired { pending_action_ids } => {
                assert_eq!(pending_action_ids, vec!["act-1".to_string()]);
            }
            other => panic!("expected UserInputRequired, got {other:?}"),
        }
    }

    #[test]
    fn legacy_finished_deserializes_dropping_result() {
        let json = serde_json::json!({
            "state": "finished",
            "completed_at": "2026-04-16T12:00:00Z",
            "result": "this field is dropped during migration",
        });
        let state: SessionState = serde_json::from_value(json).expect("legacy finished");
        assert!(matches!(state, SessionState::Finished { .. }));
    }

    #[test]
    fn legacy_cancelled_deserializes_dropping_cancelled_at() {
        let json = serde_json::json!({
            "state": "cancelled",
            "cancelled_at": "2026-04-16T12:00:00Z",
            "reason": "user cancelled",
        });
        let state: SessionState = serde_json::from_value(json).expect("legacy cancelled");
        match state {
            SessionState::Cancelled { reason } => assert_eq!(reason, "user cancelled"),
            other => panic!("expected Cancelled, got {other:?}"),
        }
    }

    #[test]
    fn legacy_error_recoverable_prefix_is_classified() {
        let json = serde_json::json!({
            "state": "error",
            "error_at": "2026-04-16T12:00:00Z",
            "error": "[recoverable] transient timeout",
        });
        let state: SessionState = serde_json::from_value(json).expect("legacy error");
        match state {
            SessionState::Error {
                message,
                recoverable,
            } => {
                assert!(recoverable);
                assert_eq!(message, "transient timeout");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn legacy_error_without_prefix_is_unrecoverable() {
        let json = serde_json::json!({
            "state": "error",
            "error_at": "2026-04-16T12:00:00Z",
            "error": "fatal configuration error",
        });
        let state: SessionState = serde_json::from_value(json).expect("legacy error");
        match state {
            SessionState::Error {
                message,
                recoverable,
            } => {
                assert!(!recoverable);
                assert_eq!(message, "fatal configuration error");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn legacy_paused_deserializes_with_none_subtask_id() {
        let json = serde_json::json!({
            "state": "paused",
            "paused_at": "2026-04-16T12:00:00Z",
            "message": "awaiting input",
        });
        let state: SessionState = serde_json::from_value(json).expect("legacy paused");
        match state {
            SessionState::Paused {
                message,
                current_subtask_id,
                ..
            } => {
                assert_eq!(message, "awaiting input");
                assert!(current_subtask_id.is_none());
            }
            other => panic!("expected Paused, got {other:?}"),
        }
    }

    #[test]
    fn legacy_wait_for_request_deserializes_unchanged() {
        let json = serde_json::json!({
            "state": "wait_for_request",
            "last_action_at": "2026-04-16T12:00:00Z",
        });
        let state: SessionState =
            serde_json::from_value(json).expect("legacy wait_for_request");
        assert!(matches!(state, SessionState::WaitForRequest { .. }));
    }

    #[test]
    fn canonical_serialization_roundtrips() {
        let original = SessionState::Running {
            current_step: 7,
            context_usage: 0.8,
        };
        let json = serde_json::to_value(&original).expect("serialize");
        let roundtripped: SessionState = serde_json::from_value(json).expect("deserialize");
        assert!(
            matches!(
                roundtripped,
                SessionState::Running {
                    current_step: 7,
                    ..
                }
            ),
            "roundtrip failed: {roundtripped:?}"
        );
    }

    #[test]
    fn unknown_legacy_variant_is_an_error() {
        let json = serde_json::json!({
            "state": "invalid_variant",
        });
        let err: Result<SessionState, _> = serde_json::from_value(json);
        assert!(err.is_err());
    }
}
