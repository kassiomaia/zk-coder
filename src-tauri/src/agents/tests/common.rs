//! Common test utilities for agent tests.

use crate::agents::{AgentId, AgentSession, SessionId};
use crate::agents::store::SessionStore;
use crate::agents::task::{AgentRequest, TaskStatus, AgentTask, SubTask, SubTaskMode};

/// Create a test session for tests.
pub fn make_test_session(store: &SessionStore) -> SessionId {
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
pub fn make_test_task(store: &SessionStore, task_id: &str, session_id: &SessionId) {
    let request = AgentRequest {
        content: "Test request".to_string(),
        attachments: vec![],
        context: None,
    };

    let task = AgentTask {
        id: task_id.to_string(),
        session_id: session_id.clone(),
        request,
        status: TaskStatus::Pending,
        steps: vec![],
        created_at: chrono::Utc::now(),
        completed_at: None,
    };

    let conn = store.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO agent_tasks (id, session_id, request, status, steps, created_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        [
            task.id,
            task.session_id.0,
            serde_json::to_string(&task.request).unwrap(),
            serde_json::to_string(&task.status).unwrap(),
            serde_json::to_string(&task.steps).unwrap(),
            task.created_at.to_rfc3339(),
            String::new(),
        ],
    ).unwrap();
}
