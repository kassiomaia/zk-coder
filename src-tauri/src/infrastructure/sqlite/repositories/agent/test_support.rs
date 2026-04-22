//! Test-only helper that bundles every domain-scoped agent repository
//! over a single shared connection.
//!
//! Before the Phase 6 ISP split, tests could call any repository trait
//! method directly on `SqliteSessionRepository`. After the split each
//! trait lives on a dedicated struct (`SqliteTaskRepository`,
//! `SqliteMemoryRepository`, …), so tests that need more than one must
//! build them from the same `Arc<Mutex<Connection>>`. This bundle
//! wraps that plumbing so each test keeps one line of setup.

use std::sync::Arc;

use super::{
    SessionRepositoryError, SqliteAgentTelemetryRepository, SqliteClarificationRepository,
    SqliteExecutionRepository, SqliteMemoryRepository, SqliteSessionRepository,
    SqliteTaskRepository,
};

/// Every agent repository over a single shared in-memory connection.
///
/// Each field is wrapped in `Arc` so tests can hand out trait objects
/// (`Arc<dyn TaskRepository>` etc.) without extra gymnastics.
pub struct TestAgentRepos {
    /// Session CRUD + `SessionRepository` trait object.
    pub session: Arc<SqliteSessionRepository>,
    /// `TaskRepository` trait object.
    pub task: Arc<SqliteTaskRepository>,
    /// `MemoryRepository` trait object.
    pub memory: Arc<SqliteMemoryRepository>,
    /// Combined `ExecutionRepository` + `RuntimeTraceRepository`.
    pub execution: Arc<SqliteExecutionRepository>,
    /// Combined `ClarificationRepository` + `ExecutionCheckpointRepository`.
    pub clarification: Arc<SqliteClarificationRepository>,
    /// `AgentTelemetryRepository` trait object.
    pub telemetry: Arc<SqliteAgentTelemetryRepository>,
}

impl TestAgentRepos {
    /// Build every repository over a fresh in-memory SQLite connection
    /// with the agent schema already applied.
    pub fn in_memory() -> Result<Self, SessionRepositoryError> {
        let session = SqliteSessionRepository::new(":memory:")?;
        let conn = session.conn.clone();
        Ok(Self {
            session: Arc::new(session),
            task: Arc::new(SqliteTaskRepository::from_shared(conn.clone())),
            memory: Arc::new(SqliteMemoryRepository::from_shared(conn.clone())),
            execution: Arc::new(SqliteExecutionRepository::from_shared(conn.clone())),
            clarification: Arc::new(SqliteClarificationRepository::from_shared(conn.clone())),
            telemetry: Arc::new(SqliteAgentTelemetryRepository::from_shared(conn)),
        })
    }
}
