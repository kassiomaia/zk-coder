use chrono::Utc;
use log::{info, warn};

use crate::application::agent::harness::handoff_from_tasks;
use crate::application::agent::read_models::HarnessMode;
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::session::SessionState;
use crate::core::agent::task::TaskStatus;

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
                SessionState::Running { .. } => RECOVERY_REASON_RUNNING,
                SessionState::UserInputRequired { .. } => RECOVERY_REASON_AWAITING_APPROVAL,
                _ => continue,
            };

            // Snapshot tasks BEFORE recovery clears in-progress state so the
            // handoff summary reflects what the session was actually working
            // on when it got interrupted.
            let tasks_snapshot = self
                .task_repo
                .get_session_tasks(&session.id.0)
                .unwrap_or_default()
                .into_iter()
                .map(|mut task| {
                    let subtasks = self.task_repo.get_subtasks(&task.id).unwrap_or_default();
                    task.subtasks = Some(subtasks);
                    task
                })
                .collect::<Vec<_>>();

            if let Err(e) = self.recover_session_tasks(&session.id.0) {
                warn!(
                    "[AgentRecovery] Failed to reconcile tasks for session {}: {}",
                    session.id.0, e
                );
            }

            let now = Utc::now();
            if let Err(e) = self.session_repo.update_state(
                &session.id,
                &SessionState::Paused {
                    paused_at: now,
                    message: RECOVERY_MESSAGE.to_string(),
                    current_subtask_id: None,
                },
            ) {
                warn!(
                    "[AgentRecovery] Failed to update session state for {}: {}",
                    session.id.0, e
                );
                continue;
            }

            self.harness_state_store
                .record_mode(&session.id.0, HarnessMode::ResumeVerifier);
            self.harness_state_store.record_handoff(
                &session.id.0,
                handoff_from_tasks(&tasks_snapshot, RECOVERY_MESSAGE, now),
            );

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
            if is_non_terminal(&task.status) {
                self.task_repo
                    .update_task_status(&task.id, TaskStatus::Failed)?;
            }

            if let Ok(subtasks) = self.task_repo.get_subtasks(&task.id) {
                for subtask in subtasks {
                    if is_non_terminal(&subtask.status) {
                        self.task_repo
                            .update_subtask_status(&subtask.id, TaskStatus::Failed)?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn is_non_terminal(status: &TaskStatus) -> bool {
    matches!(status, TaskStatus::Pending | TaskStatus::InProgress)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::application::agent::harness::HarnessStateStore;
    use crate::application::agent::read_models::{
        derive_harness_state, HarnessMode, StartupVerificationState,
    };
    use crate::application::agent::AgentServiceImpl;
    use crate::application::ChatServiceImpl;
    use crate::core::agent::repositories::{SessionRepository, TaskRepository};
    use crate::core::agent::services::{AgentService, CreateSessionRequest};
    use crate::core::agent::session::SessionState;
    use crate::core::agent::task::{
        AgentRequest, AgentTask, SubTask, SubTaskMode, TaskStatus,
    };
    use crate::core::chat::services::ChatService;
    use crate::infrastructure::agent::approval::ApprovalQueue;
    use crate::infrastructure::agent::registry::AgentRegistry;
    use crate::infrastructure::agent::runtime::AgentRuntime;
    use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;
    use crate::infrastructure::sqlite::repositories::chat::SqliteChatRepository;
    use crate::infrastructure::tools::ToolRegistry;

    fn create_test_service() -> AgentServiceImpl {
        let registry = Arc::new(AgentRegistry::new());
        let session_store =
            Arc::new(SqliteSessionRepository::new(":memory:").expect("store"));
        let chat_repository =
            Arc::new(SqliteChatRepository::in_memory().expect("chat repository"));
        let chat_service = Arc::new(ChatServiceImpl::new(chat_repository)) as Arc<dyn ChatService>;
        let approval_queue = Arc::new(ApprovalQueue::new());
        let tool_registry = Arc::new(ToolRegistry::new().expect("tool registry"));
        let runtime = Arc::new(AgentRuntime::new(
            session_store.clone(),
            chat_service,
            session_store.clone(),
            session_store.clone(),
            session_store.clone(),
            session_store.clone(),
            session_store.clone(),
            approval_queue.clone(),
            tool_registry,
        ));

        AgentServiceImpl::new(
            registry,
            session_store.clone(),
            session_store.clone(),
            session_store.clone(),
            session_store.clone(),
            session_store,
            approval_queue,
            runtime,
        )
        .expect("service")
    }

    fn seed_running_session_with_tasks(service: &AgentServiceImpl) -> String {
        let mut session = service
            .create_session(CreateSessionRequest {
                agent_id: "code-agent".to_string(),
                conversation_id: "conv-recovery".to_string(),
                thread_id: "thread-recovery".to_string(),
                workspace_root: Some("/workspace".to_string()),
                model_id: None,
            })
            .expect("create session");
        let session_id = session.id.0.clone();

        let mut task = AgentTask::new(
            session.id.clone(),
            AgentRequest {
                content: "Audit auth".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        task.id = "task-root".to_string();
        task.title = Some("Audit auth".to_string());
        task.status = TaskStatus::InProgress;
        service.task_repo.create_task(&task).expect("task");

        let completed = SubTask::new(
            task.id.clone(),
            "Investigate middleware".to_string(),
            SubTaskMode::FullAccess,
            0,
        );
        let in_progress = SubTask::new(
            task.id.clone(),
            "Fix session storage".to_string(),
            SubTaskMode::FullAccess,
            1,
        );
        service.task_repo.create_subtask(&completed).expect("sub1");
        service.task_repo.create_subtask(&in_progress).expect("sub2");
        service
            .task_repo
            .update_subtask_status(&completed.id, TaskStatus::Completed)
            .expect("st1");
        service
            .task_repo
            .update_subtask_status(&in_progress.id, TaskStatus::InProgress)
            .expect("st2");

        session
            .set_state(SessionState::Running {
                current_step: 2,
                context_usage: 0.15,
            })
            .expect("running transition");
        service
            .session_repo
            .update_state(&session.id, &session.state)
            .expect("persist running");

        session_id
    }

    #[test]
    fn recovery_records_handoff_from_tasks_and_marks_mode() {
        let service = create_test_service();
        let session_id = seed_running_session_with_tasks(&service);

        let recovered = service
            .recover_unterminated_executions()
            .expect("recovery");
        assert_eq!(recovered.len(), 1, "the running session should be recovered");

        let overrides = service.harness_state_store.get(&session_id);
        assert_eq!(overrides.mode, Some(HarnessMode::ResumeVerifier));

        let handoff = overrides.handoff.expect("handoff summary recorded");
        assert_eq!(handoff.current_objective, "Fix session storage");
        assert_eq!(handoff.last_completed_increment, "Investigate middleware");
        assert_eq!(handoff.open_issues, vec![super::RECOVERY_MESSAGE.to_string()]);
    }

    #[test]
    fn recovered_handoff_flows_through_derive_harness_state() {
        let service = create_test_service();
        let session_id = seed_running_session_with_tasks(&service);
        service
            .recover_unterminated_executions()
            .expect("recovery");

        // Re-read the session (now Paused) and the task snapshot, then derive
        // with the recorded overrides — mirrors the path used by
        // load_session_runtime_details.
        let session = service
            .session_repo
            .get_session(&crate::core::agent::session::SessionId(session_id.clone()))
            .expect("session");
        let tasks = service
            .task_repo
            .get_session_tasks(&session_id)
            .unwrap_or_default();
        let overrides = service.harness_state_store.get(&session_id);
        let harness_state = derive_harness_state(&session, &tasks, Some(&overrides));

        assert_eq!(harness_state.mode, HarnessMode::ResumeVerifier);
        // The handoff is now the recorded one, not the derived default — the
        // recorded summary carries the real in-progress subtask title.
        let handoff = harness_state.handoff.expect("handoff");
        assert_eq!(handoff.current_objective, "Fix session storage");
        assert_eq!(handoff.last_completed_increment, "Investigate middleware");
        // The derived defaults for a paused session leave `startup_verification`
        // at Idle; the writer slice does not override it, so the derived value
        // stands.
        assert_eq!(
            harness_state
                .startup_verification
                .as_ref()
                .map(|v| &v.state),
            Some(&StartupVerificationState::Idle)
        );
    }
}
