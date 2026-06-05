//! SQL layer for `agent_execution_checkpoints`.
//!
//! Stores the JSON-serialized `ExecutionCheckpoint` keyed by session.
//! `schema_version` is replicated at the column level so a cheap query
//! can decide whether a row is loadable without parsing the payload.

use rusqlite::{params, Connection, OptionalExtension};

use super::SessionRepositoryError;
use crate::core::agent::execution::ExecutionCheckpoint;

/// Inserts or replaces the checkpoint for a session.
pub fn save_checkpoint(
    conn: &Connection,
    checkpoint: &ExecutionCheckpoint,
) -> Result<(), SessionRepositoryError> {
    let payload = serde_json::to_string(checkpoint)?;
    conn.execute(
        "INSERT INTO agent_execution_checkpoints
         (session_id, task_id, schema_version, payload, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(session_id) DO UPDATE SET
             task_id = excluded.task_id,
             schema_version = excluded.schema_version,
             payload = excluded.payload,
             updated_at = excluded.updated_at",
        params![
            checkpoint.session_id,
            checkpoint.task_id,
            checkpoint.schema_version as i64,
            payload,
            chrono::Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Returns the checkpoint for a session, if any. A row whose
/// `schema_version` exceeds the loader's understanding is surfaced as a
/// `SessionRepositoryError::Json` so the caller can degrade to a
/// recoverable-error state instead of panicking.
pub fn load_checkpoint(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<ExecutionCheckpoint>, SessionRepositoryError> {
    let row: Option<(i64, String)> = conn
        .query_row(
            "SELECT schema_version, payload FROM agent_execution_checkpoints WHERE session_id = ?1",
            params![session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some((version, payload)) => {
            if (version as u32) > ExecutionCheckpoint::CURRENT_SCHEMA_VERSION {
                // Don't try to decode a future schema — emit a
                // structured error so the caller can repair the
                // session into Error { recoverable: true }.
                return Err(SessionRepositoryError::Json(serde_json::Error::io(
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "unsupported checkpoint schema_version {version} (loader supports <= {})",
                            ExecutionCheckpoint::CURRENT_SCHEMA_VERSION
                        ),
                    ),
                )));
            }
            let checkpoint: ExecutionCheckpoint = serde_json::from_str(&payload)?;
            Ok(Some(checkpoint))
        }
    }
}

/// Returns `true` when a checkpoint row exists for the session.
pub fn checkpoint_present(
    conn: &Connection,
    session_id: &str,
) -> Result<bool, SessionRepositoryError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM agent_execution_checkpoints WHERE session_id = ?1",
        params![session_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Removes the checkpoint for a session.
pub fn clear_checkpoint(conn: &Connection, session_id: &str) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "DELETE FROM agent_execution_checkpoints WHERE session_id = ?1",
        params![session_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::execution::ExecutionPhase;
    use crate::core::llm::entities::{ChatMessage, MessageContent, MessageRole, ToolCall};

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO agent_sessions (id, agent_id, agent_name, conversation_id, thread_id, state, created_at, updated_at, needs_review)
             VALUES ('s1', 'a', 'Agent', 'c', 't', '{\"type\":\"running\",\"currentStep\":0,\"contextUsage\":0.0}', '2026-04-19T00:00:00Z', '2026-04-19T00:00:00Z', 0)",
            [],
        ).unwrap();
        conn
    }

    fn sample_checkpoint() -> ExecutionCheckpoint {
        ExecutionCheckpoint {
            schema_version: ExecutionCheckpoint::CURRENT_SCHEMA_VERSION,
            session_id: "s1".into(),
            task_id: "task-1".into(),
            model_name: "claude-opus-4-7".into(),
            step_number: 7,
            current_phase: ExecutionPhase::CallingLlm,
            current_subtask_id: Some("sub-1".into()),
            last_injected_subtask_order: Some(2),
            llm_context: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: MessageContent::Text("hi".into()),
                    tool_calls: vec![],
                    tool_result: None,
                },
                ChatMessage {
                    role: MessageRole::Assistant,
                    content: MessageContent::Text("using a tool".into()),
                    tool_calls: vec![ToolCall {
                        id: "call-1".into(),
                        name: "fs_read_file".into(),
                        arguments: serde_json::json!({"path": "/tmp/x"}),
                    }],
                    tool_result: None,
                },
            ],
            recent_tool_calls: vec![("fs_read_file".into(), 42)],
            consecutive_empty_tool_iterations: 0,
            current_allowed_tool_names: Some(vec!["fs_read_file".into()]),
            total_input_tokens: 1200,
            total_output_tokens: 300,
            context_usage_percent: 15.5,
            steps: vec![],
            max_execution_steps: 24,
        }
    }

    #[test]
    fn save_then_load_roundtrips_a_checkpoint() {
        let conn = fresh_conn();
        let checkpoint = sample_checkpoint();
        save_checkpoint(&conn, &checkpoint).unwrap();
        let loaded = load_checkpoint(&conn, "s1").unwrap().expect("present");
        // `ExecutionCheckpoint` doesn't derive `PartialEq` because its
        // `ChatMessage` and `ExecutionStep` contents don't. Compare by
        // serde-canonical JSON instead.
        let before = serde_json::to_value(&checkpoint).unwrap();
        let after = serde_json::to_value(&loaded).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn load_missing_session_returns_none() {
        let conn = fresh_conn();
        assert!(load_checkpoint(&conn, "s1").unwrap().is_none());
    }

    #[test]
    fn save_upserts_on_same_session() {
        let conn = fresh_conn();
        let mut checkpoint = sample_checkpoint();
        save_checkpoint(&conn, &checkpoint).unwrap();
        checkpoint.step_number = 99;
        save_checkpoint(&conn, &checkpoint).unwrap();
        let loaded = load_checkpoint(&conn, "s1").unwrap().unwrap();
        assert_eq!(loaded.step_number, 99);
    }

    #[test]
    fn checkpoint_present_reflects_row_existence() {
        let conn = fresh_conn();
        assert!(!checkpoint_present(&conn, "s1").unwrap());
        save_checkpoint(&conn, &sample_checkpoint()).unwrap();
        assert!(checkpoint_present(&conn, "s1").unwrap());
    }

    #[test]
    fn clear_removes_the_checkpoint() {
        let conn = fresh_conn();
        save_checkpoint(&conn, &sample_checkpoint()).unwrap();
        clear_checkpoint(&conn, "s1").unwrap();
        assert!(load_checkpoint(&conn, "s1").unwrap().is_none());
    }

    #[test]
    fn llm_context_order_survives_roundtrip() {
        let conn = fresh_conn();
        let mut checkpoint = sample_checkpoint();
        checkpoint.llm_context = vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text("one".into()),
                tool_calls: vec![],
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text("two".into()),
                tool_calls: vec![],
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::Assistant,
                content: MessageContent::Text("three".into()),
                tool_calls: vec![],
                tool_result: None,
            },
        ];
        save_checkpoint(&conn, &checkpoint).unwrap();
        let loaded = load_checkpoint(&conn, "s1").unwrap().unwrap();
        let roles: Vec<MessageRole> = loaded.llm_context.iter().map(|m| m.role.clone()).collect();
        assert_eq!(
            roles,
            vec![
                MessageRole::System,
                MessageRole::User,
                MessageRole::Assistant
            ]
        );
    }

    #[test]
    fn unknown_schema_version_rejects_cleanly() {
        let conn = fresh_conn();
        // Plant a row with a future schema_version directly.
        conn.execute(
            "INSERT INTO agent_execution_checkpoints (session_id, task_id, schema_version, payload, updated_at)
             VALUES ('s1', 'task-1', 9999, '{}', '2026-04-19T00:00:00Z')",
            [],
        )
        .unwrap();
        let err = load_checkpoint(&conn, "s1").unwrap_err();
        // Any structured error is acceptable; assert it isn't a panic
        // and that the message mentions the unsupported version.
        let message = format!("{err}");
        assert!(
            message.contains("schema_version") || message.contains("9999"),
            "expected schema-version error, got: {message}"
        );
    }

    #[test]
    fn session_deletion_cascades_to_checkpoint() {
        let conn = fresh_conn();
        save_checkpoint(&conn, &sample_checkpoint()).unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        conn.execute("DELETE FROM agent_sessions WHERE id = 's1'", [])
            .unwrap();
        assert!(load_checkpoint(&conn, "s1").unwrap().is_none());
    }
}
