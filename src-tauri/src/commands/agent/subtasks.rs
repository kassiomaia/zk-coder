use crate::application::agent::conversions::core_task_to_subtask;
use crate::core::agent::entities::TaskStatus as CoreTaskStatus;
use crate::core::agent::services::AgentService;

use super::AgentState;

/// Create a new subtask for decomposition.
#[tauri::command]
pub async fn create_subtask(
    task_id: String,
    title: String,
    description: Option<String>,
    order: i32,
    state: tauri::State<'_, AgentState>,
) -> Result<String, String> {
    state
        .service
        .create_subtask(task_id, title, description, order)
        .map_err(|e| e.to_string())
}

/// Update subtask status.
#[tauri::command]
pub async fn update_subtask_status(
    subtask_id: String,
    status: String,
    state: tauri::State<'_, AgentState>,
) -> Result<(), String> {
    let task_status: CoreTaskStatus =
        serde_json::from_str(&status).map_err(|e| format!("Invalid status: {e}"))?;

    state
        .service
        .update_subtask_status(subtask_id, task_status)
        .map_err(|e| e.to_string())
}

/// Get all subtasks for a task.
#[tauri::command]
pub async fn get_subtasks(
    task_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<crate::core::agent::task::SubTask>, String> {
    state
        .service
        .get_subtasks(&task_id)
        .map(|tasks| tasks.into_iter().map(core_task_to_subtask).collect())
        .map_err(|e| e.to_string())
}
