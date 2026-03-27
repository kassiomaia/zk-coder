use tauri::AppHandle;

use crate::core::agent::entities::{AgentDomainError, SessionState as CoreSessionState};
use crate::core::agent::repositories::RepositoryResult;

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
}
