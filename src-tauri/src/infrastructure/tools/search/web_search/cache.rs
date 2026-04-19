//! Tool-side cache concerns: key construction, TTL config, and the lookup /
//! write orchestration called from `execute_inner`.
//!
//! The DB layer (`WebSearchCacheRepository`) stores opaque strings keyed by
//! SHA-256 hash. This module owns the hash input, the normalization rules
//! that drive it, and the deserialization/serialization of the JSON payload.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::core::config::{NewCachedWebSearchEntry, WebSearchCacheRepository};

use super::types::WebSearchResult;

/// Default TTL for a fresh cache entry. Matches `spec.md §4.10`.
pub const DEFAULT_CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// Inputs that feed the cache key.
#[derive(Debug, Clone)]
pub struct CacheKeyInput<'a> {
    pub query: &'a str,
    pub num_results: usize,
    pub language: Option<&'a str>,
    pub site: Option<&'a str>,
    pub workspace_key: &'a str,
}

/// Normalizes a query for cache keying: trims and collapses internal
/// whitespace so `"rust   async"` and `" rust async "` produce the same key.
#[must_use]
pub fn normalize_query(query: &str) -> String {
    query.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Returns the hex-encoded SHA-256 cache key for `input`. Normalization:
/// query whitespace-collapsed and lowercased, language/site lowercased.
#[must_use]
pub fn cache_key(input: &CacheKeyInput<'_>) -> String {
    let query = normalize_query(input.query).to_ascii_lowercase();
    let language = input
        .language
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let site = input
        .site
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let body = format!(
        "{}\n{}\n{}\n{}\n{}",
        query, input.num_results, language, site, input.workspace_key
    );
    let digest = Sha256::digest(body.as_bytes());
    format!("{digest:x}")
}

/// Bundle that the tool layer carries when the cache is enabled.
pub struct CacheHandle {
    /// Storage implementation (in-memory or SQLite-backed).
    pub repo: Arc<dyn WebSearchCacheRepository>,
    /// TTL applied to new entries.
    pub ttl: Duration,
}

impl CacheHandle {
    /// Looks up a fresh entry; returns `None` on miss, expired, DB error.
    #[must_use]
    pub fn read(&self, key: &str) -> Option<Vec<WebSearchResult>> {
        let entry = self.repo.get_fresh(key, Utc::now()).ok()??;
        serde_json::from_str::<Vec<WebSearchResult>>(&entry.results_json).ok()
    }

    /// Persists a post-policy-filter result set. DB errors are logged but
    /// never surface to the caller — a cache-put failure should not break the
    /// happy path of the tool response.
    pub fn write(
        &self,
        key: &str,
        input: &CacheKeyInput<'_>,
        provider: &str,
        results: &[WebSearchResult],
    ) {
        let Ok(results_json) = serde_json::to_string(results) else {
            return;
        };
        let now = Utc::now();
        let expires_at = now
            + chrono::Duration::from_std(self.ttl)
                .unwrap_or_else(|_| chrono::Duration::seconds(i64::MAX / 2));
        let entry = NewCachedWebSearchEntry {
            cache_key: key,
            query: input.query,
            num_results: input.num_results,
            language: input.language,
            site: input.site,
            workspace_path: input.workspace_key,
            provider,
            results_json: &results_json,
            created_at: now,
            expires_at,
        };
        if let Err(err) = self.repo.put(&entry) {
            log::warn!(
                target: "web_search",
                "failed to persist cache entry key={} error={}",
                key,
                err
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base<'a>() -> CacheKeyInput<'a> {
        CacheKeyInput {
            query: "rust async",
            num_results: 5,
            language: Some("en"),
            site: None,
            workspace_key: "/ws",
        }
    }

    #[test]
    fn same_inputs_produce_same_key() {
        let a = cache_key(&base());
        let b = cache_key(&base());
        assert_eq!(a, b);
    }

    #[test]
    fn different_query_produces_different_key() {
        let a = cache_key(&base());
        let b = cache_key(&CacheKeyInput {
            query: "rust sync",
            ..base()
        });
        assert_ne!(a, b);
    }

    #[test]
    fn different_num_results_produces_different_key() {
        let a = cache_key(&base());
        let b = cache_key(&CacheKeyInput {
            num_results: 6,
            ..base()
        });
        assert_ne!(a, b);
    }

    #[test]
    fn different_language_produces_different_key() {
        let a = cache_key(&base());
        let b = cache_key(&CacheKeyInput {
            language: Some("pt-br"),
            ..base()
        });
        assert_ne!(a, b);
    }

    #[test]
    fn different_site_produces_different_key() {
        let a = cache_key(&base());
        let b = cache_key(&CacheKeyInput {
            site: Some("github.com"),
            ..base()
        });
        assert_ne!(a, b);
    }

    #[test]
    fn different_workspace_produces_different_key() {
        let a = cache_key(&base());
        let b = cache_key(&CacheKeyInput {
            workspace_key: "/other",
            ..base()
        });
        assert_ne!(a, b);
    }

    #[test]
    fn normalization_collapses_surrounding_whitespace_in_query() {
        let a = cache_key(&base());
        let b = cache_key(&CacheKeyInput {
            query: "   rust   async   ",
            ..base()
        });
        assert_eq!(a, b);
    }

    #[test]
    fn normalization_lowercases_language_tag() {
        let a = cache_key(&CacheKeyInput {
            language: Some("EN"),
            ..base()
        });
        let b = cache_key(&CacheKeyInput {
            language: Some("en"),
            ..base()
        });
        assert_eq!(a, b);
    }

    #[test]
    fn normalization_lowercases_site_and_query() {
        let a = cache_key(&CacheKeyInput {
            query: "RUST async",
            site: Some("GITHUB.com"),
            ..base()
        });
        let b = cache_key(&CacheKeyInput {
            query: "rust async",
            site: Some("github.com"),
            ..base()
        });
        assert_eq!(a, b);
    }

    #[test]
    fn none_language_and_site_are_stable() {
        let a = cache_key(&CacheKeyInput {
            language: None,
            site: None,
            ..base()
        });
        // Empty string for language/site must produce the same key as None,
        // since both collapse to "" in the hash body.
        let b = cache_key(&CacheKeyInput {
            language: Some(""),
            site: Some(""),
            ..base()
        });
        assert_eq!(a, b);
    }
}
