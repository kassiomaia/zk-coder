//! Conversion functions between different session/task representations.
//!
//! This module contains utility functions for converting between:
//! - Core entities (database layer)
//! - Agent session/task types (application layer)
//! - Different enum variants and status types

use crate::core::agent::entities as core_entities;

/// Convert core session state to agents session state.
#[must_use]
pub fn session_state_to_agents(
    state: &core_entities::SessionState,
) -> crate::core::agent::session::SessionState {
    use crate::core::agent::session::SessionState as AgentSessionState;

    match state {
        core_entities::SessionState::WaitForRequest { last_action_at } => {
            AgentSessionState::WaitForRequest {
                last_action_at: *last_action_at,
            }
        }
        core_entities::SessionState::Processing { .. } => AgentSessionState::Running {
            current_step: 0,
            context_usage: 0.0,
        },
        core_entities::SessionState::AwaitingApproval { action_id, .. } => {
            AgentSessionState::UserInputRequired {
                pending_action_ids: vec![action_id.clone()],
            }
        }
        core_entities::SessionState::Finished { completed_at, .. } => AgentSessionState::Finished {
            completed_at: *completed_at,
        },
        core_entities::SessionState::Error { error, .. } => AgentSessionState::Error {
            recoverable:
                crate::application::agent::service::AgentServiceImpl::is_recoverable_error_message(
                    error,
                ),
            message: crate::application::agent::service::AgentServiceImpl::strip_recoverable_prefix(
                error,
            ),
        },
        core_entities::SessionState::Cancelled { reason, .. } => AgentSessionState::Cancelled {
            reason: reason.clone(),
        },
        core_entities::SessionState::Paused { paused_at, message } => AgentSessionState::Paused {
            paused_at: *paused_at,
            message: message.clone(),
            current_subtask_id: None,
        },
    }
}

/// Convert agents session state to core session state.
#[must_use]
pub fn session_state_to_core(
    state: &crate::core::agent::session::SessionState,
) -> core_entities::SessionState {
    use crate::core::agent::session::SessionState as AgentSessionState;

    match state {
        AgentSessionState::Launching => core_entities::SessionState::WaitForRequest {
            last_action_at: chrono::Utc::now(),
        },
        AgentSessionState::Running { .. } => core_entities::SessionState::Processing {
            started_at: chrono::Utc::now(),
        },
        AgentSessionState::WaitForRequest { last_action_at } => {
            core_entities::SessionState::WaitForRequest {
                last_action_at: *last_action_at,
            }
        }
        AgentSessionState::UserInputRequired { pending_action_ids } => {
            core_entities::SessionState::AwaitingApproval {
                action_id: pending_action_ids.first().cloned().unwrap_or_default(),
                requested_at: chrono::Utc::now(),
            }
        }
        AgentSessionState::Finished { completed_at } => core_entities::SessionState::Finished {
            completed_at: *completed_at,
            result: String::new(),
        },
        AgentSessionState::Cancelled { reason } => core_entities::SessionState::Cancelled {
            cancelled_at: chrono::Utc::now(),
            reason: reason.clone(),
        },
        AgentSessionState::Error { message, .. } => core_entities::SessionState::Error {
            error_at: chrono::Utc::now(),
            error: message.clone(),
        },
        AgentSessionState::Paused {
            paused_at, message, ..
        } => core_entities::SessionState::Paused {
            paused_at: *paused_at,
            message: message.clone(),
        },
    }
}

/// Convert agents session to core session.
#[must_use]
pub fn session_to_core(
    session: crate::core::agent::session::AgentSession,
) -> core_entities::AgentSession {
    core_entities::AgentSession {
        id: core_entities::SessionId(session.id.0),
        agent_id: core_entities::AgentId(session.agent_id.0),
        agent_name: session.agent_name,
        conversation_id: session.conversation_id,
        thread_id: session.thread_id,
        workspace_root: session.workspace_root,
        model_id: session.model_id,
        state: session_state_to_core(&session.state),
        created_at: session.created_at,
    }
}

/// Convert core session to agents session.
#[must_use]
pub fn session_from_core(
    session: &core_entities::AgentSession,
) -> crate::core::agent::session::AgentSession {
    crate::core::agent::session::AgentSession {
        id: crate::core::agent::session::SessionId(session.id.0.clone()),
        agent_id: crate::core::agent::types::AgentId(session.agent_id.0.clone()),
        agent_name: session.agent_name.clone(),
        conversation_id: session.conversation_id.clone(),
        thread_id: session.thread_id.clone(),
        state: session_state_to_agents(&session.state),
        workspace_root: session.workspace_root.clone(),
        parent_session_id: None,
        model_id: session.model_id.clone(),
        created_at: session.created_at,
        updated_at: session.created_at,
    }
}

/// Convert core task status to agents task status.
#[must_use]
pub const fn task_status_to_agents(
    status: core_entities::TaskStatus,
) -> crate::core::agent::task::TaskStatus {
    match status {
        core_entities::TaskStatus::Pending => crate::core::agent::task::TaskStatus::Pending,
        core_entities::TaskStatus::InProgress => crate::core::agent::task::TaskStatus::InProgress,
        core_entities::TaskStatus::Completed => crate::core::agent::task::TaskStatus::Completed,
        core_entities::TaskStatus::Failed => crate::core::agent::task::TaskStatus::Failed,
        core_entities::TaskStatus::Cancelled => crate::core::agent::task::TaskStatus::Cancelled,
        core_entities::TaskStatus::Aborted => crate::core::agent::task::TaskStatus::Cancelled,
    }
}

/// Convert core task mode to agents subtask mode.
#[must_use]
pub const fn task_mode_to_agents(
    mode: core_entities::TaskMode,
) -> crate::core::agent::task::SubTaskMode {
    match mode {
        core_entities::TaskMode::Read => crate::core::agent::task::SubTaskMode::Read,
        core_entities::TaskMode::Write => crate::core::agent::task::SubTaskMode::Write,
        core_entities::TaskMode::Execute => crate::core::agent::task::SubTaskMode::Execute,
        core_entities::TaskMode::FullAccess => crate::core::agent::task::SubTaskMode::FullAccess,
    }
}

/// Convert agents subtask mode to core task mode.
#[must_use]
pub const fn task_mode_to_core(
    mode: crate::core::agent::task::SubTaskMode,
) -> core_entities::TaskMode {
    match mode {
        crate::core::agent::task::SubTaskMode::Read => core_entities::TaskMode::Read,
        crate::core::agent::task::SubTaskMode::Write => core_entities::TaskMode::Write,
        crate::core::agent::task::SubTaskMode::Execute => core_entities::TaskMode::Execute,
        crate::core::agent::task::SubTaskMode::FullAccess => core_entities::TaskMode::FullAccess,
    }
}

/// Convert agents task status to core task status.
#[must_use]
pub const fn task_status_to_core(
    status: crate::core::agent::task::TaskStatus,
) -> core_entities::TaskStatus {
    match status {
        crate::core::agent::task::TaskStatus::Pending => core_entities::TaskStatus::Pending,
        crate::core::agent::task::TaskStatus::InProgress => core_entities::TaskStatus::InProgress,
        crate::core::agent::task::TaskStatus::WaitingApproval => core_entities::TaskStatus::Pending,
        crate::core::agent::task::TaskStatus::Completed => core_entities::TaskStatus::Completed,
        crate::core::agent::task::TaskStatus::Failed => core_entities::TaskStatus::Failed,
        crate::core::agent::task::TaskStatus::Cancelled => core_entities::TaskStatus::Cancelled,
    }
}

/// Convert core task to agents task.
#[must_use]
pub fn core_task_to_agents(task: &core_entities::AgentTask) -> crate::core::agent::task::AgentTask {
    crate::core::agent::task::AgentTask {
        id: task.id.clone(),
        session_id: crate::core::agent::session::SessionId(task.session_id.clone()),
        request: crate::core::agent::task::AgentRequest {
            content: task.title.clone(),
            attachments: vec![],
            context: task.description.as_ref().map(|description| {
                crate::core::agent::task::RequestContext {
                    file_path: None,
                    selection: Some(description.clone()),
                }
            }),
        },
        status: task_status_to_agents(task.status),
        steps: vec![],
        subtasks: None,
        created_at: chrono::Utc::now(),
        completed_at: None,
    }
}

/// Convert agents task to core task.
#[must_use]
pub fn agents_task_to_core(task: &crate::core::agent::task::AgentTask) -> core_entities::AgentTask {
    core_entities::AgentTask {
        id: task.id.clone(),
        session_id: task.session_id.0.clone(),
        parent_task_id: None,
        title: task.request.content.clone(),
        description: task
            .request
            .context
            .as_ref()
            .and_then(|ctx| ctx.selection.clone()),
        status: task_status_to_core(task.status.clone()),
        order: 0,
        mode: None,
        needs_review: false,
    }
}
