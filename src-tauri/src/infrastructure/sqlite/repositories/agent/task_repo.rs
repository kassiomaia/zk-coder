//! SQLite-backed [`TaskRepository`] implementation.
//!
//! Shares a single `Arc<Mutex<Connection>>` with the other agent
//! domain repositories (sessions, execution, clarifications, memory,
//! telemetry). Every trait method acquires the lock and delegates to
//! the free functions in the `tasks` submodule, which operate on a
//! `&Connection`.
//!
//! Introduced in the Phase 6 SOLID refactor so consumers that only
//! need task state are no longer forced to depend on `SessionRepository`,
//! `ExecutionRepository`, `MemoryRepository`, etc. through a single god
//! struct.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::core::agent::errors::AgentDomainError;
use crate::core::agent::repositories::{RepositoryResult, SubTaskRefinement, TaskRepository};
use crate::core::agent::session as agent_session;
use crate::core::agent::task as agent_task;

use super::tasks;
use super::SessionRepositoryError;

/// SQLite-backed implementation of [`TaskRepository`].
pub struct SqliteTaskRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskRepository {
    /// Builds the repository from a shared connection. The schema must
    /// already have been applied via `migrations::init_all`.
    #[must_use]
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AgentDomainError> {
        self.conn
            .lock()
            .map_err(|e| AgentDomainError::InvalidInput {
                reason: format!("Lock error: {e}"),
            })
    }
}

impl TaskRepository for SqliteTaskRepository {
    fn create_task(&self, task: &agent_task::AgentTask) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        let title = task.title.as_deref().unwrap_or("");
        let description = task
            .request
            .context
            .as_ref()
            .and_then(|ctx| ctx.selection.as_deref());
        tasks::create_task(&conn, task, Some(title), description).map_err(map_err)
    }

    fn create_subtask(&self, subtask: &agent_task::SubTask) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        tasks::create_subtask(&conn, subtask).map_err(map_err)
    }

    fn get_session_tasks(&self, session_id: &str) -> RepositoryResult<Vec<agent_task::AgentTask>> {
        let conn = self.lock_conn()?;
        let internal_id = agent_session::SessionId(session_id.to_string());
        tasks::get_session_tasks(&conn, &internal_id).map_err(map_err)
    }

    fn get_subtasks(&self, parent_task_id: &str) -> RepositoryResult<Vec<agent_task::SubTask>> {
        let conn = self.lock_conn()?;
        tasks::get_subtasks(&conn, parent_task_id).map_err(map_err)
    }

    fn update_task_status(
        &self,
        task_id: &str,
        status: agent_task::TaskStatus,
    ) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        tasks::update_task_status(&conn, task_id, &status).map_err(map_err)
    }

    fn update_task_framing(
        &self,
        task_id: &str,
        framing: Option<agent_task::TaskFraming>,
    ) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        tasks::update_task_framing(&conn, task_id, framing.as_ref()).map_err(map_err)
    }

    fn update_subtask_status(
        &self,
        subtask_id: &str,
        status: agent_task::TaskStatus,
    ) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        tasks::update_subtask_status(&conn, subtask_id, &status).map_err(map_err)
    }

    fn refine_subtask(
        &self,
        subtask_id: &str,
        refinement: SubTaskRefinement,
    ) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        tasks::refine_subtask(&conn, subtask_id, &refinement).map_err(map_err)
    }
}

fn map_err(err: SessionRepositoryError) -> AgentDomainError {
    AgentDomainError::from(err)
}
