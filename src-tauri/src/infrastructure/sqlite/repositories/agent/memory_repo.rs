//! SQLite-backed [`MemoryRepository`] implementation.
//!
//! Shares a single `Arc<Mutex<Connection>>` with the other agent
//! domain repositories. Delegates to the free functions in the
//! `memory` submodule.
//!
//! Introduced in the Phase 6 SOLID refactor so the session-memory
//! contract lives on a dedicated struct instead of riding on the
//! former eight-trait god object.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::core::agent::errors::AgentDomainError;
use crate::core::agent::memory::{MemoryRepository, SessionMemory};
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::session as agent_session;

use super::{memory, SessionRepositoryError};

/// SQLite-backed implementation of [`MemoryRepository`].
pub struct SqliteMemoryRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMemoryRepository {
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

impl MemoryRepository for SqliteMemoryRepository {
    fn get_memory(
        &self,
        session_id: &agent_session::SessionId,
    ) -> RepositoryResult<Option<SessionMemory>> {
        let conn = self.lock_conn()?;
        memory::get_memory(&conn, session_id).map_err(map_err)
    }

    fn upsert_memory(&self, session_memory: &SessionMemory) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        memory::upsert_memory(&conn, session_memory).map_err(map_err)
    }

    fn delete_memory(&self, session_id: &agent_session::SessionId) -> RepositoryResult<()> {
        let conn = self.lock_conn()?;
        memory::delete_memory(&conn, session_id).map_err(map_err)
    }
}

fn map_err(err: SessionRepositoryError) -> AgentDomainError {
    AgentDomainError::from(err)
}
