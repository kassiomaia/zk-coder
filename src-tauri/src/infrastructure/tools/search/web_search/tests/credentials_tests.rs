//! Resolver tests for `web_search/credentials.rs`. The resolver is the single
//! gate between the `tool_credentials` row and a usable `GoogleProvider`, so
//! we freeze its behavior across all failure modes here.

use crate::core::config::ToolCredentialsRepository;
use crate::infrastructure::sqlite::repositories::config::SqliteConfigRepository;
use crate::infrastructure::tools::search::web_search::credentials::{
    resolve, serialize, StoredCredentials, WEB_SEARCH_TOOL_ID,
};
use crate::infrastructure::tools::search::web_search::error::{ErrorCode, WebSearchFailure};

fn store() -> SqliteConfigRepository {
    SqliteConfigRepository::in_memory().expect("in-memory config store")
}

fn store_blob(repo: &SqliteConfigRepository, blob: &str) {
    repo.set_tool_credentials(WEB_SEARCH_TOOL_ID, blob)
        .expect("set_tool_credentials");
}

#[test]
fn returns_credentials_when_row_present_and_json_valid() {
    let repo = store();
    let blob = serialize(&StoredCredentials {
        api_key: "sk-abc".to_string(),
        cx: "cx-123".to_string(),
    })
    .expect("serialize");
    store_blob(&repo, &blob);

    let resolved = resolve(&repo).expect("resolve ok");
    assert_eq!(resolved.api_key.expose(), "sk-abc");
    assert_eq!(resolved.cx, "cx-123");
}

#[test]
fn returns_not_configured_when_row_absent() {
    let repo = store();
    let err = resolve(&repo).expect_err("absent row must fail");
    assert!(matches!(err, WebSearchFailure::NotConfigured));
    assert_eq!(err.code(), ErrorCode::NotConfigured);
}

#[test]
fn returns_not_configured_on_invalid_json_blob() {
    let repo = store();
    store_blob(&repo, "this is not json");
    let err = resolve(&repo).expect_err("invalid json must fail");
    assert!(matches!(err, WebSearchFailure::NotConfigured));
}

#[test]
fn returns_not_configured_when_api_key_is_empty() {
    let repo = store();
    store_blob(&repo, r#"{"api_key":"","cx":"cx-123"}"#);
    let err = resolve(&repo).expect_err("empty api_key must fail");
    assert!(matches!(err, WebSearchFailure::NotConfigured));
}

#[test]
fn returns_not_configured_when_cx_is_empty() {
    let repo = store();
    store_blob(&repo, r#"{"api_key":"sk-abc","cx":""}"#);
    let err = resolve(&repo).expect_err("empty cx must fail");
    assert!(matches!(err, WebSearchFailure::NotConfigured));
}

#[test]
fn trims_whitespace_from_both_fields() {
    let repo = store();
    store_blob(&repo, r#"{"api_key":"   sk-abc  ","cx":"  cx-123   "}"#);
    let resolved = resolve(&repo).expect("trimmed values are accepted");
    assert_eq!(resolved.api_key.expose(), "sk-abc");
    assert_eq!(resolved.cx, "cx-123");
}

#[test]
fn whitespace_only_credentials_are_rejected() {
    let repo = store();
    store_blob(&repo, r#"{"api_key":"   ","cx":"cx-123"}"#);
    let err = resolve(&repo).expect_err("whitespace-only api_key must fail");
    assert!(matches!(err, WebSearchFailure::NotConfigured));
}

#[test]
fn missing_json_fields_rejected_as_not_configured() {
    let repo = store();
    // Blob missing `cx`. Serde rejects; we map to NotConfigured.
    store_blob(&repo, r#"{"api_key":"sk-abc"}"#);
    let err = resolve(&repo).expect_err("missing field must fail");
    assert!(matches!(err, WebSearchFailure::NotConfigured));
}

#[test]
fn resolved_api_key_never_leaks_via_debug_or_display() {
    let repo = store();
    store_blob(&repo, r#"{"api_key":"hunter2","cx":"cx-123"}"#);
    let resolved = resolve(&repo).expect("ok");
    // Defensive: cross-check that RedactedSecret still redacts after a round
    // trip through the resolver.
    let redacted_display = format!("{}", resolved.api_key);
    let redacted_debug = format!("{:?}", resolved.api_key);
    assert!(!redacted_display.contains("hunter2"));
    assert!(!redacted_debug.contains("hunter2"));
}
