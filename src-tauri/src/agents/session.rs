//! Agent session types.
//!
//! A session is a runtime instance of an agent, tied to a specific
//! conversation and thread. It tracks execution state and pending actions.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{ActionPreview, ActionType, AgentId};

/// Unique identifier for a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
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

/// Running agent session (ephemeral, stateful).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: SessionId,
    pub agent_id: AgentId,
    pub agent_name: String,
    pub conversation_id: String,
    pub thread_id: String,
    pub state: SessionState,
    pub workspace_root: Option<PathBuf>,
    pub parent_session_id: Option<SessionId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AgentSession {
    /// Create a new session for the given agent and conversation.
    pub fn new(
        agent_id: AgentId,
        agent_name: String,
        conversation_id: String,
        thread_id: String,
        workspace_root: Option<PathBuf>,
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
            created_at: now,
            updated_at: now,
        }
    }

    /// Transition to a new state.
    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
        self.updated_at = Utc::now();
    }
}

/// Session lifecycle state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionState {
    /// Session is being initialized.
    Launching,
    /// Agent is actively executing.
    Running {
        current_step: u32,
        max_steps: u32,
    },
    /// Agent has finished a request and is waiting for the next one.
    WaitForRequest {
        last_action_at: DateTime<Utc>,
    },
    /// Agent is blocked waiting for user approval.
    UserInputRequired {
        pending_action_ids: Vec<String>,
    },
    /// Session completed successfully.
    Finished {
        completed_at: DateTime<Utc>,
    },
    /// Session was cancelled by user.
    Cancelled {
        reason: String,
    },
    /// Session encountered an error.
    Error {
        message: String,
        recoverable: bool,
    },
}

/// Pending action awaiting user approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingAction {
    pub id: String,
    pub session_id: SessionId,
    pub tool_name: String,
    pub action_type: ActionType,
    pub description: String,
    pub preview: Option<ActionPreview>,
    pub arguments: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl PendingAction {
    /// Create a new pending action from a tool call.
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

    /// Add a preview to this action.
    pub fn with_preview(mut self, preview: ActionPreview) -> Self {
        self.preview = Some(preview);
        self
    }
}
