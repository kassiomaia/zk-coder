//! End-to-end behavior tests for `WebSearchTool`'s business logic.
//!
//! We drive `execute_inner` directly (not the `Tool::execute` trampoline)
//! so we can inject a `MockProvider` without needing a real Tauri `AppHandle`.
//! Env-var state is shared across the process, so the tests coordinate
//! through a module-scoped mutex.

use std::sync::{Arc, MutexGuard};

use serde_json::{json, Value as JsonValue};

use crate::infrastructure::tools::constants::{
    web_search_test_env_lock, ENV_WEB_SEARCH_ENABLED,
};
use crate::infrastructure::tools::search::web_search::execute_inner_for_tests;
use crate::infrastructure::tools::search::web_search::provider::mock::{
    MockProvider, MockResponse,
};
use crate::infrastructure::tools::search::web_search::provider::{
    RawResult, SearchProvider, SearchProviderError,
};
use crate::infrastructure::tools::search::web_search::ExecuteDeps;

/// RAII guard that holds the process-wide env-var lock and cleans the flag
/// up on drop, even if the test panics mid-way.
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

fn flag_off() -> EnvGuard {
    let lock = web_search_test_env_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::remove_var(ENV_WEB_SEARCH_ENABLED);
    EnvGuard { _lock: lock }
}

fn sample_raws() -> Vec<RawResult> {
    vec![
        RawResult {
            title: "Async traits in <b>Rust</b>".to_string(),
            url: "https://blog.rust-lang.org/post?utm_source=x&keep=1".to_string(),
            snippet_html: "Stable <b>async</b> traits arrive in 1.75. Tom &amp; Jerry.".to_string(),
        },
        RawResult {
            title: "async-trait crate".to_string(),
            url: "https://crates.io/crates/async-trait?fbclid=abc".to_string(),
            snippet_html: "Procedural macro to allow <code>async fn</code> in traits.".to_string(),
        },
    ]
}

async fn run(
    provider: Arc<dyn SearchProvider>,
    args: JsonValue,
) -> Result<
    crate::infrastructure::tools::search::web_search::types::WebSearchOutput,
    crate::infrastructure::tools::search::web_search::error::WebSearchFailure,
> {
    // These tests exercise the tool without a workspace policy or cache;
    // policy enforcement and cache behavior have their own dedicated suites
    // (`policy_tests.rs`, `cache_integration_tests.rs`).
    execute_inner_for_tests(ExecuteDeps::test_minimal(provider), args).await
}

#[tokio::test]
async fn execute_returns_disabled_error_when_feature_flag_off() {
    let _guard = flag_off();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let err = run(provider, json!({ "query": "rust" }))
        .await
        .expect_err("flag off must surface DisabledByFlag");
    assert_eq!(
        err.code(),
        crate::infrastructure::tools::search::web_search::error::ErrorCode::DisabledByFlag
    );
}

#[tokio::test]
async fn execute_returns_invalid_args_on_empty_query() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let err = run(provider, json!({ "query": "   " }))
        .await
        .expect_err("empty query must fail");
    assert_eq!(
        err.code(),
        crate::infrastructure::tools::search::web_search::error::ErrorCode::InvalidArguments
    );
}

#[tokio::test]
async fn execute_returns_invalid_args_on_query_too_long() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let oversized = "a".repeat(10_000);
    let err = run(provider, json!({ "query": oversized }))
        .await
        .expect_err("oversize query must fail");
    assert_eq!(
        err.code(),
        crate::infrastructure::tools::search::web_search::error::ErrorCode::InvalidArguments
    );
}

#[tokio::test]
async fn execute_returns_invalid_args_on_unknown_property() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let err = run(provider, json!({ "query": "rust", "secret": true }))
        .await
        .expect_err("unknown key must fail");
    assert_eq!(
        err.code(),
        crate::infrastructure::tools::search::web_search::error::ErrorCode::InvalidArguments
    );
}

#[tokio::test]
async fn execute_strips_html_from_snippets() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let out = run(provider, json!({ "query": "rust" })).await.expect("ok");
    for result in &out.results {
        assert!(!result.title.contains('<'), "title still has html tags");
        assert!(!result.snippet.contains('<'), "snippet still has html tags");
    }
}

#[tokio::test]
async fn execute_decodes_entities_in_snippets() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let out = run(provider, json!({ "query": "rust" })).await.expect("ok");
    assert!(out.results[0].snippet.contains("Tom & Jerry"));
    assert!(!out.results[0].snippet.contains("&amp;"));
}

#[tokio::test]
async fn execute_removes_utm_tracking_params_from_urls() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let out = run(provider, json!({ "query": "rust" })).await.expect("ok");
    for result in &out.results {
        assert!(!result.url.contains("utm_"), "utm param leaked: {}", result.url);
        assert!(!result.url.contains("fbclid"), "fbclid leaked: {}", result.url);
    }
    // Non-tracking params must be preserved.
    assert!(out.results[0].url.contains("keep=1"));
}

#[tokio::test]
async fn execute_derives_source_domain_from_url() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let out = run(provider, json!({ "query": "rust" })).await.expect("ok");
    assert_eq!(out.results[0].source_domain, "blog.rust-lang.org");
    assert_eq!(out.results[1].source_domain, "crates.io");
}

#[tokio::test]
async fn execute_clamps_num_results_to_max_10() {
    let _guard = flag_on();
    // Provider returns 15 entries; tool must cap at 10.
    let big: Vec<RawResult> = (0..15)
        .map(|i| RawResult {
            title: format!("r{i}"),
            url: format!("https://example.com/{i}"),
            snippet_html: String::new(),
        })
        .collect();
    let provider = Arc::new(MockProvider::with_results(big));
    let out = run(provider, json!({ "query": "rust", "num_results": 99 }))
        .await
        .expect("ok");
    assert_eq!(out.count, 10);
    assert_eq!(out.results.len(), 10);
}

#[tokio::test]
async fn execute_returns_structured_error_on_provider_rate_limit() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    provider.enqueue(MockResponse::RateLimited);
    let err = run(provider, json!({ "query": "rust" }))
        .await
        .expect_err("rate-limited must surface");
    assert_eq!(
        err.code(),
        crate::infrastructure::tools::search::web_search::error::ErrorCode::RateLimited
    );
}

#[tokio::test]
async fn execute_returns_structured_error_on_provider_unauthorized() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    provider.enqueue(MockResponse::Unauthorized);
    let err = run(provider, json!({ "query": "rust" }))
        .await
        .expect_err("unauthorized must surface");
    assert_eq!(
        err.code(),
        crate::infrastructure::tools::search::web_search::error::ErrorCode::Unauthorized
    );
}

#[tokio::test]
async fn execute_maps_provider_network_failure_to_upstream_error() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    provider.enqueue(MockResponse::Network("dns timeout".to_string()));
    let err = run(provider, json!({ "query": "rust" }))
        .await
        .expect_err("network error must surface");
    assert_eq!(
        err.code(),
        crate::infrastructure::tools::search::web_search::error::ErrorCode::UpstreamError
    );
}

#[tokio::test]
async fn execute_returns_empty_results_as_success() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(Vec::new()));
    let out = run(provider, json!({ "query": "rust" })).await.expect("ok");
    assert_eq!(out.count, 0);
    assert!(out.results.is_empty());
    assert_eq!(
        out.provider,
        crate::infrastructure::tools::search::web_search::types::PROVIDER_GOOGLE_CUSTOM_SEARCH
    );
}

#[tokio::test]
async fn execute_passes_site_restriction_to_provider() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let _ = run(
        provider.clone(),
        json!({ "query": "rust", "site": "github.com" }),
    )
    .await
    .expect("ok");
    let observed = provider.observed_requests();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].site.as_deref(), Some("github.com"));
}

#[tokio::test]
async fn execute_passes_language_to_provider() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let _ = run(
        provider.clone(),
        json!({ "query": "rust", "language": "pt-BR" }),
    )
    .await
    .expect("ok");
    let observed = provider.observed_requests();
    // The locale resolver lowercases the BCP-47 tag so cache keys stay
    // stable across case variants. Google accepts either case for `hl`.
    assert_eq!(observed[0].language.as_deref(), Some("pt-br"));
}

#[tokio::test]
async fn execute_output_excludes_html_and_tracking_params() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let out = run(provider, json!({ "query": "rust" })).await.expect("ok");
    let serialized = serde_json::to_string(&out).expect("serializable");
    assert!(!serialized.contains("<b>"));
    assert!(!serialized.contains("utm_source"));
    assert!(!serialized.contains("fbclid"));
}

#[tokio::test]
async fn execute_drops_results_with_non_http_urls() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(vec![
        RawResult {
            title: "keep".to_string(),
            url: "https://example.com/ok".to_string(),
            snippet_html: String::new(),
        },
        RawResult {
            title: "drop".to_string(),
            url: "javascript:alert(1)".to_string(),
            snippet_html: String::new(),
        },
    ]));
    let out = run(provider, json!({ "query": "rust" })).await.expect("ok");
    assert_eq!(out.count, 1);
    assert_eq!(out.results[0].title, "keep");
}

#[tokio::test]
async fn execute_success_output_defaults_cached_false_and_filter_none() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    let out = run(provider, json!({ "query": "rust" })).await.expect("ok");
    assert!(!out.cached);
    assert_eq!(out.filtered_by_policy, None);
}

#[tokio::test]
async fn execute_maps_upstream_provider_error() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(sample_raws()));
    provider.enqueue(MockResponse::Upstream("HTTP 502".to_string()));
    let err = run(provider, json!({ "query": "rust" }))
        .await
        .expect_err("upstream error must surface");
    assert_eq!(
        err.code(),
        crate::infrastructure::tools::search::web_search::error::ErrorCode::UpstreamError
    );
    // Test the From adapter too.
    let from: crate::infrastructure::tools::search::web_search::error::WebSearchFailure =
        SearchProviderError::Upstream("x".to_string()).into();
    assert_eq!(
        from.code(),
        crate::infrastructure::tools::search::web_search::error::ErrorCode::UpstreamError
    );
}
