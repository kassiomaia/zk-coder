use crate::core::agent::services::AgentService;

use super::AgentState;

/// Get all file snapshots tracked for a session.
#[tauri::command]
pub async fn get_session_file_snapshots(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<crate::infrastructure::agent::file_snapshot_registry::FileSnapshot>, String> {
    Ok(state.service.runtime_session_snapshots(&session_id))
}

/// Roll back a single file to its pre-agent state.
#[tauri::command]
pub async fn rollback_file(
    session_id: String,
    path: String,
    state: tauri::State<'_, AgentState>,
) -> Result<bool, String> {
    state
        .service
        .rollback_file(&session_id, &path)
        .map_err(|e| e.to_string())
}

/// Roll back all files modified in a session to their pre-agent state.
#[tauri::command]
pub async fn rollback_session(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<String>, String> {
    state
        .service
        .rollback_session(&session_id)
        .map_err(|e| e.to_string())
}
