//! Tool-credentials operations on the config store.
//!
//! The table `tool_credentials` stores an opaque JSON blob per tool id; this
//! module only cares about persistence / retrieval. Parsing / validating the
//! blob is the responsibility of the tool that owns it (see
//! `infrastructure/tools/search/web_search/credentials.rs` for web search).

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::ConfigRepositoryError;

/// Upserts the credentials JSON for `tool_id`. Refreshes `updated_at` on every
/// write so the Settings UI can show "last updated" timestamps.
pub fn set_tool_credentials(
    conn: &Connection,
    tool_id: &str,
    credentials: &str,
) -> Result<(), ConfigRepositoryError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO tool_credentials (tool_id, credentials, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(tool_id) DO UPDATE SET
            credentials = excluded.credentials,
            updated_at  = excluded.updated_at",
        params![tool_id, credentials, now],
    )?;
    Ok(())
}

/// Returns `Ok(None)` when no row is persisted for `tool_id`.
pub fn get_tool_credentials(
    conn: &Connection,
    tool_id: &str,
) -> Result<Option<String>, ConfigRepositoryError> {
    let credentials: Option<String> = conn
        .query_row(
            "SELECT credentials FROM tool_credentials WHERE tool_id = ?1",
            params![tool_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(credentials)
}

/// Deletes any row associated with `tool_id`. Silent no-op when the row is
/// already absent (the caller's intent is "ensure gone").
pub fn delete_tool_credentials(
    conn: &Connection,
    tool_id: &str,
) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "DELETE FROM tool_credentials WHERE tool_id = ?1",
        params![tool_id],
    )?;
    Ok(())
}

/// Returns `true` when at least one row is persisted for `tool_id`.
pub fn has_tool_credentials(
    conn: &Connection,
    tool_id: &str,
) -> Result<bool, ConfigRepositoryError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tool_credentials WHERE tool_id = ?1)",
        params![tool_id],
        |row| row.get(0),
    )?;
    Ok(exists)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::migrations::config_schema;

    fn make_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        config_schema::init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn set_then_get_roundtrips_the_credentials_blob() {
        let conn = make_conn();
        let blob = r#"{"api_key":"sk-abc","cx":"cx-123"}"#;
        set_tool_credentials(&conn, "search_web_search", blob).unwrap();
        let round_trip = get_tool_credentials(&conn, "search_web_search").unwrap();
        assert_eq!(round_trip.as_deref(), Some(blob));
    }

    #[test]
    fn get_returns_none_when_row_absent() {
        let conn = make_conn();
        assert!(get_tool_credentials(&conn, "missing").unwrap().is_none());
    }

    #[test]
    fn delete_removes_the_row() {
        let conn = make_conn();
        set_tool_credentials(&conn, "t1", "blob").unwrap();
        delete_tool_credentials(&conn, "t1").unwrap();
        assert!(get_tool_credentials(&conn, "t1").unwrap().is_none());
    }

    #[test]
    fn delete_on_missing_row_is_ok() {
        let conn = make_conn();
        delete_tool_credentials(&conn, "never-stored").unwrap();
    }

    #[test]
    fn has_reflects_presence() {
        let conn = make_conn();
        assert!(!has_tool_credentials(&conn, "t1").unwrap());
        set_tool_credentials(&conn, "t1", "{}").unwrap();
        assert!(has_tool_credentials(&conn, "t1").unwrap());
    }

    #[test]
    fn set_upserts_on_duplicate_tool_id() {
        let conn = make_conn();
        set_tool_credentials(&conn, "t1", "old").unwrap();
        set_tool_credentials(&conn, "t1", "new").unwrap();
        let stored = get_tool_credentials(&conn, "t1").unwrap();
        assert_eq!(stored.as_deref(), Some("new"));
        // There must still be exactly one row for the tool_id.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tool_credentials WHERE tool_id = ?1",
                params!["t1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn updated_at_is_refreshed_on_set() {
        let conn = make_conn();
        set_tool_credentials(&conn, "t1", "a").unwrap();
        let first: String = conn
            .query_row(
                "SELECT updated_at FROM tool_credentials WHERE tool_id = ?1",
                params!["t1"],
                |row| row.get(0),
            )
            .unwrap();

        // Sleep briefly so the RFC3339 timestamp differs. Keeps the assertion
        // robust on fast machines without introducing a real clock abstraction
        // for this one-off test.
        std::thread::sleep(std::time::Duration::from_millis(10));

        set_tool_credentials(&conn, "t1", "b").unwrap();
        let second: String = conn
            .query_row(
                "SELECT updated_at FROM tool_credentials WHERE tool_id = ?1",
                params!["t1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_ne!(first, second, "updated_at must change on upsert");
    }
}
