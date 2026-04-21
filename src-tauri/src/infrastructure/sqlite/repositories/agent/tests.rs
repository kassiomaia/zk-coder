//! Integration tests for `SqliteSessionRepository` inherent methods.
//!
//! Extracted from `mod.rs` during the Phase 6 SOLID split to keep the
//! host module under the 800-LoC hard limit.

use super::*;

fn make_session() -> crate::core::agent::session::AgentSession {
    crate::core::agent::session::AgentSession::new(
        crate::core::agent::types::AgentId::new("code-agent"),
        "Code Agent".to_string(),
        "conv-1".to_string(),
        "thread-1".to_string(),
        None,
        None,
    )
}

#[test]
fn test_session_store_integration() {
    let repository = SqliteSessionRepository::new(":memory:").unwrap();
    let session = make_session();

    repository.create_session(&session).unwrap();
    let loaded = repository.get_session(&session.id).unwrap();

    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.id, session.id);
    assert_eq!(loaded.agent_id.0, "code-agent");
    assert_eq!(loaded.conversation_id, "conv-1");
}

#[test]
fn test_task_integration() {
    let repository = SqliteSessionRepository::new(":memory:").unwrap();
    let session = make_session();
    repository.create_session(&session).unwrap();

    let task = crate::core::agent::task::AgentTask::new(
        session.id.clone(),
        crate::core::agent::task::AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        },
    );
    repository.create_task(&task, None, None).unwrap();

    let tasks = repository.get_session_tasks(&session.id).unwrap();
    assert_eq!(tasks.len(), 1);
}

#[test]
fn test_subtask_integration() {
    let repository = SqliteSessionRepository::new(":memory:").unwrap();
    let session = make_session();
    repository.create_session(&session).unwrap();

    let task = crate::core::agent::task::AgentTask::new(
        session.id.clone(),
        crate::core::agent::task::AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        },
    );
    repository.create_task(&task, None, None).unwrap();

    let subtask = crate::core::agent::task::SubTask::new(
        task.id.clone(),
        "Test Subtask".to_string(),
        crate::core::agent::task::SubTaskMode::Read,
        0,
    );
    repository.create_subtask(&subtask).unwrap();

    let subtasks = repository.get_subtasks(&task.id).unwrap();
    assert_eq!(subtasks.len(), 1);
    assert_eq!(subtasks[0].title, "Test Subtask");
}

#[test]
fn test_runtime_data_integration() {
    let repository = SqliteSessionRepository::new(":memory:").unwrap();
    let session = make_session();
    repository.create_session(&session).unwrap();

    repository
        .add_file_touched(&session.id, "/test/file.txt", "write", Some("diff"))
        .unwrap();
    repository
        .add_terminal_line(&session.id, "stdout", "output")
        .unwrap();
    repository
        .add_status_phrase(&session.id, "Working...")
        .unwrap();

    let files = repository.get_session_files(&session.id).unwrap();
    assert_eq!(files.len(), 1);

    let terminal = repository.get_session_terminal_output(&session.id).unwrap();
    assert_eq!(terminal.len(), 1);

    let phrases = repository.get_session_status_phrases(&session.id).unwrap();
    assert_eq!(phrases.len(), 1);
}

#[test]
fn test_tool_call_integration() {
    let repository = SqliteSessionRepository::new(":memory:").unwrap();
    let session = make_session();
    repository.create_session(&session).unwrap();

    repository
        .insert_tool_call(
            &session.id,
            "tool-1",
            "fs_read_file",
            &serde_json::json!({"path": "/test"}),
            None,
        )
        .unwrap();

    let tool_calls = repository.get_tool_calls_for_session(&session.id).unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].tool_name, "fs_read_file");
}

#[test]
fn test_clear_session_runtime_data_preserves_runtime_traces() {
    let repository = SqliteSessionRepository::new(":memory:").unwrap();
    let session = make_session();
    repository.create_session(&session).unwrap();
    let task = crate::core::agent::task::AgentTask::new(
        session.id.clone(),
        crate::core::agent::task::AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        },
    );
    repository.create_task(&task, None, None).unwrap();

    let trace = crate::core::agent::execution::RuntimeTrace {
        id: "trace-1".to_string(),
        session_id: session.id.0.clone(),
        task_id: task.id.clone(),
        subtask_id: None,
        step_number: 1,
        trace_type: crate::core::agent::execution::RuntimeTraceType::ExecutionStarted,
        payload_json: serde_json::json!({"requestContent": "hello"}),
        created_at: chrono::Utc::now(),
    };
    repository.append_runtime_trace(&trace).unwrap();
    repository
        .add_terminal_line(&session.id, "stdout", "temporary output")
        .unwrap();

    repository.clear_session_runtime_data(&session.id).unwrap();

    let traces = repository.get_runtime_traces(&session.id.0, None).unwrap();
    let terminal = repository.get_session_terminal_output(&session.id).unwrap();

    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].id, "trace-1");
    assert!(terminal.is_empty());
}
