//! Tauri commands for web-search cache management.

use serde::Serialize;

use crate::commands::config::state::ConfigState;
use crate::core::config::WebSearchCacheRepository;
use crate::infrastructure::tools::search::web_search::cache::DEFAULT_CACHE_TTL_SECS;

/// Stats surfaced in the Settings UI.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    /// Total row count (including expired rows awaiting the next sweep).
    pub entries: usize,
    /// Default TTL applied to new entries, in seconds.
    pub default_ttl_secs: u64,
}

pub(crate) fn clear_web_search_cache_impl(
    repo: &dyn WebSearchCacheRepository,
    workspace_path: Option<&str>,
) -> Result<usize, String> {
    repo.clear(workspace_path).map_err(|e| e.to_string())
}

pub(crate) fn web_search_cache_stats_impl(
    repo: &dyn WebSearchCacheRepository,
) -> Result<CacheStats, String> {
    let entries = repo.count().map_err(|e| e.to_string())?;
    Ok(CacheStats {
        entries,
        default_ttl_secs: DEFAULT_CACHE_TTL_SECS,
    })
}

/// Clears cache entries. `workspace_path = None` empties the whole table.
#[tauri::command]
pub async fn clear_web_search_cache(
    workspace_path: Option<String>,
    state: tauri::State<'_, ConfigState>,
) -> Result<usize, String> {
    clear_web_search_cache_impl(&*state.repository, workspace_path.as_deref())
}

/// Returns cache-level stats (entry count + default TTL).
#[tauri::command]
pub async fn web_search_cache_stats(
    state: tauri::State<'_, ConfigState>,
) -> Result<CacheStats, String> {
    web_search_cache_stats_impl(&*state.repository)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::NewCachedWebSearchEntry;
    use crate::infrastructure::sqlite::repositories::config::SqliteConfigRepository;
    use chrono::{Duration, Utc};
    use std::sync::Arc;

    fn repo() -> Arc<SqliteConfigRepository> {
        Arc::new(SqliteConfigRepository::in_memory().expect("in-memory"))
    }

    fn insert_entry(repo: &SqliteConfigRepository, key: &str, workspace: &str) {
        let now = Utc::now();
        repo.put(&NewCachedWebSearchEntry {
            cache_key: key,
            query: "rust",
            num_results: 5,
            language: None,
            site: None,
            workspace_path: workspace,
            provider: "google_custom_search",
            results_json: "[]",
            created_at: now,
            expires_at: now + Duration::hours(1),
        })
        .unwrap();
    }

    #[test]
    fn clear_web_search_cache_all_empties_table() {
        let r = repo();
        insert_entry(&r, "k1", "/a");
        insert_entry(&r, "k2", "/b");
        let removed = clear_web_search_cache_impl(r.as_ref(), None).unwrap();
        assert_eq!(removed, 2);
        let stats = web_search_cache_stats_impl(r.as_ref()).unwrap();
        assert_eq!(stats.entries, 0);
    }

    #[test]
    fn clear_web_search_cache_by_workspace_removes_partition() {
        let r = repo();
        insert_entry(&r, "k1", "/a");
        insert_entry(&r, "k2", "/b");
        let removed = clear_web_search_cache_impl(r.as_ref(), Some("/a")).unwrap();
        assert_eq!(removed, 1);
        let stats = web_search_cache_stats_impl(r.as_ref()).unwrap();
        assert_eq!(stats.entries, 1);
    }

    #[test]
    fn cache_stats_returns_entry_count_and_default_ttl() {
        let r = repo();
        insert_entry(&r, "k1", "/a");
        let stats = web_search_cache_stats_impl(r.as_ref()).unwrap();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.default_ttl_secs, DEFAULT_CACHE_TTL_SECS);
    }
}
