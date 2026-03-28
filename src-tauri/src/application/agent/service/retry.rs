use log::info;
use tauri::{AppHandle, Emitter};

use crate::core::agent::entities::{AgentDomainError, SessionState as CoreSessionState};
use crate::core::agent::repositories::RepositoryResult;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};

use super::AgentServiceImpl;

impl AgentServiceImpl {
    /// Retry execution using the latest persisted request content for a session.
    pub async fn retry_last_request(
        &self,
        session_id: String,
        app_handle: AppHandle,
    ) -> RepositoryResult<String> {
        let sid = crate::core::agent::entities::SessionId(session_id.clone());
        let session = self.session_repo.get_session(&sid)?;

        if matches!(
            session.state,
            CoreSessionState::Processing { .. } | CoreSessionState::AwaitingApproval { .. }
        ) {
            return Err(AgentDomainError::InvalidInput {
                reason: "Cannot retry while session is still processing".to_string(),
            });
        }

        let content = self.latest_request_content(&session_id)?;
        self.spawn_execution(session_id, content, app_handle).await
    }

    fn latest_request_content(&self, session_id: &str) -> RepositoryResult<String> {
        let tasks = self.task_repo.get_session_tasks(session_id)?;
        let latest = tasks
            .first()
            .ok_or_else(|| AgentDomainError::InvalidInput {
                reason: "No previous request available to retry".to_string(),
            })?;

        let content = latest
            .description
            .clone()
            .unwrap_or_else(|| latest.title.clone())
            .trim()
            .to_string();

        if content.is_empty() {
            return Err(AgentDomainError::InvalidInput {
                reason: "Last request content is empty; unable to retry".to_string(),
            });
        }

        Ok(content)
    }

    pub fn is_recoverable_error_message(message: &str) -> bool {
        message.starts_with("[recoverable]")
    }

    pub fn strip_recoverable_prefix(message: &str) -> String {
        message
            .strip_prefix("[recoverable]")
            .map(str::trim_start)
            .unwrap_or(message)
            .to_string()
    }

    /// Continue execution from a paused session.
    ///
    /// Resumes the agent with the original request, preserving all context.
    pub async fn continue_execution(
        &self,
        session_id: String,
        app_handle: AppHandle,
    ) -> RepositoryResult<String> {
        let sid = crate::core::agent::entities::SessionId(session_id.clone());
        let session = self.session_repo.get_session(&sid)?;

        // Verify session is in Paused state
        if !matches!(session.state, CoreSessionState::Paused { .. }) {
            return Err(AgentDomainError::InvalidInput {
                reason: "Can only continue from a paused session".to_string(),
            });
        }

        let content = self.latest_request_content(&session_id)?;

        info!("[AgentRecovery] Continuing execution for session {}", session_id);

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::RecoveryContinue {
                session_id: session_id.clone(),
            },
        );

        self.spawn_execution(session_id, content, app_handle).await
    }

    /// Replan and continue execution from a paused session.
    ///
    /// The agent will review all previous work and create a fresh plan.
    pub async fn replan_and_continue(
        &self,
        session_id: String,
        app_handle: AppHandle,
    ) -> RepositoryResult<String> {
        let sid = crate::core::agent::entities::SessionId(session_id.clone());
        let session = self.session_repo.get_session(&sid)?;

        // Verify session is in Paused state
        if !matches!(session.state, CoreSessionState::Paused { .. }) {
            return Err(AgentDomainError::InvalidInput {
                reason: "Can only replan from a paused session".to_string(),
            });
        }

        let content = self.latest_request_content(&session_id)?;

        // Prepend replan instruction
        let replan_content = format!(
            "{}\n\nPlease review all the work that has been done so far and create a fresh plan for completing the remaining tasks.",
            content
        );

        info!("[AgentRecovery] Replanning and continuing execution for session {}", session_id);

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::RecoveryReplan {
                session_id: session_id.clone(),
            },
        );

        self.spawn_execution(session_id, replan_content, app_handle).await
    }
}
