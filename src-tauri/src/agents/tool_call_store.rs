//! Tool call persistence for agent sessions.
//!
//! Extends `SessionStore` with methods to persist and query tool calls
//! made during agent execution.

use chrono::Utc;
use rusqlite::params;

use super::session::SessionId;
use super::store::{SessionStore, SessionStoreError};

/// A persisted tool call record.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolCallEntry {
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    #[serde(rename = "isError")]
    pub is_error: bool,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<i64>,
    pub timestamp: String,
}

impl SessionStore {
    /// Insert a new tool call record.
    pub fn insert_tool_call(
        &self,
        session_id: &SessionId,
        id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let now = Utc::now().to_rfc3339();
        let args_str = serde_json::to_string(arguments)?;

        conn.execute(
            "INSERT INTO agent_tool_calls (id, session_id, tool_name, arguments, timestamp, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, session_id.0, tool_name, args_str, now, now],
        )?;

        Ok(())
    }

    /// Update a tool call with its result.
    pub fn update_tool_call_result(
        &self,
        id: &str,
        result: &serde_json::Value,
        is_error: bool,
        duration_ms: Option<i64>,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let result_str = serde_json::to_string(result)?;

        conn.execute(
            "UPDATE agent_tool_calls SET result = ?1, is_error = ?2, duration_ms = ?3 WHERE id = ?4",
            params![result_str, is_error as i32, duration_ms, id],
        )?;

        Ok(())
    }

    /// Get all tool calls for a session, ordered by timestamp.
    pub fn get_tool_calls_for_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<ToolCallEntry>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, tool_name, arguments, result, is_error, duration_ms, timestamp
             FROM agent_tool_calls
             WHERE session_id = ?1
             ORDER BY timestamp ASC",
        )?;

        let mut entries = Vec::new();
        let mut rows = stmt.query(params![session_id.0])?;
        while let Some(row) = rows.next()? {
            let args_str: String = row.get(3)?;
            let result_str: Option<String> = row.get(4)?;
            let is_error_int: i32 = row.get(5)?;

            entries.push(ToolCallEntry {
                id: row.get(0)?,
                session_id: row.get(1)?,
                tool_name: row.get(2)?,
                arguments: serde_json::from_str(&args_str).unwrap_or(serde_json::Value::Null),
                result: result_str
                    .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::Value::Null)),
                is_error: is_error_int != 0,
                duration_ms: row.get(6)?,
                timestamp: row.get(7)?,
            });
        }

        Ok(entries)
    }
}
