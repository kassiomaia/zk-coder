use crate::core::agent::errors::AgentDomainError;
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::session::PendingAction;

use super::AgentServiceImpl;

impl AgentServiceImpl {
    /// Approves all pending actions for a session and returns the count.
    pub fn auto_approve_pending_for_session(&self, session_id: &str) -> RepositoryResult<usize> {
        let pending = self.get_pending_actions_impl(session_id)?;
        let mut approved = 0;

        for action in pending {
            self.approve_action_impl(&action.id)?;
            approved += 1;
        }

        Ok(approved)
    }

    /// Approves a pending action asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the action is not found.
    pub async fn approve_action_async(&self, action_id: &str) -> RepositoryResult<()> {
        self.approval_queue
            .approve(action_id)
            .await
            .ok_or_else(|| AgentDomainError::ActionNotFound {
                id: action_id.to_string(),
            })?;
        Ok(())
    }

    /// Denies a pending action asynchronously.
    ///
    /// # Errors
    ///
    /// Returns an error if the action is not found.
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
    ) -> RepositoryResult<Vec<PendingAction>> {
        Ok(self.block_on(self.approval_queue.get_pending_for_session(session_id)))
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
