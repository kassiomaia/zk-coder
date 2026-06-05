//! SQL layer for `agent_pending_clarifications`.
//!
//! Owns the CRUD helpers used by the `ClarificationRepository` trait
//! impl in `mod.rs`. The table stores the authoritative pause state
//! that `application::agent::clarification` caches on top.

use rusqlite::{params, Connection, OptionalExtension};

use super::SessionRepositoryError;
use crate::core::agent::clarification::PendingClarificationRecord;

fn parse_asked_at(raw: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

/// Inserts or replaces the pending clarification for a session.
pub fn upsert_pending(
    conn: &Connection,
    record: &PendingClarificationRecord,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "INSERT INTO agent_pending_clarifications (session_id, task_id, subtask_id, prompt, asked_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(session_id) DO UPDATE SET
             task_id = excluded.task_id,
             subtask_id = excluded.subtask_id,
             prompt = excluded.prompt,
             asked_at = excluded.asked_at",
        params![
            record.session_id,
            record.task_id,
            record.subtask_id,
            record.prompt,
            record.asked_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Returns the pending clarification for a session, if any.
pub fn get_pending(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<PendingClarificationRecord>, SessionRepositoryError> {
    let row = conn
        .query_row(
            "SELECT session_id, task_id, subtask_id, prompt, asked_at
             FROM agent_pending_clarifications WHERE session_id = ?1",
            params![session_id],
            |row| {
                let session_id: String = row.get(0)?;
                let task_id: String = row.get(1)?;
                let subtask_id: Option<String> = row.get(2)?;
                let prompt: String = row.get(3)?;
                let asked_at: String = row.get(4)?;
                Ok((session_id, task_id, subtask_id, prompt, asked_at))
            },
        )
        .optional()?;
    Ok(row.map(
        |(session_id, task_id, subtask_id, prompt, asked_at)| PendingClarificationRecord {
            session_id,
            task_id,
            subtask_id,
            prompt,
            asked_at: parse_asked_at(&asked_at),
        },
    ))
}

/// Returns every pending clarification, across all sessions.
pub fn list_pending(
    conn: &Connection,
) -> Result<Vec<PendingClarificationRecord>, SessionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT session_id, task_id, subtask_id, prompt, asked_at
         FROM agent_pending_clarifications ORDER BY asked_at ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let session_id: String = row.get(0)?;
            let task_id: String = row.get(1)?;
            let subtask_id: Option<String> = row.get(2)?;
            let prompt: String = row.get(3)?;
            let asked_at: String = row.get(4)?;
            Ok(PendingClarificationRecord {
                session_id,
                task_id,
                subtask_id,
                prompt,
                asked_at: parse_asked_at(&asked_at),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Removes the pending clarification for a session, if any.
pub fn clear_pending(conn: &Connection, session_id: &str) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "DELETE FROM agent_pending_clarifications WHERE session_id = ?1",
        params![session_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();
        // Satisfy the session FK for inserts.
        conn.execute(
            "INSERT INTO agent_sessions (id, agent_id, agent_name, conversation_id, thread_id, state, created_at, updated_at, needs_review)
             VALUES ('s1', 'a', 'Agent', 'c', 't', '{\"type\":\"running\",\"currentStep\":0,\"contextUsage\":0.0}', '2026-04-19T00:00:00Z', '2026-04-19T00:00:00Z', 0)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO agent_sessions (id, agent_id, agent_name, conversation_id, thread_id, state, created_at, updated_at, needs_review)
             VALUES ('s2', 'a', 'Agent', 'c', 't', '{\"type\":\"running\",\"currentStep\":0,\"contextUsage\":0.0}', '2026-04-19T00:00:00Z', '2026-04-19T00:00:00Z', 0)",
            [],
        ).unwrap();
        conn
    }

    fn sample(session: &str, subtask: Option<&str>) -> PendingClarificationRecord {
        PendingClarificationRecord {
            session_id: session.to_string(),
            task_id: "task-1".into(),
            subtask_id: subtask.map(str::to_string),
            prompt: format!("Q for {session}"),
            asked_at: Utc::now(),
        }
    }

    #[test]
    fn upsert_then_get_roundtrips_the_record() {
        let conn = fresh_conn();
        let record = sample("s1", Some("sub-1"));
        upsert_pending(&conn, &record).unwrap();
        let loaded = get_pending(&conn, "s1")
            .unwrap()
            .expect("pending should exist");
        assert_eq!(loaded.session_id, "s1");
        assert_eq!(loaded.task_id, "task-1");
        assert_eq!(loaded.subtask_id.as_deref(), Some("sub-1"));
        assert_eq!(loaded.prompt, "Q for s1");
    }

    #[test]
    fn upsert_replaces_existing_row_for_same_session() {
        let conn = fresh_conn();
        upsert_pending(&conn, &sample("s1", Some("sub-1"))).unwrap();
        let mut second = sample("s1", Some("sub-2"));
        second.prompt = "second question".into();
        upsert_pending(&conn, &second).unwrap();
        let loaded = get_pending(&conn, "s1").unwrap().unwrap();
        assert_eq!(loaded.subtask_id.as_deref(), Some("sub-2"));
        assert_eq!(loaded.prompt, "second question");
    }

    #[test]
    fn clear_removes_the_record() {
        let conn = fresh_conn();
        upsert_pending(&conn, &sample("s1", None)).unwrap();
        assert!(get_pending(&conn, "s1").unwrap().is_some());
        clear_pending(&conn, "s1").unwrap();
        assert!(get_pending(&conn, "s1").unwrap().is_none());
    }

    #[test]
    fn list_pending_returns_every_session() {
        let conn = fresh_conn();
        upsert_pending(&conn, &sample("s1", None)).unwrap();
        upsert_pending(&conn, &sample("s2", Some("x"))).unwrap();
        let all = list_pending(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn get_pending_returns_none_when_absent() {
        let conn = fresh_conn();
        assert!(get_pending(&conn, "s1").unwrap().is_none());
    }

    #[test]
    fn session_deletion_cascades() {
        // Required-by-plan invariant: if a session is deleted, its
        // pending clarification row must not linger.
        let conn = fresh_conn();
        upsert_pending(&conn, &sample("s1", Some("sub-1"))).unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        conn.execute("DELETE FROM agent_sessions WHERE id = 's1'", [])
            .unwrap();
        assert!(get_pending(&conn, "s1").unwrap().is_none());
    }
}
