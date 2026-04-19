use log::info;
use tauri::{AppHandle, Emitter};

use crate::core::agent::errors::AgentDomainError;
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::session::{SessionId, SessionState};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};

use super::planner::UserIntentHint;
use super::AgentServiceImpl;

impl AgentServiceImpl {
    /// Retry execution using the latest persisted request content for a session.
    pub async fn retry_last_request(
        &self,
        session_id: String,
        app_handle: AppHandle,
    ) -> RepositoryResult<String> {
        let sid = SessionId(session_id.clone());
        let session = self.session_repo.get_session(&sid)?;

        if matches!(
            session.state,
            SessionState::Running { .. } | SessionState::UserInputRequired { .. }
        ) {
            return Err(AgentDomainError::InvalidInput {
                reason: "Cannot retry while session is still processing".to_string(),
            });
        }

        let content = self.latest_request_content(&session_id)?;
        self.harness_state_store
            .record_pending_intent(&session_id, UserIntentHint::Fresh);
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
            .request
            .context
            .as_ref()
            .and_then(|ctx| ctx.selection.clone())
            .or_else(|| latest.title.clone())
            .unwrap_or_else(|| latest.request.content.clone())
            .trim()
            .to_string();

        if content.is_empty() {
            return Err(AgentDomainError::InvalidInput {
                reason: "Last request content is empty; unable to retry".to_string(),
            });
        }

        Ok(content)
    }

    /// Check if an error message indicates a recoverable error.
    ///
    /// Recoverable errors are prefixed with `[recoverable]`.
    #[must_use]
    pub fn is_recoverable_error_message(message: &str) -> bool {
        message.starts_with("[recoverable]")
    }

    /// Remove the `[recoverable]` prefix from an error message if present.
    pub fn strip_recoverable_prefix(message: &str) -> String {
        message
            .strip_prefix("[recoverable]")
            .map_or(message, str::trim_start)
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
        let sid = SessionId(session_id.clone());
        let session = self.session_repo.get_session(&sid)?;

        // Verify session is in Paused state
        if !matches!(session.state, SessionState::Paused { .. }) {
            return Err(AgentDomainError::InvalidInput {
                reason: "Can only continue from a paused session".to_string(),
            });
        }

        let content = self.latest_request_content(&session_id)?;

        info!("[AgentRecovery] Continuing execution for session {session_id}");

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::RecoveryContinue {
                session_id: session_id.clone(),
            },
        );

        // Tell bootstrap's planner this is a Resume — it will reuse the
        // existing plan and just seek the cursor to the first non-
        // completed subtask instead of regenerating the plan.
        self.harness_state_store
            .record_pending_intent(&session_id, UserIntentHint::Resume);
        self.spawn_execution(session_id, content, app_handle).await
    }

    /// Replan and continue execution from a paused session.
    ///
    /// The agent will review all previous work and create a fresh plan
    /// for the remaining work. The previous task row stays in history.
    pub async fn replan_and_continue(
        &self,
        session_id: String,
        app_handle: AppHandle,
    ) -> RepositoryResult<String> {
        let sid = SessionId(session_id.clone());
        let session = self.session_repo.get_session(&sid)?;

        // Verify session is in Paused state
        if !matches!(session.state, SessionState::Paused { .. }) {
            return Err(AgentDomainError::InvalidInput {
                reason: "Can only replan from a paused session".to_string(),
            });
        }

        let content = self.latest_request_content(&session_id)?;

        info!("[AgentRecovery] Replanning and continuing execution for session {session_id}");

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::RecoveryReplan {
                session_id: session_id.clone(),
            },
        );

        // The planner's system prompt, combined with UserIntentHint::Replan
        // and the existing plan state, owns the "review prior work and
        // produce a revised plan" instruction — no need to splice it
        // into the user content.
        self.harness_state_store
            .record_pending_intent(&session_id, UserIntentHint::Replan);
        self.spawn_execution(session_id, content, app_handle).await
    }
}
