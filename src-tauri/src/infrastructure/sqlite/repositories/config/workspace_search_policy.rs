//! Per-workspace domain policy persistence for the web search tool.
//!
//! Stored under the canonicalized workspace path (empty string = global
//! fallback). Allow/deny lists are persisted as JSON arrays of domain
//! patterns; parsing / matching is the caller's responsibility
//! (`infrastructure/tools/search/web_search/policy.rs`).

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::ConfigRepositoryError;

/// Raw allow/deny lists exactly as persisted. Empty vectors mean "no filter".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredPolicyLists {
    pub allowlist: Vec<String>,
    pub denylist: Vec<String>,
}

/// Returns the stored policy or `None` when the row is absent.
pub fn get_workspace_policy(
    conn: &Connection,
    workspace_path: &str,
) -> Result<Option<StoredPolicyLists>, ConfigRepositoryError> {
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT allowlist, denylist FROM workspace_search_policy WHERE workspace_path = ?1",
            params![workspace_path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((allow_json, deny_json)) = row else {
        return Ok(None);
    };
    let allowlist: Vec<String> = serde_json::from_str(&allow_json).unwrap_or_default();
    let denylist: Vec<String> = serde_json::from_str(&deny_json).unwrap_or_default();
    Ok(Some(StoredPolicyLists {
        allowlist,
        denylist,
    }))
}

/// Upserts the allow/deny lists for `workspace_path`, refreshing `updated_at`.
pub fn set_workspace_policy(
    conn: &Connection,
    workspace_path: &str,
    allowlist: &[String],
    denylist: &[String],
) -> Result<(), ConfigRepositoryError> {
    let now = Utc::now().to_rfc3339();
    let allow_json = serde_json::to_string(allowlist).unwrap_or_else(|_| "[]".to_string());
    let deny_json = serde_json::to_string(denylist).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO workspace_search_policy (workspace_path, allowlist, denylist, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(workspace_path) DO UPDATE SET
            allowlist  = excluded.allowlist,
            denylist   = excluded.denylist,
            updated_at = excluded.updated_at",
        params![workspace_path, allow_json, deny_json, now],
    )?;
    Ok(())
}

/// Deletes any stored policy for `workspace_path` (no-op when absent).
pub fn delete_workspace_policy(
    conn: &Connection,
    workspace_path: &str,
) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "DELETE FROM workspace_search_policy WHERE workspace_path = ?1",
        params![workspace_path],
    )?;
    Ok(())
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
    fn empty_policy_is_returned_when_no_row() {
        let conn = make_conn();
        assert!(get_workspace_policy(&conn, "/home/me/project")
            .unwrap()
            .is_none());
    }

    #[test]
    fn set_persists_allowlist_and_denylist() {
        let conn = make_conn();
        set_workspace_policy(
            &conn,
            "/home/me/project",
            &["github.com".to_string(), ".rust-lang.org".to_string()],
            &["pinterest.com".to_string()],
        )
        .unwrap();
        let stored = get_workspace_policy(&conn, "/home/me/project")
            .unwrap()
            .expect("row present");
        assert_eq!(stored.allowlist, vec!["github.com", ".rust-lang.org"]);
        assert_eq!(stored.denylist, vec!["pinterest.com"]);
    }

    #[test]
    fn set_upserts_on_duplicate_workspace_path() {
        let conn = make_conn();
        set_workspace_policy(&conn, "/p", &["a".to_string()], &[]).unwrap();
        set_workspace_policy(&conn, "/p", &["b".to_string()], &["c".to_string()]).unwrap();
        let stored = get_workspace_policy(&conn, "/p").unwrap().expect("row");
        assert_eq!(stored.allowlist, vec!["b"]);
        assert_eq!(stored.denylist, vec!["c"]);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM workspace_search_policy WHERE workspace_path = ?1",
                params!["/p"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn delete_removes_the_row() {
        let conn = make_conn();
        set_workspace_policy(&conn, "/p", &["a".to_string()], &[]).unwrap();
        delete_workspace_policy(&conn, "/p").unwrap();
        assert!(get_workspace_policy(&conn, "/p").unwrap().is_none());
    }

    #[test]
    fn delete_on_missing_row_is_ok() {
        let conn = make_conn();
        delete_workspace_policy(&conn, "/never-set").unwrap();
    }

    #[test]
    fn empty_string_workspace_path_is_the_global_bucket() {
        // The resolver uses the empty string as a sentinel for "no workspace".
        let conn = make_conn();
        set_workspace_policy(&conn, "", &[], &["bad.example.com".to_string()]).unwrap();
        let stored = get_workspace_policy(&conn, "").unwrap().expect("row");
        assert_eq!(stored.denylist, vec!["bad.example.com"]);
    }
}
