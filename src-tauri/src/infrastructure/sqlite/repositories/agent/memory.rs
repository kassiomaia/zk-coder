//! SQLite storage for the session working-memory artifact (Slice 5 /
//! Milestone 3.1). One row per session, keyed by session_id and cleaned
//! up by `ON DELETE CASCADE` when the parent session is removed.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value as JsonValue;

use super::SessionRepositoryError;
use crate::core::agent::memory::SessionMemory;
use crate::core::agent::session::SessionId;

/// Fetches the session's memory record, if any.
pub fn get_memory(
    conn: &Connection,
    session_id: &SessionId,
) -> Result<Option<SessionMemory>, SessionRepositoryError> {
    let row = conn
        .query_row(
            "SELECT session_id, task_id, task_goal, active_plan, \
                    completed_findings_json, open_questions_json, \
                    extracted_facts_json, created_at, updated_at \
             FROM agent_session_memory \
             WHERE session_id = ?1",
            params![session_id.0],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()?;

    match row {
        None => Ok(None),
        Some((sid, tid, goal, plan, findings, questions, facts, created, updated)) => {
            let completed_findings = decode_string_list(&findings)?;
            let open_questions = decode_string_list(&questions)?;
            let extracted_facts = decode_string_list(&facts)?;
            let created_at = parse_rfc3339(&created)?;
            let updated_at = parse_rfc3339(&updated)?;
            Ok(Some(SessionMemory {
                session_id: SessionId::from(sid),
                task_id: tid,
                task_goal: goal,
                active_plan: plan,
                completed_findings,
                open_questions,
                extracted_facts,
                created_at,
                updated_at,
            }))
        }
    }
}

/// Inserts or updates the session's memory record. The `updated_at`
/// column is bumped to the current wall clock on every write; the
/// `created_at` column is preserved on update.
pub fn upsert_memory(
    conn: &Connection,
    memory: &SessionMemory,
) -> Result<(), SessionRepositoryError> {
    let now = Utc::now().to_rfc3339();
    let completed = encode_string_list(&memory.completed_findings)?;
    let questions = encode_string_list(&memory.open_questions)?;
    let facts = encode_string_list(&memory.extracted_facts)?;

    conn.execute(
        "INSERT INTO agent_session_memory \
            (session_id, task_id, task_goal, active_plan, \
             completed_findings_json, open_questions_json, extracted_facts_json, \
             created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
         ON CONFLICT(session_id) DO UPDATE SET \
            task_id = excluded.task_id, \
            task_goal = excluded.task_goal, \
            active_plan = excluded.active_plan, \
            completed_findings_json = excluded.completed_findings_json, \
            open_questions_json = excluded.open_questions_json, \
            extracted_facts_json = excluded.extracted_facts_json, \
            updated_at = excluded.updated_at",
        params![
            memory.session_id.0,
            memory.task_id,
            memory.task_goal,
            memory.active_plan,
            completed,
            questions,
            facts,
            memory.created_at.to_rfc3339(),
            now,
        ],
    )?;

    Ok(())
}

/// Removes the memory record for a session. A no-op when no row exists.
pub fn delete_memory(
    conn: &Connection,
    session_id: &SessionId,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "DELETE FROM agent_session_memory WHERE session_id = ?1",
        params![session_id.0],
    )?;
    Ok(())
}

fn encode_string_list(items: &[String]) -> Result<String, SessionRepositoryError> {
    Ok(serde_json::to_string(items)?)
}

fn decode_string_list(raw: &str) -> Result<Vec<String>, SessionRepositoryError> {
    let value: JsonValue = serde_json::from_str(raw)?;
    let arr = value.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect())
}

fn parse_rfc3339(raw: &str) -> Result<DateTime<Utc>, SessionRepositoryError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            SessionRepositoryError::Json(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid rfc3339 timestamp: {e}"),
            )))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::AgentSession;
    use crate::core::agent::types::AgentId;
    use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;

    fn make_session(repo: &SqliteSessionRepository) -> AgentSession {
        let session = AgentSession::new(
            AgentId::new("code-agent"),
            "Code Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        );
        repo.create_session(&session).unwrap();
        session
    }

    fn sample(session_id: &SessionId, task_id: &str) -> SessionMemory {
        SessionMemory::new(
            session_id.clone(),
            task_id.to_string(),
            "ship the feature".to_string(),
            "step A; step B".to_string(),
        )
    }

    #[test]
    fn get_returns_none_when_no_memory_exists() {
        let repo = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session(&repo);
        let conn = repo.conn.lock().unwrap();

        let found = get_memory(&conn, &session.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn upsert_then_get_roundtrips_all_fields() {
        let repo = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session(&repo);
        let mut memory = sample(&session.id, "task-1");
        memory.completed_findings = vec!["f1".into(), "f2".into()];
        memory.open_questions = vec!["q1".into()];
        memory.extracted_facts = vec!["x1".into(), "x2".into(), "x3".into()];

        let conn = repo.conn.lock().unwrap();
        upsert_memory(&conn, &memory).unwrap();

        let loaded = get_memory(&conn, &session.id).unwrap().expect("must exist");
        assert_eq!(loaded.session_id, memory.session_id);
        assert_eq!(loaded.task_id, memory.task_id);
        assert_eq!(loaded.task_goal, memory.task_goal);
        assert_eq!(loaded.active_plan, memory.active_plan);
        assert_eq!(loaded.completed_findings, memory.completed_findings);
        assert_eq!(loaded.open_questions, memory.open_questions);
        assert_eq!(loaded.extracted_facts, memory.extracted_facts);
    }

    #[test]
    fn repeated_upsert_is_idempotent_on_body_and_bumps_updated_at() {
        let repo = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session(&repo);
        let memory = sample(&session.id, "task-1");

        let conn = repo.conn.lock().unwrap();
        upsert_memory(&conn, &memory).unwrap();
        let first = get_memory(&conn, &session.id).unwrap().unwrap();
        // Ensure the wall clock can move forward before the second write.
        std::thread::sleep(std::time::Duration::from_millis(5));
        upsert_memory(&conn, &memory).unwrap();
        let second = get_memory(&conn, &session.id).unwrap().unwrap();

        // Body must match exactly — idempotent on payload.
        assert_eq!(first.task_goal, second.task_goal);
        assert_eq!(first.active_plan, second.active_plan);
        assert_eq!(first.completed_findings, second.completed_findings);
        // created_at preserved; updated_at must not go backwards.
        assert_eq!(first.created_at, second.created_at);
        assert!(
            second.updated_at >= first.updated_at,
            "updated_at must not regress: first={:?} second={:?}",
            first.updated_at,
            second.updated_at
        );
    }

    #[test]
    fn delete_memory_removes_the_row() {
        let repo = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session(&repo);
        let memory = sample(&session.id, "task-1");

        let conn = repo.conn.lock().unwrap();
        upsert_memory(&conn, &memory).unwrap();
        assert!(get_memory(&conn, &session.id).unwrap().is_some());
        delete_memory(&conn, &session.id).unwrap();
        assert!(get_memory(&conn, &session.id).unwrap().is_none());
    }

    #[test]
    fn delete_memory_is_a_no_op_when_no_row_exists() {
        let repo = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session(&repo);
        let conn = repo.conn.lock().unwrap();
        // Must not error.
        delete_memory(&conn, &session.id).unwrap();
    }

    #[test]
    fn session_delete_cascades_and_removes_memory() {
        let repo = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session(&repo);
        let memory = sample(&session.id, "task-1");

        {
            let conn = repo.conn.lock().unwrap();
            upsert_memory(&conn, &memory).unwrap();
            assert!(get_memory(&conn, &session.id).unwrap().is_some());
        }

        repo.delete_session(&session.id).unwrap();

        let conn = repo.conn.lock().unwrap();
        assert!(
            get_memory(&conn, &session.id).unwrap().is_none(),
            "memory row must be removed when its session is deleted",
        );
    }

    #[test]
    fn upsert_preserves_list_ordering_across_roundtrips() {
        let repo = SqliteSessionRepository::new(":memory:").unwrap();
        let session = make_session(&repo);
        let mut memory = sample(&session.id, "task-1");
        memory.completed_findings = vec!["zeta".into(), "alpha".into(), "mu".into()];

        let conn = repo.conn.lock().unwrap();
        upsert_memory(&conn, &memory).unwrap();
        let loaded = get_memory(&conn, &session.id).unwrap().unwrap();
        assert_eq!(
            loaded.completed_findings,
            vec!["zeta", "alpha", "mu"],
            "list must not be reordered on roundtrip",
        );
    }
}
