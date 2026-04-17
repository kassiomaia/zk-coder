use tauri::Emitter;

use super::deps::ExecutionDeps;
use crate::application::agent::read_models::derive_harness_state;
use crate::core::agent::session::SessionId;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::AgentRuntimeError;

/// Recompute and emit additive harness state for the current session.
pub fn emit_harness_state_updated(
    deps: &ExecutionDeps<'_>,
    session_id: &SessionId,
) -> Result<(), AgentRuntimeError> {
    let session = deps
        .session_repo
        .get_session(session_id)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    let tasks = deps
        .task_repo
        .get_session_tasks(&session_id.0)
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?
        .into_iter()
        .map(|mut task| {
            let subtasks = deps.task_repo.get_subtasks(&task.id).unwrap_or_default();
            task.subtasks = Some(subtasks);
            task
        })
        .collect::<Vec<_>>();

    let harness_state = derive_harness_state(&session, &tasks);
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::HarnessStateUpdated {
            session_id: session_id.0.clone(),
            harness_state,
        },
    );

    Ok(())
}
