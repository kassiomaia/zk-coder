//! Offline tests for `GoogleProvider`, backed by `wiremock`.
//!
//! Every test stands up a fresh mock server, points the provider at it via
//! `with_base_url`, and asserts either the request shape sent to Google or
//! the mapping of a canned response into `RawResult`s. No network access
//! required.

use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::infrastructure::tools::search::web_search::provider::google::GoogleProvider;
use crate::infrastructure::tools::search::web_search::provider::{
    SearchProvider, SearchProviderError, SearchRequest,
};
use crate::infrastructure::tools::search::web_search::secret::RedactedSecret;

fn base_request() -> SearchRequest {
    SearchRequest {
        query: "rust async traits".to_string(),
        num_results: 5,
        language: None,
        site: None,
    }
}

fn provider_for(server: &MockServer) -> GoogleProvider {
    GoogleProvider::new(RedactedSecret::new("test-key"), "test-cx".to_string())
        .with_base_url(server.uri())
}

#[tokio::test]
async fn maps_response_items_to_raw_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
                {
                    "title": "Async traits in Rust",
                    "link": "https://blog.rust-lang.org/async",
                    "snippet": "Stable async traits",
                    "htmlSnippet": "Stable <b>async</b> traits"
                },
                {
                    "title": "async-trait",
                    "link": "https://crates.io/crates/async-trait",
                    "snippet": "Procedural macro."
                }
            ]
        })))
        .mount(&server)
        .await;

    let raws = provider_for(&server)
        .search(base_request())
        .await
        .expect("ok");
    assert_eq!(raws.len(), 2);
    assert_eq!(raws[0].title, "Async traits in Rust");
    assert_eq!(raws[0].url, "https://blog.rust-lang.org/async");
    // Prefers htmlSnippet over snippet when both are present.
    assert_eq!(raws[0].snippet_html, "Stable <b>async</b> traits");
    // Falls back to snippet when htmlSnippet is absent.
    assert_eq!(raws[1].snippet_html, "Procedural macro.");
}

#[tokio::test]
async fn returns_rate_limited_on_http_429() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let err = provider_for(&server)
        .search(base_request())
        .await
        .expect_err("should error");
    assert!(matches!(err, SearchProviderError::RateLimited));
}

#[tokio::test]
async fn returns_unauthorized_on_http_401() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let err = provider_for(&server)
        .search(base_request())
        .await
        .expect_err("should error");
    assert!(matches!(err, SearchProviderError::Unauthorized));
}

#[tokio::test]
async fn returns_unauthorized_on_http_403() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    let err = provider_for(&server)
        .search(base_request())
        .await
        .expect_err("should error");
    assert!(matches!(err, SearchProviderError::Unauthorized));
}

#[tokio::test]
async fn returns_upstream_on_http_5xx_with_status_in_message() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let err = provider_for(&server)
        .search(base_request())
        .await
        .expect_err("should error");
    match err {
        SearchProviderError::Upstream(msg) => assert!(msg.contains("503"), "msg: {msg}"),
        other => panic!("expected Upstream, got {other:?}"),
    }
}

#[tokio::test]
async fn returns_upstream_on_malformed_json_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not a json"))
        .mount(&server)
        .await;
    let err = provider_for(&server)
        .search(base_request())
        .await
        .expect_err("should error");
    match err {
        SearchProviderError::Upstream(msg) => assert!(msg.contains("parse")),
        other => panic!("expected Upstream, got {other:?}"),
    }
}

#[tokio::test]
async fn returns_empty_vec_when_api_returns_zero_items() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;
    let raws = provider_for(&server)
        .search(base_request())
        .await
        .expect("ok");
    assert!(raws.is_empty());
}

#[tokio::test]
async fn builds_query_string_with_required_google_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .and(query_param("key", "test-key"))
        .and(query_param("cx", "test-cx"))
        .and(query_param("q", "rust async traits"))
        .and(query_param("num", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": []})))
        .mount(&server)
        .await;

    provider_for(&server)
        .search(base_request())
        .await
        .expect("mock accepts only matching params");
}

#[tokio::test]
async fn adds_hl_when_language_is_set() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .and(query_param("hl", "pt-BR"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": []})))
        .mount(&server)
        .await;

    let request = SearchRequest {
        language: Some("pt-BR".to_string()),
        ..base_request()
    };
    provider_for(&server)
        .search(request)
        .await
        .expect("hl must be forwarded");
}

#[tokio::test]
async fn adds_site_search_when_site_is_set() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/customsearch/v1"))
        .and(query_param("siteSearch", "github.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items": []})))
        .mount(&server)
        .await;

    let request = SearchRequest {
        site: Some("github.com".to_string()),
        ..base_request()
    };
    provider_for(&server)
        .search(request)
        .await
        .expect("siteSearch must be forwarded");
}

#[tokio::test]
async fn network_error_maps_to_network_variant() {
    // Point the provider at a URL guaranteed to fail transport (localhost:1).
    let provider = GoogleProvider::new(RedactedSecret::new("k"), "cx".to_string())
        .with_base_url("http://127.0.0.1:1");
    let err = provider.search(base_request()).await.expect_err("should error");
    assert!(
        matches!(err, SearchProviderError::Network(_)),
        "expected Network, got {err:?}"
    );
}

/// Live test, opt-in: only runs with `--features integration-test` AND when
/// real credentials are present in the environment. In all other runs the
/// test is skipped silently so CI and contributors without keys stay green.
#[cfg(feature = "integration-test")]
#[tokio::test]
async fn live_search_returns_nonempty_results_for_stable_query() {
    use crate::infrastructure::tools::constants::{
        ENV_WEB_SEARCH_API_KEY, ENV_WEB_SEARCH_CX,
    };

    let (Ok(key), Ok(cx)) = (
        std::env::var(ENV_WEB_SEARCH_API_KEY),
        std::env::var(ENV_WEB_SEARCH_CX),
    ) else {
        eprintln!("SKIP: live web_search test needs both env vars set");
        return;
    };

    let provider = GoogleProvider::new(RedactedSecret::new(key), cx);
    let out = provider
        .search(SearchRequest {
            query: "rust programming language".to_string(),
            num_results: 3,
            language: None,
            site: None,
        })
        .await
        .expect("live call should succeed");
    assert!(!out.is_empty(), "live query returned no results");
}
