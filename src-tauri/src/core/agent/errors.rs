//! Error types for the agent domain layer.

use super::session::SessionState;

/// Error type for agent domain operations.
#[derive(Debug, Clone)]
pub enum AgentDomainError {
    /// Session not found
    SessionNotFound {
        /// The session ID that was not found.
        id: String,
    },
    /// Agent not found
    AgentNotFound {
        /// The agent ID that was not found.
        id: String,
    },
    /// Session is in invalid state for operation
    InvalidSessionState {
        /// The invalid state.
        state: SessionState,
    },
    /// Action not found
    ActionNotFound {
        /// The action ID that was not found.
        id: String,
    },
    /// Invalid input
    InvalidInput {
        /// The reason for the invalid input.
        reason: String,
    },
}

impl std::fmt::Display for AgentDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionNotFound { id } => write!(f, "Session not found: {id}"),
            Self::AgentNotFound { id } => write!(f, "Agent not found: {id}"),
            Self::InvalidSessionState { state } => write!(f, "Invalid session state: {state:?}"),
            Self::ActionNotFound { id } => write!(f, "Action not found: {id}"),
            Self::InvalidInput { reason } => write!(f, "Invalid input: {reason}"),
        }
    }
}

impl std::error::Error for AgentDomainError {}
