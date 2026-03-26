//! Conversion utilities between agent store types and core domain entities.
//!
//! This module provides `From` implementations and helper functions
//! for converting between the agent infrastructure layer and the core domain layer.

use chrono::Utc;

use crate::agents::session::{AgentSession, PendingAction, SessionId, SessionState};
use crate::agents::task::{SubTask, TaskStatus};
use crate::agents::types::{Agent, AgentId, ActionType, TrustMode};
use crate::core::agent::entities::{
    Agent as CoreAgent, AgentDomainError, AgentSession as CoreAgentSession,
    AgentTask as CoreAgentTask, AgentTrustMode, PendingAction as CorePendingAction,
    SessionId as CoreSessionId, SessionState as CoreSessionState, TaskStatus as CoreTaskStatus,
};

// =========================================================================
// Session ID Conversions
// =========================================================================

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        SessionId(s.to_string())
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        SessionId(s)
    }
}

impl From<SessionId> for String {
    fn from(id: SessionId) -> Self {
        id.0
    }
}

impl From<&SessionId> for CoreSessionId {
    fn from(id: &SessionId) -> Self {
        CoreSessionId(id.0.clone())
    }
}

impl From<SessionId> for CoreSessionId {
    fn from(id: SessionId) -> Self {
        CoreSessionId(id.0)
    }
}

impl From<&CoreSessionId> for SessionId {
    fn from(id: &CoreSessionId) -> Self {
        SessionId(id.0.clone())
    }
}

// =========================================================================
// Agent ID Conversions
// =========================================================================

impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        AgentId(s.to_string())
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        AgentId(s)
    }
}

impl From<AgentId> for String {
    fn from(id: AgentId) -> Self {
        id.0
    }
}

impl From<&AgentId> for crate::core::agent::entities::AgentId {
    fn from(id: &AgentId) -> Self {
        crate::core::agent::entities::AgentId(id.0.clone())
    }
}

impl From<AgentId> for crate::core::agent::entities::AgentId {
    fn from(id: AgentId) -> Self {
        crate::core::agent::entities::AgentId(id.0)
    }
}

// =========================================================================
// Trust Mode Conversions
// =========================================================================

impl From<TrustMode> for AgentTrustMode {
    fn from(mode: TrustMode) -> Self {
        match mode {
            TrustMode::Trusted => AgentTrustMode::Never,
            TrustMode::Untrusted => AgentTrustMode::RiskyOnly,
            TrustMode::Restricted => AgentTrustMode::Always,
        }
    }
}

impl From<AgentTrustMode> for TrustMode {
    fn from(mode: AgentTrustMode) -> Self {
        match mode {
            AgentTrustMode::Never => TrustMode::Trusted,
            AgentTrustMode::RiskyOnly => TrustMode::Untrusted,
            AgentTrustMode::Always => TrustMode::Restricted,
        }
    }
}

// =========================================================================
// Agent Entity Conversions
// =========================================================================

impl From<Agent> for CoreAgent {
    fn from(agent: Agent) -> Self {
        CoreAgent {
            id: crate::core::agent::entities::AgentId(agent.id.0),
            name: agent.name,
            description: agent.description,
            system_prompt: agent.system_prompt,
            trust_mode: agent.trust_mode.into(),
        }
    }
}

// =========================================================================
// Session State Conversions
// =========================================================================

impl From<SessionState> for CoreSessionState {
    fn from(state: SessionState) -> Self {
        match state {
            SessionState::Launching => CoreSessionState::WaitForRequest {
                last_action_at: Utc::now(),
            },
            SessionState::Running { .. } => CoreSessionState::Processing {
                started_at: Utc::now(),
            },
            SessionState::WaitForRequest { last_action_at } => {
                CoreSessionState::WaitForRequest { last_action_at }
            }
            SessionState::UserInputRequired { .. } => CoreSessionState::AwaitingApproval {
                action_id: String::new(),
                requested_at: Utc::now(),
            },
            SessionState::Finished { completed_at } => CoreSessionState::Finished {
                completed_at,
                result: String::new(),
            },
            SessionState::Cancelled { reason } => CoreSessionState::Cancelled {
                cancelled_at: Utc::now(),
                reason,
            },
            SessionState::Error { message, .. } => CoreSessionState::Error {
                error_at: Utc::now(),
                error: message,
            },
        }
    }
}

// =========================================================================
// Agent Session Conversions
// =========================================================================

impl From<AgentSession> for CoreAgentSession {
    fn from(session: AgentSession) -> Self {
        CoreAgentSession {
            id: (&session.id).into(),
            agent_id: (&session.agent_id).into(),
            agent_name: session.agent_name,
            conversation_id: session.conversation_id,
            thread_id: session.thread_id,
            workspace_root: session.workspace_root,
            state: session.state.into(),
            created_at: session.created_at,
        }
    }
}

// =========================================================================
// Pending Action Conversions
// =========================================================================

impl From<PendingAction> for CorePendingAction {
    fn from(action: PendingAction) -> Self {
        use crate::core::agent::entities::ApprovalStatus;

        CorePendingAction {
            id: action.id,
            session_id: action.session_id.0,
            action_type: match &action.action_type {
                ActionType::ReadFile { path } => format!("read_file:{}", path),
                ActionType::WriteFile { path, .. } => format!("write_file:{}", path),
                ActionType::DeleteFile { path } => format!("delete_file:{}", path),
                ActionType::ExecuteCommand { command } => format!("execute:{}", command),
                ActionType::WebRequest { url } => format!("web_request:{}", url),
            },
            description: action.description,
            parameters: action.arguments,
            requested_at: action.created_at,
            status: ApprovalStatus::Pending,
        }
    }
}

// =========================================================================
// Task Status Conversions
// =========================================================================

impl From<TaskStatus> for CoreTaskStatus {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Pending => CoreTaskStatus::Pending,
            TaskStatus::InProgress => CoreTaskStatus::InProgress,
            TaskStatus::WaitingApproval => CoreTaskStatus::Pending,
            TaskStatus::Completed => CoreTaskStatus::Completed,
            TaskStatus::Failed => CoreTaskStatus::Failed,
            TaskStatus::Cancelled => CoreTaskStatus::Cancelled,
        }
    }
}

impl From<CoreTaskStatus> for TaskStatus {
    fn from(status: CoreTaskStatus) -> Self {
        match status {
            CoreTaskStatus::Pending => TaskStatus::Pending,
            CoreTaskStatus::InProgress => TaskStatus::InProgress,
            CoreTaskStatus::Completed => TaskStatus::Completed,
            CoreTaskStatus::Failed => TaskStatus::Failed,
            CoreTaskStatus::Cancelled => TaskStatus::Cancelled,
        }
    }
}

// =========================================================================
// SubTask to CoreAgentTask Conversions
// =========================================================================

impl From<SubTask> for CoreAgentTask {
    fn from(subtask: SubTask) -> Self {
        CoreAgentTask {
            id: subtask.id,
            session_id: String::new(), // Subtasks don't have session_id directly
            title: subtask.title,
            description: subtask.description,
            status: subtask.status.into(),
            order: subtask.order,
        }
    }
}

// =========================================================================
// Error Conversions
// =========================================================================

/// Convert any displayable error to an AgentDomainError.
pub fn store_error_to_domain<E: std::fmt::Display>(err: E) -> AgentDomainError {
    AgentDomainError::InvalidInput {
        reason: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_conversion() {
        let sid = SessionId::from("test-session");
        assert_eq!(sid.0, "test-session");

        let core_id: CoreSessionId = (&sid).into();
        assert_eq!(core_id.0, "test-session");
    }

    #[test]
    fn test_agent_id_conversion() {
        let aid = AgentId::from("test-agent");
        assert_eq!(aid.0, "test-agent");

        let core_id: crate::core::agent::entities::AgentId = (&aid).into();
        assert_eq!(core_id.0, "test-agent");
    }

    #[test]
    fn test_trust_mode_conversion() {
        assert_eq!(AgentTrustMode::from(TrustMode::Trusted), AgentTrustMode::Never);
        assert_eq!(
            AgentTrustMode::from(TrustMode::Untrusted),
            AgentTrustMode::RiskyOnly
        );
        assert_eq!(
            AgentTrustMode::from(TrustMode::Restricted),
            AgentTrustMode::Always
        );
    }

    #[test]
    fn test_task_status_conversion() {
        assert_eq!(CoreTaskStatus::from(TaskStatus::Pending), CoreTaskStatus::Pending);
        assert_eq!(
            CoreTaskStatus::from(TaskStatus::WaitingApproval),
            CoreTaskStatus::Pending
        );
    }
}
