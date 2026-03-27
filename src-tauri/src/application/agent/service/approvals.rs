use crate::core::agent::entities::{AgentDomainError, PendingAction as CorePendingAction};
use crate::core::agent::repositories::RepositoryResult;

use super::AgentServiceImpl;

impl AgentServiceImpl {
    pub async fn approve_action_async(&self, action_id: &str) -> RepositoryResult<()> {
        self.approval_queue
            .approve(action_id)
            .await
            .ok_or_else(|| AgentDomainError::ActionNotFound {
                id: action_id.to_string(),
            })?;
        Ok(())
    }

    pub async fn deny_action_async(&self, action_id: &str) -> RepositoryResult<()> {
        let denied = self.approval_queue.deny(action_id).await;
        if !denied {
            return Err(AgentDomainError::ActionNotFound {
                id: action_id.to_string(),
            });
        }
        Ok(())
    }

    pub(super) fn get_pending_actions_impl(
        &self,
        session_id: &str,
    ) -> RepositoryResult<Vec<CorePendingAction>> {
        Ok(self
            .block_on(self.approval_queue.get_pending_for_session(session_id))
            .into_iter()
            .map(crate::core::agent::entities::PendingAction::from)
            .collect())
    }

    pub(super) fn approve_action_impl(&self, action_id: &str) -> RepositoryResult<()> {
        self.block_on(self.approval_queue.approve(action_id))
            .ok_or_else(|| AgentDomainError::ActionNotFound {
                id: action_id.to_string(),
            })?;
        Ok(())
    }

    pub(super) fn deny_action_impl(&self, action_id: &str) -> RepositoryResult<()> {
        let denied = self.block_on(self.approval_queue.deny(action_id));

        if !denied {
            return Err(AgentDomainError::ActionNotFound {
                id: action_id.to_string(),
            });
        }

        Ok(())
    }
}
