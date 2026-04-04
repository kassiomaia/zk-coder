use chrono::Utc;
use log::{info, warn};

use crate::core::agent::entities::{
    SessionState as CoreSessionState, TaskStatus as CoreTaskStatus,
};
use crate::core::agent::repositories::RepositoryResult;

use super::AgentServiceImpl;

const RECOVERY_MESSAGE: &str = "Great! Your progress is saved. Pick up where you left off.";
const RECOVERY_REASON_RUNNING: &str = "startup_recovery_running_session";
const RECOVERY_REASON_AWAITING_APPROVAL: &str = "startup_recovery_awaiting_approval";

/// Information about a recovered session that can be used to emit events.
pub struct RecoveredSession {
    pub session_id: String,
    pub conversation_id: String,
    pub thread_id: String,
    pub message: String,
    pub paused_at: chrono::DateTime<chrono::Utc>,
}

impl AgentServiceImpl {
    /// Reconcile stale unterminated executions after process restart.
    ///
    /// Sessions left in `processing` or `awaiting_approval` are moved to
    /// `Paused` state so the user can choose to continue or replan.
    ///
    /// Returns a list of recovered sessions that can be used to emit events to the frontend.
    pub fn recover_unterminated_executions(&self) -> RepositoryResult<Vec<RecoveredSession>> {
        let sessions = self.session_repo.list_sessions()?;
        let mut recovered = Vec::new();

        for session in sessions {
            let reason = match &session.state {
                CoreSessionState::Processing { .. } => RECOVERY_REASON_RUNNING,
                CoreSessionState::AwaitingApproval { .. } => RECOVERY_REASON_AWAITING_APPROVAL,
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
                &CoreSessionState::Paused {
                    paused_at: now,
                    message: RECOVERY_MESSAGE.to_string(),
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

            recovered.push(RecoveredSession {
                session_id: session.id.0.clone(),
                conversation_id: session.conversation_id,
                thread_id: session.thread_id,
                message: RECOVERY_MESSAGE.to_string(),
                paused_at: now,
            });

            info!(
                "[AgentRecovery] Recovered unterminated session {} ({})",
                session.id.0, reason
            );
        }

        Ok(recovered)
    }

    fn recover_session_tasks(&self, session_id: &str) -> RepositoryResult<()> {
        let tasks = self.task_repo.get_session_tasks(session_id)?;

        for task in tasks {
            if is_non_terminal(task.status) {
                self.task_repo
                    .update_task_status(&task.id, CoreTaskStatus::Failed)?;
            }

            if let Ok(subtasks) = self.task_repo.get_subtasks(&task.id) {
                for subtask in subtasks {
                    if is_non_terminal(subtask.status) {
                        self.task_repo
                            .update_subtask_status(&subtask.id, CoreTaskStatus::Failed)?;
                    }
                }
            }
        }

        Ok(())
    }
}

const fn is_non_terminal(status: CoreTaskStatus) -> bool {
    matches!(status, CoreTaskStatus::Pending | CoreTaskStatus::InProgress)
}
