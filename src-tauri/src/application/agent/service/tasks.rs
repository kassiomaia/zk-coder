use crate::application::agent::conversions::core_task_to_subtask;
use crate::core::agent::entities::AgentTask as CoreAgentTask;
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};

use super::AgentServiceImpl;

impl AgentServiceImpl {
    pub(super) fn create_subtask_impl(
        &self,
        task_id: String,
        title: String,
        description: Option<String>,
        order: i32,
    ) -> RepositoryResult<String> {
        let subtask = SubTask::new(task_id.clone(), title, SubTaskMode::FullAccess, order);
        let subtask = if let Some(description) = description {
            subtask.with_description(description)
        } else {
            subtask
        };

        let subtask_id = subtask.id.clone();
        self.task_repo.create_subtask(
            &task_id,
            &CoreAgentTask {
                id: subtask.id.clone(),
                session_id: String::new(),
                parent_task_id: Some(task_id.clone()),
                title: subtask.title.clone(),
                description: subtask.description.clone(),
                status: crate::core::agent::entities::TaskStatus::Pending,
                order: subtask.order,
                mode: Some(crate::core::agent::entities::TaskMode::FullAccess),
                needs_review: false,
            },
        )?;

        Ok(subtask_id)
    }

    pub(super) fn update_subtask_status_impl(
        &self,
        subtask_id: String,
        status: TaskStatus,
    ) -> RepositoryResult<()> {
        let core_status: crate::core::agent::entities::TaskStatus = status.into();
        self.task_repo
            .update_subtask_status(&subtask_id, core_status)
    }

    pub(super) fn get_subtasks_impl(&self, task_id: &str) -> RepositoryResult<Vec<SubTask>> {
        Ok(self
            .task_repo
            .get_subtasks(task_id)?
            .into_iter()
            .map(core_task_to_subtask)
            .collect())
    }
}
