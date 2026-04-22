//! SQLite-backed [`ExecutionRepository`] + [`RuntimeTraceRepository`]
//! implementation.
//!
//! Shares a single `Arc<Mutex<Connection>>` with the other agent
//! domain repositories. Delegates every method to the free functions
//! in the `execution_debug` submodule, which operate on a
//! `&Connection`.
//!
//! Introduced in the Phase 6 SOLID refactor so the snapshot + trace
//! contract lives on a dedicated struct instead of riding on the
//! former eight-trait god object.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::core::agent::errors::AgentDomainError;
use crate::core::agent::execution::{ExecutionSnapshot, RuntimeTrace};
use crate::core::agent::repositories::{
    ExecutionRepository, RepositoryResult, RuntimeTraceRepository,
};

use super::execution_debug;
use super::SessionRepositoryError;

/// SQLite-backed implementation of [`ExecutionRepository`] and
/// [`RuntimeTraceRepository`]. The two traits share a file because
/// both wrap rows in the same `execution_debug` table family.
pub struct SqliteExecutionRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteExecutionRepository {
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

impl ExecutionRepository for SqliteExecutionRepository {
    fn save_execution_snapshot(&self, snapshot: &ExecutionSnapshot) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        execution_debug::save_execution_snapshot(&conn, snapshot).map_err(map_err)
    }

    fn get_execution_snapshot(
        &self,
        session_id: &str,
    ) -> RepositoryResult<Option<ExecutionSnapshot>> {
        let conn = self.lock_conn()?;
        execution_debug::get_execution_snapshot(&conn, session_id).map_err(map_err)
    }

    fn clear_execution_snapshot(&self, session_id: &str) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        execution_debug::clear_execution_snapshot(&conn, session_id).map_err(map_err)
    }
}

impl RuntimeTraceRepository for SqliteExecutionRepository {
    fn append_runtime_trace(&self, trace: &RuntimeTrace) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        execution_debug::append_runtime_trace(&conn, trace).map_err(map_err)
    }

    fn get_runtime_traces(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> RepositoryResult<Vec<RuntimeTrace>> {
        let conn = self.lock_conn()?;
        execution_debug::get_runtime_traces(&conn, session_id, limit).map_err(map_err)
    }
}

fn map_err(err: SessionRepositoryError) -> AgentDomainError {
    AgentDomainError::from(err)
}
