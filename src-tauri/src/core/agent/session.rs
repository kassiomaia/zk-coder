//! Defines the core types for agent sessions.
//!
//! A session represents a single, stateful instance of an agent's execution,
//! tied to a specific conversation and thread. It is responsible for tracking
//! the agent's lifecycle state and managing any actions that require user approval.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{ActionPreview, ActionType, AgentId};

/// A unique identifier for an agent session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    /// Creates a new, randomly generated `SessionId`.
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
    /// The timestamp of when the session was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of when the session was last updated.
    pub updated_at: DateTime<Utc>,
}

impl AgentSession {
    /// Creates a new `AgentSession` for a given agent and conversation.
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
            created_at: now,
            updated_at: now,
        }
    }

    /// Transitions the session to a new state and updates the `updated_at` timestamp.
    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
        self.updated_at = Utc::now();
    }
}

/// Represents the lifecycle state of an agent session.
///
/// This enum defines a state machine that governs the behavior of an `AgentSession`
/// as it progresses through its execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn with_preview(mut self, preview: ActionPreview) -> Self {
        self.preview = Some(preview);
        self
    }
}
