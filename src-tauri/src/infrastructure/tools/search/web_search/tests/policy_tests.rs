//! End-to-end tests for policy enforcement inside `execute_inner`. Pure
//! `Policy` logic is covered by the inline tests in `policy.rs`; this file
//! focuses on the integration seam: how arguments, provider behavior, and
//! output `filtered_by_policy` flag interact with a non-trivial policy.

use std::sync::{Arc, MutexGuard};

use serde_json::{json, Value as JsonValue};

use crate::infrastructure::tools::constants::{
    web_search_test_env_lock, ENV_WEB_SEARCH_ENABLED,
};
use crate::infrastructure::tools::search::web_search::error::{ErrorCode, WebSearchFailure};
use crate::infrastructure::tools::search::web_search::execute_inner_for_tests;
use crate::infrastructure::tools::search::web_search::locale::FixedLocale;
use crate::infrastructure::tools::search::web_search::policy::Policy;
use crate::infrastructure::tools::search::web_search::provider::mock::MockProvider;
use crate::infrastructure::tools::search::web_search::provider::{RawResult, SearchProvider};
use crate::infrastructure::tools::search::web_search::types::WebSearchOutput;
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

fn raw(domain: &str) -> RawResult {
    RawResult {
        title: format!("result {domain}"),
        url: format!("https://{domain}/"),
        snippet_html: "snippet".to_string(),
    }
}

async fn run(
    provider: Arc<dyn SearchProvider>,
    policy: Policy,
    args: JsonValue,
) -> Result<WebSearchOutput, WebSearchFailure> {
    let deps = ExecuteDeps {
        provider,
        policy,
        cache: None,
        workspace_key: String::new(),
        locale: Arc::new(FixedLocale(None)),
    };
    execute_inner_for_tests(deps, args).await
}

#[tokio::test]
async fn allows_all_when_allowlist_and_denylist_are_empty() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(vec![raw("github.com")]));
    let out = run(provider, Policy::empty(), json!({ "query": "rust" }))
        .await
        .expect("ok");
    assert_eq!(out.count, 1);
    assert_eq!(out.filtered_by_policy, None);
}

#[tokio::test]
async fn rejects_explicit_site_not_in_allowlist() {
    let _guard = flag_on();
    let policy = Policy::from_raw(&["github.com".to_string()], &[]);
    let provider = Arc::new(MockProvider::with_results(vec![raw("github.com")]));
    let err = run(
        provider,
        policy,
        json!({ "query": "rust", "site": "pinterest.com" }),
    )
    .await
    .expect_err("should fail policy");
    assert_eq!(err.code(), ErrorCode::FilteredByPolicy);
}

#[tokio::test]
async fn rejects_explicit_site_in_denylist() {
    let _guard = flag_on();
    let policy = Policy::from_raw(&[], &["pinterest.com".to_string()]);
    let provider = Arc::new(MockProvider::with_results(vec![raw("pinterest.com")]));
    let err = run(
        provider,
        policy,
        json!({ "query": "rust", "site": "pinterest.com" }),
    )
    .await
    .expect_err("denied site should fail policy");
    assert_eq!(err.code(), ErrorCode::FilteredByPolicy);
}

#[tokio::test]
async fn accepts_subdomain_site_when_pattern_has_leading_dot() {
    let _guard = flag_on();
    let policy = Policy::from_raw(&[".rust-lang.org".to_string()], &[]);
    let provider = Arc::new(MockProvider::with_results(vec![raw("blog.rust-lang.org")]));
    let out = run(
        provider,
        policy,
        json!({ "query": "rust", "site": "blog.rust-lang.org" }),
    )
    .await
    .expect("subdomain should be permitted");
    assert_eq!(out.count, 1);
}

#[tokio::test]
async fn wildcard_star_allowlist_permits_any_site() {
    let _guard = flag_on();
    let policy = Policy::from_raw(&["*".to_string()], &[]);
    let provider = Arc::new(MockProvider::with_results(vec![raw("anywhere.example")]));
    let out = run(
        provider,
        policy,
        json!({ "query": "rust", "site": "anywhere.example" }),
    )
    .await
    .expect("star permits any");
    assert_eq!(out.count, 1);
}

#[tokio::test]
async fn filters_provider_results_to_allowlist() {
    let _guard = flag_on();
    let policy = Policy::from_raw(&["github.com".to_string()], &[]);
    let provider = Arc::new(MockProvider::with_results(vec![
        raw("github.com"),
        raw("pinterest.com"),
    ]));
    let out = run(provider, policy, json!({ "query": "rust" }))
        .await
        .expect("ok");
    assert_eq!(out.count, 1);
    assert_eq!(out.results[0].source_domain, "github.com");
    assert_eq!(out.filtered_by_policy, Some(true));
}

#[tokio::test]
async fn drops_denylisted_urls_from_results() {
    let _guard = flag_on();
    let policy = Policy::from_raw(&[], &["pinterest.com".to_string()]);
    let provider = Arc::new(MockProvider::with_results(vec![
        raw("github.com"),
        raw("pinterest.com"),
    ]));
    let out = run(provider, policy, json!({ "query": "rust" }))
        .await
        .expect("ok");
    assert_eq!(out.count, 1);
    assert_eq!(out.filtered_by_policy, Some(true));
}

#[tokio::test]
async fn returns_filtered_by_policy_flag_when_filtering_empties_results() {
    let _guard = flag_on();
    let policy = Policy::from_raw(&[], &[".pinterest.com".to_string()]);
    let provider = Arc::new(MockProvider::with_results(vec![raw("de.pinterest.com")]));
    let out = run(provider, policy, json!({ "query": "rust" }))
        .await
        .expect("ok");
    // Spec: filtering to zero is still a success, not an error.
    assert_eq!(out.count, 0);
    assert!(out.results.is_empty());
    assert_eq!(out.filtered_by_policy, Some(true));
}

#[tokio::test]
async fn rewrites_request_to_or_expanded_site_when_no_site_given_and_allowlist_set() {
    let _guard = flag_on();
    let policy = Policy::from_raw(
        &["github.com".to_string(), ".rust-lang.org".to_string()],
        &[],
    );
    let provider = Arc::new(MockProvider::with_results(vec![raw("github.com")]));
    let _ = run(provider.clone(), policy, json!({ "query": "rust" }))
        .await
        .expect("ok");
    let observed = provider.observed_requests();
    assert_eq!(observed.len(), 1);
    let q = &observed[0].query;
    assert!(q.contains("site:github.com"), "q: {q}");
    assert!(q.contains("site:rust-lang.org"), "q: {q}");
    assert!(q.contains("OR"), "expected OR-expansion: {q}");
}

#[tokio::test]
async fn does_not_rewrite_query_when_explicit_site_given() {
    let _guard = flag_on();
    let policy = Policy::from_raw(&["github.com".to_string()], &[]);
    let provider = Arc::new(MockProvider::with_results(vec![raw("github.com")]));
    let _ = run(
        provider.clone(),
        policy,
        json!({ "query": "rust", "site": "github.com" }),
    )
    .await
    .expect("ok");
    let observed = provider.observed_requests();
    assert_eq!(observed[0].query, "rust", "explicit site must not trigger rewrite");
    assert_eq!(observed[0].site.as_deref(), Some("github.com"));
}

#[tokio::test]
async fn empty_policy_does_not_rewrite_query() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(vec![raw("github.com")]));
    let _ = run(provider.clone(), Policy::empty(), json!({ "query": "rust" }))
        .await
        .expect("ok");
    let observed = provider.observed_requests();
    assert_eq!(observed[0].query, "rust");
}

#[tokio::test]
async fn star_only_allowlist_does_not_rewrite_query() {
    let _guard = flag_on();
    let policy = Policy::from_raw(&["*".to_string()], &[]);
    let provider = Arc::new(MockProvider::with_results(vec![raw("anywhere.example")]));
    let _ = run(provider.clone(), policy, json!({ "query": "rust" }))
        .await
        .expect("ok");
    let observed = provider.observed_requests();
    assert_eq!(observed[0].query, "rust");
}
