//! End-to-end cache tests exercising `execute_inner` with an in-memory
//! SQLite-backed cache. The DB layer is already unit-tested in
//! `web_search_cache.rs`; these tests focus on the tool-level wiring:
//! - a hit skips the provider call and sets `cached: true`,
//! - the current policy is re-applied on read,
//! - an expired entry bypasses the cache and re-fetches,
//! - a fresh miss populates the cache on write.

use std::sync::{Arc, MutexGuard};
use std::time::Duration;

use serde_json::json;

use crate::core::config::WebSearchCacheRepository;
use crate::infrastructure::sqlite::repositories::config::SqliteConfigRepository;
use crate::infrastructure::tools::constants::{
    web_search_test_env_lock, ENV_WEB_SEARCH_ENABLED,
};
use crate::infrastructure::tools::search::web_search::cache::CacheHandle;
use crate::infrastructure::tools::search::web_search::execute_inner_for_tests;
use crate::infrastructure::tools::search::web_search::locale::FixedLocale;
use crate::infrastructure::tools::search::web_search::policy::Policy;
use crate::infrastructure::tools::search::web_search::provider::mock::MockProvider;
use crate::infrastructure::tools::search::web_search::provider::{RawResult, SearchProvider};
use crate::infrastructure::tools::search::web_search::ExecuteDeps;

struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        std::env::remove_var(ENV_WEB_SEARCH_ENABLED);
    }
}

fn flag_on() -> EnvGuard {
    let lock = web_search_test_env_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::set_var(ENV_WEB_SEARCH_ENABLED, "1");
    EnvGuard { _lock: lock }
}

fn make_cache(ttl: Duration) -> (CacheHandle, Arc<SqliteConfigRepository>) {
    let repo = Arc::new(
        SqliteConfigRepository::in_memory().expect("in-memory config repo"),
    );
    let handle = CacheHandle {
        repo: repo.clone() as Arc<dyn WebSearchCacheRepository>,
        ttl,
    };
    (handle, repo)
}

fn raw(domain: &str) -> RawResult {
    RawResult {
        title: format!("r-{domain}"),
        url: format!("https://{domain}/"),
        snippet_html: "snippet".to_string(),
    }
}

#[tokio::test]
async fn serves_repeat_request_from_cache_without_provider_call() {
    let _guard = flag_on();
    let (cache, _repo) = make_cache(Duration::from_secs(300));
    let provider = Arc::new(MockProvider::with_results(vec![raw("github.com")]));

    let deps1 = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::empty(),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let first = execute_inner_for_tests(deps1, json!({ "query": "rust" }))
        .await
        .expect("first ok");
    assert!(!first.cached, "first call is a miss");
    assert_eq!(provider.observed_requests().len(), 1);

    let deps2 = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::empty(),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let second = execute_inner_for_tests(deps2, json!({ "query": "rust" }))
        .await
        .expect("second ok");
    assert!(second.cached, "second call must come from the cache");
    assert_eq!(second.results, first.results);
    assert_eq!(
        provider.observed_requests().len(),
        1,
        "provider must not be called a second time"
    );
}

#[tokio::test]
async fn bypasses_cache_when_ttl_expired() {
    let _guard = flag_on();
    let (cache, _repo) = make_cache(Duration::from_millis(1));
    let provider = Arc::new(MockProvider::with_results(vec![raw("github.com")]));

    let deps = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::empty(),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let _ = execute_inner_for_tests(deps, json!({ "query": "rust" }))
        .await
        .expect("first ok");

    // Wait past the TTL.
    std::thread::sleep(std::time::Duration::from_millis(15));

    let deps2 = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::empty(),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let second = execute_inner_for_tests(deps2, json!({ "query": "rust" }))
        .await
        .expect("second ok");
    assert!(!second.cached, "expired entry must not serve");
    assert_eq!(
        provider.observed_requests().len(),
        2,
        "provider must be hit again after expiry"
    );
}

#[tokio::test]
async fn reapplies_policy_to_cached_results_after_policy_change() {
    let _guard = flag_on();
    let (cache, _repo) = make_cache(Duration::from_secs(300));
    let provider = Arc::new(MockProvider::with_results(vec![
        raw("github.com"),
        raw("pinterest.com"),
    ]));

    // First call: no policy → both results get cached.
    let deps_open = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::empty(),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let first = execute_inner_for_tests(deps_open, json!({ "query": "rust" }))
        .await
        .expect("first ok");
    assert_eq!(first.count, 2);
    assert_eq!(first.filtered_by_policy, None);

    // Second call: the cached rows are still here, but the policy now
    // denylists pinterest.com → the cached result must be filtered on read.
    let deps_tight = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::from_raw(&[], &["pinterest.com".to_string()]),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let second = execute_inner_for_tests(deps_tight, json!({ "query": "rust" }))
        .await
        .expect("second ok");
    assert!(second.cached);
    assert_eq!(second.count, 1);
    assert_eq!(second.results[0].source_domain, "github.com");
    assert_eq!(second.filtered_by_policy, Some(true));
    assert_eq!(
        provider.observed_requests().len(),
        1,
        "policy change must not cause a re-fetch"
    );
}

#[tokio::test]
async fn cache_partition_is_scoped_per_workspace() {
    let _guard = flag_on();
    let (cache, _repo) = make_cache(Duration::from_secs(300));
    let provider = Arc::new(MockProvider::with_results(vec![raw("github.com")]));

    // Workspace A populates the cache.
    let deps_a = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::empty(),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws-a".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let _ = execute_inner_for_tests(deps_a, json!({ "query": "rust" }))
        .await
        .expect("A miss");
    assert_eq!(provider.observed_requests().len(), 1);

    // Workspace B issues the same query — must miss because cache is
    // partitioned per `workspace_key`.
    let deps_b = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::empty(),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws-b".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let b = execute_inner_for_tests(deps_b, json!({ "query": "rust" }))
        .await
        .expect("B miss");
    assert!(!b.cached);
    assert_eq!(
        provider.observed_requests().len(),
        2,
        "separate workspace must not share cached results"
    );
}

#[tokio::test]
async fn cache_put_survives_an_empty_result_set() {
    let _guard = flag_on();
    let (cache, _repo) = make_cache(Duration::from_secs(300));
    let provider = Arc::new(MockProvider::with_results(Vec::new()));

    let deps1 = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::empty(),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let _ = execute_inner_for_tests(deps1, json!({ "query": "rust" }))
        .await
        .expect("first ok");

    let deps2 = ExecuteDeps {
        provider: provider.clone(),
        policy: Policy::empty(),
        cache: Some(CacheHandle {
            repo: cache.repo.clone(),
            ttl: cache.ttl,
        }),
        workspace_key: "/ws".to_string(),
        locale: Arc::new(FixedLocale(None)),
    };
    let second = execute_inner_for_tests(deps2, json!({ "query": "rust" }))
        .await
        .expect("second ok");
    assert!(second.cached, "zero-result response is still cacheable");
    assert_eq!(second.count, 0);
}
