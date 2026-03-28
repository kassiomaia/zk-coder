use chrono::Utc;
use log::{info, warn};

use crate::core::agent::entities::{
    SessionState as CoreSessionState, TaskStatus as CoreTaskStatus,
};
use crate::core::agent::repositories::RepositoryResult;

use super::AgentServiceImpl;

const RECOVERABLE_PREFIX: &str = "[recoverable]";
const RECOVERY_REASON_RUNNING: &str = "startup_recovery_running_session";
const RECOVERY_REASON_AWAITING_APPROVAL: &str = "startup_recovery_awaiting_approval";

impl AgentServiceImpl {
    /// Reconcile stale unterminated executions after process restart.
    ///
    /// Sessions left in `processing` or `awaiting_approval` are moved to
    /// recoverable error state. Any non-terminal tasks/subtasks are marked failed.
    pub fn recover_unterminated_executions(&self) -> RepositoryResult<usize> {
        let sessions = self.session_repo.list_sessions()?;
        let mut recovered_count = 0usize;

        for session in sessions {
            let (reason, message) = match &session.state {
                CoreSessionState::Processing { .. } => (
                    RECOVERY_REASON_RUNNING,
                    format!(
                        "{RECOVERABLE_PREFIX} Recovered from interrupted execution after app restart. \
                        The previous run did not finish. Use retry to continue."
                    ),
                ),
                CoreSessionState::AwaitingApproval { .. } => (
                    RECOVERY_REASON_AWAITING_APPROVAL,
                    format!(
                        "{RECOVERABLE_PREFIX} Recovered from interrupted execution while waiting for approval. \
                        Pending approval actions were lost after restart. Use retry to continue."
                    ),
                ),
                _ => continue,
            };

            if let Err(e) = self.recover_session_tasks(&session.id.0) {
                warn!(
                    "[AgentRecovery] Failed to reconcile tasks for session {}: {}",
                    session.id.0, e
                );
            }

            let now = Utc::now();
            if let Err(e) = self.session_repo.update_state(
                &session.id,
                &CoreSessionState::Error {
                    error_at: now,
                    error: message,
                },
            ) {
                warn!(
                    "[AgentRecovery] Failed to update session state for {}: {}",
                    session.id.0, e
                );
                continue;
            }

            if let Err(e) = self
                .session_repo
                .record_recovery_event(&session.id, reason, now)
            {
                warn!(
                    "[AgentRecovery] Failed to record recovery event for {}: {}",
                    session.id.0, e
                );
            }

            recovered_count += 1;
            info!(
                "[AgentRecovery] Recovered unterminated session {} ({})",
                session.id.0, reason
            );
        }

        Ok(recovered_count)
    }

    fn recover_session_tasks(&self, session_id: &str) -> RepositoryResult<()> {
        let tasks = self.task_repo.get_session_tasks(session_id)?;

        for task in tasks {
            if is_non_terminal(task.status) {
                self.task_repo
                    .update_task_status(&task.id, CoreTaskStatus::Failed)?;
            }

            // Note: We don't mark subtasks as failed anymore.
            // Subtasks that were InProgress should remain InProgress so users can see
            // what was being worked on. Only the root task is marked as Failed
            // to indicate the execution didn't complete successfully.
        }

        Ok(())
    }
}

fn is_non_terminal(status: CoreTaskStatus) -> bool {
    matches!(status, CoreTaskStatus::Pending | CoreTaskStatus::InProgress)
}
