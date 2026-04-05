//! Tool call persistence operations for the agent store.

use rusqlite::{params, Connection};

use super::SessionRepositoryError;
use crate::core::agent::session::SessionId;

/// Database entry for a tool call.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolCallEntry {
    /// Unique identifier for the tool call.
    pub id: String,
    /// Session identifier.
    pub session_id: String,
    /// Name of the tool that was called.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub arguments: serde_json::Value,
    /// Result returned by the tool, if available.
    pub result: Option<serde_json::Value>,
    /// Whether the tool call resulted in an error.
    pub is_error: bool,
    /// Duration of the tool call in milliseconds.
    pub duration_ms: Option<i64>,
    /// Optional subtask identifier if this call was made within a subtask.
    pub subtask_id: Option<String>,
    /// Timestamp when the tool call was recorded.
    pub timestamp: String,
}

/// Insert a tool call record.
pub fn insert_tool_call(
    conn: &Connection,
    session_id: &SessionId,
    tool_call_id: &str,
    tool_name: &str,
    arguments: &serde_json::Value,
    subtask_id: Option<&str>,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "INSERT INTO agent_tool_calls (id, session_id, tool_name, arguments, timestamp, created_at, subtask_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            tool_call_id,
            session_id.0,
            tool_name,
            serde_json::to_string(arguments)?,
            chrono::Utc::now().to_rfc3339(),
            chrono::Utc::now().to_rfc3339(),
            subtask_id,
        ],
    )?;
    Ok(())
}

/// Update tool call result.
pub fn update_tool_call_result(
    conn: &Connection,
    tool_call_id: &str,
    result: &serde_json::Value,
    is_error: bool,
    duration_ms: Option<i64>,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "UPDATE agent_tool_calls SET result = ?1, is_error = ?2, duration_ms = ?3 WHERE id = ?4",
        params![
            serde_json::to_string(result)?,
            i32::from(is_error),
            duration_ms,
            tool_call_id
        ],
    )?;
    Ok(())
}

/// Get all tool calls for a session.
pub fn get_tool_calls_for_session(
    conn: &Connection,
    session_id: &SessionId,
) -> Result<Vec<ToolCallEntry>, SessionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, tool_name, arguments, result, is_error, duration_ms, subtask_id, timestamp
         FROM agent_tool_calls WHERE session_id = ?1 ORDER BY timestamp",
    )?;

    let tool_calls = stmt
        .query_map(params![session_id.0], |row| {
            let arguments_str: String = row.get(3)?;
            let result_str: Option<String> = row.get(4)?;

            let arguments: serde_json::Value =
                serde_json::from_str(&arguments_str).map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    )
                })?;

            let result = result_str
                .map(|s| {
                    serde_json::from_str(&s).map_err(|e| {
                        rusqlite::Error::ToSqlConversionFailure(
                            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                        )
                    })
                })
                .transpose()?;

            Ok(ToolCallEntry {
                id: row.get(0)?,
                session_id: row.get(1)?,
                tool_name: row.get(2)?,
                arguments,
                result,
                is_error: row.get::<_, i32>(5)? != 0,
                duration_ms: row.get(6)?,
                subtask_id: row.get(7)?,
                timestamp: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(tool_calls)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::{AgentSession, SessionId};
    use crate::core::agent::task::SubTask;
    use crate::core::agent::types::AgentId;

    fn make_test_session(conn: &Connection) -> SessionId {
        let session = AgentSession::new(
            AgentId::new("test-agent"),
            "Test Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        );
        super::super::sessions::create_session(conn, &session).unwrap();
        session.id
    }

    fn make_test_task(conn: &Connection, session_id: &SessionId) -> String {
        let task = crate::core::agent::task::AgentTask::new(
            session_id.clone(),
            crate::core::agent::task::AgentRequest {
                content: "Test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        let task_id = task.id.clone();
        super::super::tasks::create_task(conn, &task, None, None).unwrap();
        task_id
    }

    #[test]
    fn test_tool_call_with_subtask_id() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task_id = make_test_task(&conn, &session_id);

        let subtask = SubTask::new(
            task_id,
            "Test Subtask".to_string(),
            crate::core::agent::task::SubTaskMode::Read,
            0,
        );
        super::super::tasks::create_subtask(&conn, &subtask).unwrap();

        let tool_call_id = "tool-call-1";
        insert_tool_call(
            &conn,
            &session_id,
            tool_call_id,
            "fs_read_file",
            &serde_json::json!({"path": "/test/file.txt"}),
            Some(&subtask.id),
        )
        .unwrap();

        let tool_calls = get_tool_calls_for_session(&conn, &session_id).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, tool_call_id);
        assert_eq!(tool_calls[0].tool_name, "fs_read_file");
        assert_eq!(tool_calls[0].subtask_id, Some(subtask.id));
        assert!(!tool_calls[0].timestamp.is_empty());
    }

    #[test]
    fn test_tool_call_without_subtask_id() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);

        let tool_call_id = "tool-call-1";
        insert_tool_call(
            &conn,
            &session_id,
            tool_call_id,
            "agent_plan_subtasks",
            &serde_json::json!({"subtasks": []}),
            None,
        )
        .unwrap();

        let tool_calls = get_tool_calls_for_session(&conn, &session_id).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].subtask_id, None);
        assert!(!tool_calls[0].timestamp.is_empty());
    }

    #[test]
    fn test_update_tool_call_result() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);

        let tool_call_id = "tool-call-1";
        insert_tool_call(
            &conn,
            &session_id,
            tool_call_id,
            "fs_read_file",
            &serde_json::json!({"path": "/test/file.txt"}),
            None,
        )
        .unwrap();

        let result = serde_json::json!({"content": "file content"});
        update_tool_call_result(&conn, tool_call_id, &result, false, Some(100)).unwrap();

        let tool_calls = get_tool_calls_for_session(&conn, &session_id).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert!(tool_calls[0].result.is_some());
        assert_eq!(tool_calls[0].result.as_ref().unwrap(), &result);
        assert_eq!(tool_calls[0].is_error, false);
        assert_eq!(tool_calls[0].duration_ms, Some(100));
    }
}
