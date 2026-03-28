use std::path::PathBuf;

use chrono::Utc;
use log::info;

use crate::core::agent::entities::{
    AgentDomainError, AgentSession as CoreAgentSession, SessionId as CoreSessionId,
    SessionState as CoreSessionState,
};
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::services::{CreateSessionRequest, SendAgentRequestRequest};
use crate::core::agent::session::{AgentSession, SessionState};

use super::AgentServiceImpl;

impl AgentServiceImpl {
    pub(super) fn create_session_impl(
        &self,
        request: CreateSessionRequest,
    ) -> RepositoryResult<CoreAgentSession> {
        info!(
            "[AgentService] Creating session for agent: {}",
            request.agent_id
        );

        let agent_id = self.to_agents_agent_id(&request.agent_id);
        let workspace_root = request.workspace_root.map(PathBuf::from);

        let session = AgentSession::new(
            agent_id,
            request.agent_id.clone(),
            request.conversation_id,
            request.thread_id,
            workspace_root,
            request.model_id,
        );

        let core_session: CoreAgentSession = session.clone().into();
        self.session_repo.create_session(&core_session)?;

        let new_state = SessionState::WaitForRequest {
            last_action_at: Utc::now(),
        };
        let core_state: crate::core::agent::entities::SessionState = new_state.into();
        self.session_repo
            .update_state(&CoreSessionId(session.id.0.clone()), &core_state)?;

        self.session_repo
            .get_session(&CoreSessionId(session.id.0.clone()))
    }

    pub(super) fn get_session_impl(&self, session_id: &str) -> RepositoryResult<CoreAgentSession> {
        self.session_repo
            .get_session(&CoreSessionId(session_id.to_string()))
    }

    pub(super) fn get_conversation_sessions_impl(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<CoreAgentSession>> {
        self.session_repo
            .get_sessions_for_conversation(conversation_id)
    }

    pub(super) fn cancel_session_impl(&self, session_id: &str) -> RepositoryResult<()> {
        let new_state = SessionState::Cancelled {
            reason: "User cancelled".to_string(),
        };
        let core_state: crate::core::agent::entities::SessionState = new_state.into();

        self.session_repo
            .update_state(&CoreSessionId(session_id.to_string()), &core_state)?;
        self.block_on(self.approval_queue.clear_session(session_id));
        self.file_snapshot_registry.clear_session(session_id);

        Ok(())
    }

    pub(super) fn delete_session_impl(&self, session_id: &str) -> RepositoryResult<()> {
        self.block_on(self.approval_queue.clear_session(session_id));
        self.block_on(self.abort_execution(session_id));
        self.session_repo
            .delete_session(&CoreSessionId(session_id.to_string()))?;
        self.file_snapshot_registry.clear_session(session_id);
        Ok(())
    }

    pub(super) fn update_session_workspace_impl(
        &self,
        session_id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<()> {
        self.session_repo
            .update_workspace(&CoreSessionId(session_id.to_string()), workspace_root)
    }

    pub(super) fn send_request_impl(
        &self,
        request: SendAgentRequestRequest,
    ) -> RepositoryResult<CoreSessionId> {
        let session = self
            .session_repo
            .get_session(&CoreSessionId(request.session_id.clone()))?;

        if self.block_on(self.is_execution_active(&request.session_id)) {
            return Err(AgentDomainError::InvalidInput {
                reason: "An agent run is already active for this session".to_string(),
            });
        }

        match session.state {
            CoreSessionState::Processing { .. } | CoreSessionState::AwaitingApproval { .. } => {
                return Err(AgentDomainError::InvalidInput {
                    reason: "This session is already processing a request".to_string(),
                });
            }
            CoreSessionState::Cancelled { .. } => {
                return Err(AgentDomainError::InvalidInput {
                    reason: "This session was cancelled and cannot accept new requests".to_string(),
                });
            }
            CoreSessionState::WaitForRequest { .. }
            | CoreSessionState::Finished { .. }
            | CoreSessionState::Error { .. }
            | CoreSessionState::Paused { .. } => {}
        }

        info!(
            "[AgentService] Request validated for session {}",
            request.session_id
        );

        Ok(CoreSessionId(request.session_id))
    }
}
