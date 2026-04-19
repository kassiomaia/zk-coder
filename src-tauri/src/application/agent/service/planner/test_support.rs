//! Shared test fixtures for the planner submodules.

use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};

use super::{SubtaskPlan, TaskPlan};

pub(super) fn create_test_task(title: &str) -> crate::core::agent::task::AgentTask {
    use crate::core::agent::session::SessionId;
    use crate::core::agent::task::{AgentRequest, AgentTask, RequestContext};
    let mut task = AgentTask::new(
        SessionId("session-1".to_string()),
        AgentRequest {
            content: title.to_string(),
            attachments: vec![],
            context: Some(RequestContext {
                file_path: None,
                selection: Some("Test task".to_string()),
            }),
        },
    );
    task.id = "task-1".to_string();
    task.title = Some(title.to_string());
    task.status = TaskStatus::InProgress;
    task
}

pub(super) fn create_test_subtask(title: &str, order: i32) -> SubTask {
    let mut subtask = SubTask::new(
        "task-1".to_string(),
        title.to_string(),
        SubTaskMode::FullAccess,
        order,
    );
    subtask.id = format!("subtask-{}", order);
    subtask.status = TaskStatus::Completed;
    subtask
}

pub(super) fn build_plan_from_task(
    task: &crate::core::agent::task::AgentTask,
    subtasks: &[SubTask],
) -> TaskPlan {
    TaskPlan {
        title: task.title.clone().unwrap_or_default(),
        description: task
            .request
            .context
            .as_ref()
            .and_then(|ctx| ctx.selection.clone())
            .unwrap_or_default(),
        is_non_trivial: !subtasks.is_empty(),
        subtasks: subtasks
            .iter()
            .map(|st| SubtaskPlan {
                title: st.title.clone(),
                description: st.description.clone(),
                order: st.order,
            })
            .collect(),
    }
}

pub(super) fn pending_subtask(title: &str, order: i32) -> SubTask {
    let mut subtask = SubTask::new(
        "task-1".to_string(),
        title.to_string(),
        SubTaskMode::FullAccess,
        order,
    );
    subtask.id = format!("subtask-{order}");
    subtask.status = TaskStatus::Pending;
    subtask
}

pub(super) fn in_progress_subtask(title: &str, order: i32) -> SubTask {
    let mut s = pending_subtask(title, order);
    s.status = TaskStatus::InProgress;
    s
}

pub(super) fn completed_subtask(title: &str, order: i32) -> SubTask {
    let mut s = pending_subtask(title, order);
    s.status = TaskStatus::Completed;
    s
}
