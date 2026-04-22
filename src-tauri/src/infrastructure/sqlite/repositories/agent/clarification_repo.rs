//! SQLite-backed [`ClarificationRepository`] +
//! [`ExecutionCheckpointRepository`] implementation.
//!
//! Shares a single `Arc<Mutex<Connection>>` with the other agent
//! domain repositories. Delegates to the free functions in the
//! `clarifications` and `execution_checkpoints` submodules.
//!
//! Introduced in the Phase 6 SOLID refactor so the pause/resume
//! contract (pending clarifications + execution checkpoints) lives on
//! a dedicated struct instead of riding on the former eight-trait god
//! object.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::core::agent::clarification::PendingClarificationRecord;
use crate::core::agent::errors::AgentDomainError;
use crate::core::agent::execution::ExecutionCheckpoint;
use crate::core::agent::repositories::{
    ClarificationRepository, ExecutionCheckpointRepository, RepositoryResult,
};

use super::{clarifications, execution_checkpoints, SessionRepositoryError};

/// SQLite-backed implementation of [`ClarificationRepository`] and
/// [`ExecutionCheckpointRepository`]. The two traits share a file
/// because both participate in the same pause/resume surface (the
/// pending-clarification row and the matching checkpoint row are
/// written in the park stage and cleared together on resume).
pub struct SqliteClarificationRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteClarificationRepository {
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

impl ClarificationRepository for SqliteClarificationRepository {
    fn upsert_pending(&self, record: &PendingClarificationRecord) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        clarifications::upsert_pending(&conn, record).map_err(map_err)
    }

    fn get_pending(
        &self,
        session_id: &str,
    ) -> RepositoryResult<Option<PendingClarificationRecord>> {
        let conn = self.lock_conn()?;
        clarifications::get_pending(&conn, session_id).map_err(map_err)
    }

    fn list_pending(&self) -> RepositoryResult<Vec<PendingClarificationRecord>> {
        let conn = self.lock_conn()?;
        clarifications::list_pending(&conn).map_err(map_err)
    }

    fn clear_pending(&self, session_id: &str) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        clarifications::clear_pending(&conn, session_id).map_err(map_err)
    }
}

impl ExecutionCheckpointRepository for SqliteClarificationRepository {
    fn save_checkpoint(&self, checkpoint: &ExecutionCheckpoint) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        execution_checkpoints::save_checkpoint(&conn, checkpoint).map_err(map_err)
    }

    fn load_checkpoint(
        &self,
        session_id: &str,
    ) -> RepositoryResult<Option<ExecutionCheckpoint>> {
        let conn = self.lock_conn()?;
        execution_checkpoints::load_checkpoint(&conn, session_id).map_err(map_err)
    }

    fn checkpoint_present(&self, session_id: &str) -> RepositoryResult<bool> {
        let conn = self.lock_conn()?;
        execution_checkpoints::checkpoint_present(&conn, session_id).map_err(map_err)
    }

    fn clear_checkpoint(&self, session_id: &str) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        execution_checkpoints::clear_checkpoint(&conn, session_id).map_err(map_err)
    }
}

fn map_err(err: SessionRepositoryError) -> AgentDomainError {
    AgentDomainError::from(err)
}
