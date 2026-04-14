//! Persistence layer for the web-search result cache.
//!
//! The key is SHA-256-derived and opaque to this layer; key construction
//! lives next to the tool in
//! `infrastructure/tools/search/web_search/cache.rs`. Time is passed in by
//! the caller so unit tests can drive expiry deterministically without
//! reaching for a mockable clock.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::ConfigRepositoryError;

/// Raw row returned by a fresh lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCacheEntry {
    /// JSON-encoded `Vec<WebSearchResult>` exactly as persisted.
    pub results_json: String,
    /// Provider id recorded at insert time.
    pub provider: String,
}

/// Everything the cache needs to persist a new entry.
pub struct NewCacheEntry<'a> {
    pub cache_key: &'a str,
    pub query: &'a str,
    pub num_results: usize,
    pub language: Option<&'a str>,
    pub site: Option<&'a str>,
    pub workspace_path: &'a str,
    pub provider: &'a str,
    pub results_json: &'a str,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Returns the cached entry when one exists AND has not expired relative to
/// `now`. Expired rows are left in place for the startup sweep to collect.
pub fn get_fresh(
    conn: &Connection,
    cache_key: &str,
    now: DateTime<Utc>,
) -> Result<Option<StoredCacheEntry>, ConfigRepositoryError> {
    let row: Option<(String, String, String)> = conn
        .query_row(
            "SELECT results, provider, expires_at
             FROM web_search_cache
             WHERE cache_key = ?1",
            params![cache_key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    let Some((results, provider, expires_at)) = row else {
        return Ok(None);
    };

    let expires = DateTime::parse_from_rfc3339(&expires_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(now);
    if expires <= now {
        return Ok(None);
    }

    Ok(Some(StoredCacheEntry {
        results_json: results,
        provider,
    }))
}

/// Upserts an entry. Reusing the same key refreshes both the payload and the
/// expiry, matching how the tool layer expects "most recent read wins".
pub fn put(
    conn: &Connection,
    entry: &NewCacheEntry<'_>,
) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "INSERT INTO web_search_cache (
            cache_key, query, num_results, language, site, workspace_path,
            provider, results, created_at, expires_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(cache_key) DO UPDATE SET
            query          = excluded.query,
            num_results    = excluded.num_results,
            language       = excluded.language,
            site           = excluded.site,
            workspace_path = excluded.workspace_path,
            provider       = excluded.provider,
            results        = excluded.results,
            created_at     = excluded.created_at,
            expires_at     = excluded.expires_at",
        params![
            entry.cache_key,
            entry.query,
            i64::try_from(entry.num_results).unwrap_or(i64::MAX),
            entry.language,
            entry.site,
            entry.workspace_path,
            entry.provider,
            entry.results_json,
            entry.created_at.to_rfc3339(),
            entry.expires_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Drops every row whose `expires_at <= now`. Returns the number deleted.
pub fn sweep_expired(
    conn: &Connection,
    now: DateTime<Utc>,
) -> Result<usize, ConfigRepositoryError> {
    let removed = conn.execute(
        "DELETE FROM web_search_cache WHERE expires_at <= ?1",
        params![now.to_rfc3339()],
    )?;
    Ok(removed)
}

/// When `workspace_path` is `Some`, clears only that partition; otherwise
/// empties the whole table. Returns the number deleted.
pub fn clear(
    conn: &Connection,
    workspace_path: Option<&str>,
) -> Result<usize, ConfigRepositoryError> {
    let removed = match workspace_path {
        Some(path) => conn.execute(
            "DELETE FROM web_search_cache WHERE workspace_path = ?1",
            params![path],
        )?,
        None => conn.execute("DELETE FROM web_search_cache", [])?,
    };
    Ok(removed)
}

/// Total row count (including any expired rows yet to be swept).
pub fn count(conn: &Connection) -> Result<usize, ConfigRepositoryError> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM web_search_cache", [], |row| row.get(0))?;
    Ok(usize::try_from(n).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::migrations::config_schema;
    use chrono::Duration;

    fn make_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        config_schema::init_schema(&conn).unwrap();
        conn
    }

    fn sample_entry<'a>(
        key: &'a str,
        workspace: &'a str,
        now: DateTime<Utc>,
        expires: DateTime<Utc>,
    ) -> NewCacheEntry<'a> {
        NewCacheEntry {
            cache_key: key,
            query: "rust async",
            num_results: 5,
            language: Some("en"),
            site: None,
            workspace_path: workspace,
            provider: "google_custom_search",
            results_json: r#"[{"title":"x","url":"https://x/","snippet":"","source_domain":"x"}]"#,
            created_at: now,
            expires_at: expires,
        }
    }

    #[test]
    fn miss_returns_none_when_key_absent() {
        let conn = make_conn();
        let now = Utc::now();
        assert!(get_fresh(&conn, "missing", now).unwrap().is_none());
    }

    #[test]
    fn miss_returns_none_when_entry_expired() {
        let conn = make_conn();
        let now = Utc::now();
        let expired_at = now - Duration::seconds(1);
        put(&conn, &sample_entry("k1", "/ws", now - Duration::hours(1), expired_at)).unwrap();
        assert!(get_fresh(&conn, "k1", now).unwrap().is_none());
    }

    #[test]
    fn hit_returns_results_when_entry_fresh() {
        let conn = make_conn();
        let now = Utc::now();
        let expires = now + Duration::hours(1);
        put(&conn, &sample_entry("k1", "/ws", now, expires)).unwrap();
        let hit = get_fresh(&conn, "k1", now).unwrap().expect("hit");
        assert!(hit.results_json.contains("\"url\":\"https://x/\""));
        assert_eq!(hit.provider, "google_custom_search");
    }

    #[test]
    fn put_upserts_on_duplicate_key() {
        let conn = make_conn();
        let now = Utc::now();
        let expires = now + Duration::hours(1);

        put(&conn, &sample_entry("k1", "/ws", now, expires)).unwrap();

        let replacement = NewCacheEntry {
            results_json: r#"[{"title":"new","url":"https://new/","snippet":"","source_domain":"new"}]"#,
            ..sample_entry("k1", "/ws", now, expires)
        };
        put(&conn, &replacement).unwrap();

        assert_eq!(count(&conn).unwrap(), 1, "upsert must not create duplicates");
        let hit = get_fresh(&conn, "k1", now).unwrap().expect("hit");
        assert!(hit.results_json.contains("\"url\":\"https://new/\""));
    }

    #[test]
    fn sweep_expired_removes_only_expired_rows() {
        let conn = make_conn();
        let now = Utc::now();
        put(
            &conn,
            &sample_entry("fresh", "/ws", now, now + Duration::hours(1)),
        )
        .unwrap();
        put(
            &conn,
            &sample_entry("stale", "/ws", now - Duration::days(2), now - Duration::seconds(5)),
        )
        .unwrap();
        let removed = sweep_expired(&conn, now).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(count(&conn).unwrap(), 1);
        assert!(get_fresh(&conn, "fresh", now).unwrap().is_some());
    }

    #[test]
    fn clear_by_workspace_path_removes_only_that_partition() {
        let conn = make_conn();
        let now = Utc::now();
        let expires = now + Duration::hours(1);
        put(&conn, &sample_entry("a", "/ws-1", now, expires)).unwrap();
        put(&conn, &sample_entry("b", "/ws-2", now, expires)).unwrap();

        let removed = clear(&conn, Some("/ws-1")).unwrap();
        assert_eq!(removed, 1);
        assert!(get_fresh(&conn, "a", now).unwrap().is_none());
        assert!(get_fresh(&conn, "b", now).unwrap().is_some());
    }

    #[test]
    fn clear_all_empties_the_table() {
        let conn = make_conn();
        let now = Utc::now();
        let expires = now + Duration::hours(1);
        put(&conn, &sample_entry("a", "/ws-1", now, expires)).unwrap();
        put(&conn, &sample_entry("b", "/ws-2", now, expires)).unwrap();
        let removed = clear(&conn, None).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(count(&conn).unwrap(), 0);
    }
}
