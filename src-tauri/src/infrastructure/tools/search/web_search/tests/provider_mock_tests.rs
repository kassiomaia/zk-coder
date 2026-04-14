//! Unit tests for the `MockProvider` — these double as executable documentation
//! of the `SearchProvider` seam that `WebSearchTool` will depend on in Phase 3.

use crate::infrastructure::tools::search::web_search::provider::mock::{
    MockProvider, MockResponse,
};
use crate::infrastructure::tools::search::web_search::provider::{
    RawResult, SearchProvider, SearchProviderError, SearchRequest,
};

fn sample_request() -> SearchRequest {
    SearchRequest {
        query: "rust async traits".to_string(),
        num_results: 5,
        language: Some("en".to_string()),
        site: None,
    }
}

fn sample_results() -> Vec<RawResult> {
    vec![
        RawResult {
            title: "Async traits in Rust".to_string(),
            url: "https://blog.rust-lang.org/async-traits".to_string(),
            snippet_html: "Stable <b>async</b> traits…".to_string(),
        },
        RawResult {
            title: "async-trait crate".to_string(),
            url: "https://crates.io/crates/async-trait".to_string(),
            snippet_html: "Procedural macro to…".to_string(),
        },
    ]
}

#[tokio::test]
async fn mock_provider_returns_canned_results() {
    let mock = MockProvider::with_results(sample_results());
    let out = mock.search(sample_request()).await.expect("ok");
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].url, "https://blog.rust-lang.org/async-traits");
}

#[tokio::test]
async fn mock_provider_records_observed_requests() {
    let mock = MockProvider::with_results(sample_results());
    let first = SearchRequest {
        query: "first".to_string(),
        num_results: 3,
        language: None,
        site: None,
    };
    let second = SearchRequest {
        query: "second".to_string(),
        num_results: 7,
        language: Some("pt-BR".to_string()),
        site: Some("svelte.dev".to_string()),
    };

    let _ = mock.search(first.clone()).await;
    let _ = mock.search(second.clone()).await;

    let observed = mock.observed_requests();
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[0], first);
    assert_eq!(observed[1], second);
}

#[tokio::test]
async fn mock_provider_returns_rate_limit_error_on_demand() {
    let mock = MockProvider::with_results(sample_results());
    mock.enqueue(MockResponse::RateLimited);

    let err = mock.search(sample_request()).await.expect_err("should error");
    assert!(matches!(err, SearchProviderError::RateLimited));
}

#[tokio::test]
async fn mock_provider_returns_network_error_on_demand() {
    let mock = MockProvider::with_results(sample_results());
    mock.enqueue(MockResponse::Network("dns lookup failed".to_string()));

    let err = mock.search(sample_request()).await.expect_err("should error");
    match err {
        SearchProviderError::Network(msg) => assert_eq!(msg, "dns lookup failed"),
        other => panic!("expected Network, got {other:?}"),
    }
}

#[tokio::test]
async fn mock_provider_returns_unauthorized_error_on_demand() {
    let mock = MockProvider::with_results(sample_results());
    mock.enqueue(MockResponse::Unauthorized);
    let err = mock.search(sample_request()).await.expect_err("should error");
    assert!(matches!(err, SearchProviderError::Unauthorized));
}

#[tokio::test]
async fn mock_provider_returns_upstream_error_on_demand() {
    let mock = MockProvider::with_results(sample_results());
    mock.enqueue(MockResponse::Upstream("HTTP 503".to_string()));
    let err = mock.search(sample_request()).await.expect_err("should error");
    match err {
        SearchProviderError::Upstream(msg) => assert_eq!(msg, "HTTP 503"),
        other => panic!("expected Upstream, got {other:?}"),
    }
}

#[tokio::test]
async fn mock_provider_scripted_responses_consume_fifo_then_fall_back() {
    let mock = MockProvider::with_results(sample_results());
    mock.enqueue(MockResponse::RateLimited);
    mock.enqueue(MockResponse::Ok(vec![RawResult {
        title: "second call".to_string(),
        url: "https://example.com".to_string(),
        snippet_html: String::new(),
    }]));

    assert!(matches!(
        mock.search(sample_request()).await,
        Err(SearchProviderError::RateLimited)
    ));

    let second = mock.search(sample_request()).await.expect("second ok");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].title, "second call");

    let third = mock.search(sample_request()).await.expect("third falls back");
    assert_eq!(third.len(), 2, "default response returns after script drains");
}
