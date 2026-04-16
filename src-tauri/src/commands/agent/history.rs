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
    })
}
