//! Provides conversion utilities between agent store types and core domain entities.
//!
//! This module contains `From` implementations and helper functions that facilitate
//! the conversion of data structures between the agent infrastructure layer and the
//! core domain layer.

use chrono::Utc;

use crate::core::agent::entities::{
    Agent as CoreAgent, AgentDomainError, AgentSession as CoreAgentSession,
    AgentTask as CoreAgentTask, AgentTrustMode, PendingAction as CorePendingAction,
    SessionId as CoreSessionId, SessionState as CoreSessionState, TaskStatus as CoreTaskStatus,
};
use crate::core::agent::session::{AgentSession, PendingAction, SessionId, SessionState};
use crate::core::agent::task::{
    AgentRequest, AgentTask, RequestContext, SubTask, SubTaskMode, TaskStatus,
};
use crate::core::agent::types::{ActionType, Agent, AgentId, TrustMode};

// =========================================================================
// Session ID Conversions
// =========================================================================

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

impl From<SessionId> for String {
    fn from(id: SessionId) -> Self {
        id.0
    }
}

impl From<&SessionId> for CoreSessionId {
    fn from(id: &SessionId) -> Self {
        Self(id.0.clone())
    }
}

impl From<SessionId> for CoreSessionId {
    fn from(id: SessionId) -> Self {
        Self(id.0)
    }
}

impl From<&CoreSessionId> for SessionId {
    fn from(id: &CoreSessionId) -> Self {
        Self(id.0.clone())
    }
}

// =========================================================================
// Agent ID Conversions
// =========================================================================

impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<AgentId> for String {
    fn from(id: AgentId) -> Self {
        id.0
    }
}

impl From<&AgentId> for crate::core::agent::entities::AgentId {
    fn from(id: &AgentId) -> Self {
        Self(id.0.clone())
    }
}

impl From<AgentId> for crate::core::agent::entities::AgentId {
    fn from(id: AgentId) -> Self {
        Self(id.0)
    }
}

// =========================================================================
// Trust Mode Conversions
// =========================================================================

impl From<TrustMode> for AgentTrustMode {
    fn from(mode: TrustMode) -> Self {
        match mode {
            TrustMode::Trusted => Self::Never,
            TrustMode::Untrusted => Self::RiskyOnly,
            TrustMode::Restricted => Self::Always,
        }
    }
}

impl From<AgentTrustMode> for TrustMode {
    fn from(mode: AgentTrustMode) -> Self {
        match mode {
            AgentTrustMode::Never => Self::Trusted,
            AgentTrustMode::RiskyOnly => Self::Untrusted,
            AgentTrustMode::Always => Self::Restricted,
        }
    }
}

impl From<SubTaskMode> for crate::core::agent::entities::TaskMode {
    fn from(mode: SubTaskMode) -> Self {
        match mode {
            SubTaskMode::Read => Self::Read,
            SubTaskMode::Write => Self::Write,
            SubTaskMode::Execute => Self::Execute,
            SubTaskMode::FullAccess => Self::FullAccess,
        }
    }
}

// =========================================================================
// Agent Entity Conversions
// =========================================================================

impl From<Agent> for CoreAgent {
    fn from(agent: Agent) -> Self {
        Self {
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
            SessionState::Launching => Self::WaitForRequest {
                last_action_at: Utc::now(),
            },
            SessionState::Running { .. } => Self::Processing {
                started_at: Utc::now(),
            },
            SessionState::WaitForRequest { last_action_at } => {
                Self::WaitForRequest { last_action_at }
            }
            SessionState::UserInputRequired { .. } => Self::AwaitingApproval {
                action_id: String::new(),
                requested_at: Utc::now(),
            },
            SessionState::Finished { completed_at } => Self::Finished {
                completed_at,
                result: String::new(),
            },
            SessionState::Cancelled { reason } => Self::Cancelled {
                cancelled_at: Utc::now(),
                reason,
            },
            SessionState::Error { message, .. } => Self::Error {
                error_at: Utc::now(),
                error: message,
            },
            SessionState::Paused { .. } => Self::WaitForRequest {
                last_action_at: Utc::now(),
            },
        }
    }
}

// =========================================================================
// Agent Session Conversions
// =========================================================================

impl From<AgentSession> for CoreAgentSession {
    fn from(session: AgentSession) -> Self {
        Self {
            id: (&session.id).into(),
            agent_id: (&session.agent_id).into(),
            agent_name: session.agent_name,
            conversation_id: session.conversation_id,
            thread_id: session.thread_id,
            workspace_root: session.workspace_root,
            model_id: session.model_id,
            state: session.state.into(),
            created_at: session.created_at,
        }
    }
}

/// Converts a core agent session into the runtime session representation used by the UI.
#[must_use]
pub fn core_session_to_agent_session(session: CoreAgentSession) -> AgentSession {
    AgentSession {
        id: SessionId(session.id.0),
        agent_id: AgentId(session.agent_id.0),
        agent_name: session.agent_name,
        conversation_id: session.conversation_id,
        thread_id: session.thread_id,
        workspace_root: session.workspace_root,
        parent_session_id: None,
        model_id: session.model_id,
        state: match session.state {
            CoreSessionState::WaitForRequest { last_action_at } => {
                SessionState::WaitForRequest { last_action_at }
            }
            CoreSessionState::Processing { .. } => SessionState::Running {
                current_step: 0,
                context_usage: 0.0,
            },
            CoreSessionState::AwaitingApproval { .. } => SessionState::UserInputRequired {
                pending_action_ids: vec![],
            },
            CoreSessionState::Finished { completed_at, .. } => {
                SessionState::Finished { completed_at }
            }
            CoreSessionState::Cancelled { reason, .. } => SessionState::Cancelled { reason },
            CoreSessionState::Error { error, .. } => SessionState::Error {
                recoverable: crate::application::agent::service::AgentServiceImpl::is_recoverable_error_message(&error),
                message: crate::application::agent::service::AgentServiceImpl::strip_recoverable_prefix(&error),
            },
            CoreSessionState::Paused { paused_at, message } => {
                // TODO: Query task repo for in-progress subtask
                let current_subtask_id = None;
                SessionState::Paused {
                    paused_at,
                    message,
                    current_subtask_id,
                }
            }
        },
        created_at: session.created_at,
        updated_at: session.created_at,
    }
}

// =========================================================================
// Pending Action Conversions
// =========================================================================

impl From<PendingAction> for CorePendingAction {
    fn from(action: PendingAction) -> Self {
        use crate::core::agent::entities::ApprovalStatus;

        Self {
            id: action.id,
            session_id: action.session_id.0,
            action_type: match &action.action_type {
                ActionType::ReadFile { path } => format!("read_file:{path}"),
                ActionType::WriteFile { path, .. } => format!("write_file:{path}"),
                ActionType::DeleteFile { path } => format!("delete_file:{path}"),
                ActionType::ExecuteCommand { command } => format!("execute:{command}"),
                ActionType::WebRequest { url } => format!("web_request:{url}"),
                ActionType::CreateDocx { filename } => format!("create_docx:{filename}"),
                ActionType::MutateDocxDraft { document_id } => format!("mutate_docx:{document_id}"),
                ActionType::InspectDocxDraft { document_id } => {
                    format!("inspect_docx:{document_id}")
                }
                ActionType::CreatePptx { filename } => format!("create_pptx:{filename}"),
                ActionType::MutatePptxDraft { deck_id } => format!("mutate_pptx:{deck_id}"),
                ActionType::InspectPptxDraft { deck_id } => format!("inspect_pptx:{deck_id}"),
                ActionType::CreatePdf { filename } => format!("create_pdf:{filename}"),
                ActionType::MutatePdfDraft { pdf_id } => format!("mutate_pdf:{pdf_id}"),
                ActionType::InspectPdfDraft { pdf_id } => format!("inspect_pdf:{pdf_id}"),
                ActionType::FinalizeDocx { document_id } => format!("finalize_docx:{document_id}"),
                ActionType::FinalizePptx { deck_id } => format!("finalize_pptx:{deck_id}"),
                ActionType::FinalizePdf { pdf_id } => format!("finalize_pdf:{pdf_id}"),
            },
            description: action.description,
            parameters: action.arguments,
            requested_at: action.created_at,
            status: ApprovalStatus::Pending,
        }
    }
}

/// Converts a core agent task into the runtime task representation used by the UI.
#[must_use]
pub fn core_task_to_runtime_task(task: CoreAgentTask) -> AgentTask {
    AgentTask {
        id: task.id,
        session_id: SessionId(task.session_id),
        request: AgentRequest {
            content: task.title,
            attachments: vec![],
            context: task.description.map(|description| RequestContext {
                file_path: None,
                selection: Some(description),
            }),
        },
        status: task.status.into(),
        steps: vec![],
        subtasks: None,
        created_at: Utc::now(),
        completed_at: None,
    }
}

// =========================================================================
// Task Status Conversions
// =========================================================================

impl From<TaskStatus> for CoreTaskStatus {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Pending => Self::Pending,
            TaskStatus::InProgress => Self::InProgress,
            TaskStatus::WaitingApproval => Self::Pending,
            TaskStatus::Completed => Self::Completed,
            TaskStatus::Failed => Self::Failed,
            TaskStatus::Cancelled => Self::Cancelled,
        }
    }
}

impl From<CoreTaskStatus> for TaskStatus {
    fn from(status: CoreTaskStatus) -> Self {
        match status {
            CoreTaskStatus::Pending => Self::Pending,
            CoreTaskStatus::InProgress => Self::InProgress,
            CoreTaskStatus::Completed => Self::Completed,
            CoreTaskStatus::Failed => Self::Failed,
            CoreTaskStatus::Cancelled => Self::Cancelled,
            CoreTaskStatus::Aborted => Self::Cancelled,
        }
    }
}

// =========================================================================
// SubTask to CoreAgentTask Conversions
// =========================================================================

impl From<SubTask> for CoreAgentTask {
    fn from(subtask: SubTask) -> Self {
        Self {
            id: subtask.id,
            session_id: String::new(),
            parent_task_id: Some(subtask.task_id),
            title: subtask.title,
            description: subtask.description,
            status: subtask.status.into(),
            order: subtask.order,
            mode: Some(subtask.mode.into()),
            needs_review: false,
        }
    }
}

/// Converts a core agent task into the public subtask representation used by IPC commands.
#[must_use]
pub fn core_task_to_subtask(task: CoreAgentTask) -> SubTask {
    use crate::core::agent::entities::TaskMode;

    SubTask {
        id: task.id,
        task_id: task.parent_task_id.unwrap_or_default(),
        title: task.title,
        description: task.description,
        mode: match task.mode.unwrap_or(TaskMode::FullAccess) {
            TaskMode::Read => SubTaskMode::Read,
            TaskMode::Write => SubTaskMode::Write,
            TaskMode::Execute => SubTaskMode::Execute,
            TaskMode::FullAccess => SubTaskMode::FullAccess,
        },
        status: task.status.into(),
        order: task.order,
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
    }
}

// =========================================================================
// Error Conversions
// =========================================================================

/// Converts any displayable error into an `AgentDomainError`.
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
        assert_eq!(
            AgentTrustMode::from(TrustMode::Trusted),
            AgentTrustMode::Never
        );
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
        assert_eq!(
            CoreTaskStatus::from(TaskStatus::Pending),
            CoreTaskStatus::Pending
        );
        assert_eq!(
            CoreTaskStatus::from(TaskStatus::WaitingApproval),
            CoreTaskStatus::Pending
        );
    }
}
