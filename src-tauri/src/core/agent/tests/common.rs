//! Common test utilities for agent tests.

use crate::core::agent::entities::{AgentTask, TaskMode, TaskStatus};
use crate::core::agent::repositories::TaskRepository;
use crate::core::agent::session::{AgentSession, SessionId};
use crate::core::agent::types::AgentId;
use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;

/// Create a test session for tests.
pub fn make_test_session(store: &SqliteSessionRepository) -> SessionId {
    let session = AgentSession::new(
        AgentId::new("test-agent"),
        "Test Agent".to_string(),
        "conv-1".to_string(),
        "thread-1".to_string(),
        None,
    );
    let session_id = session.id.clone();
    store.create_session(&session).unwrap();
    session_id
}

/// Create a test task for subtask tests.
pub fn make_test_task(store: &SqliteSessionRepository, task_id: &str, session_id: &SessionId) {
    let task = AgentTask {
        id: task_id.to_string(),
        session_id: session_id.0.clone(),
        parent_task_id: None,
        title: "Test request".to_string(),
        description: None,
        status: TaskStatus::Pending,
        order: 0,
        mode: Some(TaskMode::FullAccess),
    };
    TaskRepository::create_task(store, &task).unwrap();
}
