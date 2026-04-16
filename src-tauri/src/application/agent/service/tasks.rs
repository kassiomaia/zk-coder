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
        self.task_repo.create_subtask(&subtask)?;

        Ok(subtask_id)
    }

    pub(super) fn update_subtask_status_impl(
        &self,
        subtask_id: String,
        status: TaskStatus,
    ) -> RepositoryResult<()> {
        self.task_repo.update_subtask_status(&subtask_id, status)
    }

    pub(super) fn get_subtasks_impl(&self, task_id: &str) -> RepositoryResult<Vec<SubTask>> {
        self.task_repo.get_subtasks(task_id)
    }
}
