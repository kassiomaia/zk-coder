use crate::core::agent::services::AgentService;

use super::AgentState;

/// Session details response with runtime data.
#[derive(Debug, serde::Serialize)]
pub struct SessionDetailsResponse {
    /// The agent session data.
    #[serde(flatten)]
    pub session: crate::core::agent::session::AgentSession,
    /// Files touched during the session.
    #[serde(rename = "filesTouched", skip_serializing_if = "Option::is_none")]
    pub files_touched: Option<Vec<crate::application::agent::read_models::FileTouchedView>>,
    /// Terminal output lines from the session.
    #[serde(rename = "terminalOutput", skip_serializing_if = "Option::is_none")]
    pub terminal_output: Option<Vec<crate::application::agent::read_models::TerminalLineView>>,
    /// Execution steps recorded during the session.
    #[serde(rename = "executionSteps", skip_serializing_if = "Option::is_none")]
    pub execution_steps: Option<Vec<serde_json::Value>>,
    /// Status phrases reported during execution.
    #[serde(rename = "statusPhrases", skip_serializing_if = "Option::is_none")]
    pub status_phrases: Option<Vec<String>>,
    /// Tasks created during the session.
    #[serde(rename = "tasks", skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<crate::core::agent::task::AgentTask>>,
    /// Tool calls made during the session.
    #[serde(rename = "toolCalls", skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<crate::application::agent::read_models::ToolCallView>>,
    /// Long-running harness session metadata for the frontend workflow.
    #[serde(rename = "harnessState", skip_serializing_if = "Option::is_none")]
    pub harness_state: Option<crate::application::agent::read_models::AgentSessionHarnessState>,
}

/// Get completed session history for a conversation.
#[tauri::command]
pub async fn get_conversation_session_history(
    conversation_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<crate::core::agent::session::AgentSession>, String> {
    state
        .service
        .get_conversation_history(&conversation_id)
        .map_err(|e| e.to_string())
}

/// Get full session details including runtime data.
#[tauri::command]
pub async fn get_session_details(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<SessionDetailsResponse, String> {
    let details = state
        .service
        .load_session_runtime_details(&session_id)
        .map_err(|e| e.to_string())?;

    Ok(SessionDetailsResponse {
        session: details.session,
        files_touched: Some(details.files_touched),
        terminal_output: Some(details.terminal_output),
        execution_steps: Some(details.execution_steps),
        status_phrases: Some(details.status_phrases),
        tasks: Some(details.tasks),
        tool_calls: Some(details.tool_calls),
        harness_state: details.harness_state,
    })
}

#[cfg(test)]
mod tests {
    use super::SessionDetailsResponse;
    use crate::application::agent::read_models::{
        AgentSessionHarnessState, EnvironmentHealth, HarnessMode, SessionHandoffSummary,
        StartupVerificationState, StartupVerificationSummary,
    };
    use crate::core::agent::session::AgentSession;
    use crate::core::agent::types::AgentId;

    fn sample_session() -> AgentSession {
        AgentSession::new(
            AgentId("code-agent".to_string()),
            "Code Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        )
    }

    fn sample_harness_state() -> AgentSessionHarnessState {
        AgentSessionHarnessState {
            mode: HarnessMode::Worker,
            startup_verification: Some(StartupVerificationSummary {
                state: StartupVerificationState::Passed,
                headline: "Workspace check passed".to_string(),
                details: Some(vec!["Git status clean".to_string()]),
                checked_at: Some("2026-04-17T12:00:00Z".to_string()),
            }),
            handoff: Some(SessionHandoffSummary {
                last_completed_increment: "Stabilized tool catalog ordering".to_string(),
                current_objective: "Add typed harness state".to_string(),
                next_recommended_action: "Update frontend store mapping".to_string(),
                open_issues: vec![],
                environment_health: EnvironmentHealth::Healthy,
                recorded_at: Some("2026-04-17T12:05:00Z".to_string()),
            }),
            clean_exit_recorded: false,
        }
    }

    #[test]
    fn session_details_serializes_harness_state_when_present() {
        let response = SessionDetailsResponse {
            session: sample_session(),
            files_touched: Some(vec![]),
            terminal_output: Some(vec![]),
            execution_steps: Some(vec![]),
            status_phrases: Some(vec![]),
            tasks: Some(vec![]),
            tool_calls: Some(vec![]),
            harness_state: Some(sample_harness_state()),
        };

        let value = serde_json::to_value(response).expect("response should serialize");
        let harness = value
            .get("harnessState")
            .and_then(serde_json::Value::as_object)
            .expect("harnessState should be present");

        assert_eq!(
            harness.get("mode").and_then(serde_json::Value::as_str),
            Some("worker")
        );
        assert_eq!(
            harness
                .get("startupVerification")
                .and_then(serde_json::Value::as_object)
                .and_then(|summary| summary.get("headline"))
                .and_then(serde_json::Value::as_str),
            Some("Workspace check passed")
        );
    }

    #[test]
    fn session_details_omits_harness_state_when_absent() {
        let response = SessionDetailsResponse {
            session: sample_session(),
            files_touched: Some(vec![]),
            terminal_output: Some(vec![]),
            execution_steps: Some(vec![]),
            status_phrases: Some(vec![]),
            tasks: Some(vec![]),
            tool_calls: Some(vec![]),
            harness_state: None,
        };

        let value = serde_json::to_value(response).expect("response should serialize");
        assert!(value.get("harnessState").is_none());
    }
}
