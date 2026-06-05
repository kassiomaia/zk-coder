//! Pure argument-validation tests for `WebSearchArgs`.
//!
//! No HTTP, no DB, no Tauri runtime.

use serde_json::json;

use crate::infrastructure::tools::constants::{
    DEFAULT_WEB_SEARCH_LIMIT, MAX_WEB_SEARCH_LIMIT, MAX_WEB_SEARCH_QUERY_LEN,
};
use crate::infrastructure::tools::error::{parse_args, ToolError};
use crate::infrastructure::tools::search::web_search::types::WebSearchArgs;

/// Parses a JSON value through the canonical tool entry point and runs the
/// same validation `WebSearchTool::execute` will invoke.
fn parse_and_validate(value: serde_json::Value) -> Result<WebSearchArgs, ToolError> {
    let raw: WebSearchArgs = parse_args("search_web_search", value)?;
    raw.into_validated()
}

#[test]
fn parses_minimal_args_with_only_query() {
    let args = parse_and_validate(json!({ "query": "rust async traits" })).expect("valid");
    assert_eq!(args.query, "rust async traits");
    assert_eq!(args.num_results, DEFAULT_WEB_SEARCH_LIMIT);
    assert_eq!(args.language, None);
    assert_eq!(args.site, None);
}

#[test]
fn applies_default_num_results_when_missing() {
    let args = parse_and_validate(json!({ "query": "svelte 5 runes" })).expect("valid");
    assert_eq!(args.num_results, DEFAULT_WEB_SEARCH_LIMIT);
}

#[test]
fn rejects_empty_query_after_trim() {
    let err =
        parse_and_validate(json!({ "query": "   \t  " })).expect_err("whitespace-only should fail");
    match err {
        ToolError::InvalidArguments(msg) => assert!(msg.contains("non-empty")),
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
}

#[test]
fn rejects_query_longer_than_max_web_search_query_len() {
    let oversized: String = "a".repeat(MAX_WEB_SEARCH_QUERY_LEN + 1);
    let err = parse_and_validate(json!({ "query": oversized })).expect_err("oversize must fail");
    match err {
        ToolError::InvalidArguments(msg) => {
            assert!(
                msg.contains(&MAX_WEB_SEARCH_QUERY_LEN.to_string()),
                "error should mention the limit: {msg}"
            );
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
}

#[test]
fn accepts_query_exactly_at_max_length() {
    let boundary: String = "a".repeat(MAX_WEB_SEARCH_QUERY_LEN);
    let args = parse_and_validate(json!({ "query": boundary.clone() })).expect("valid");
    assert_eq!(args.query.len(), MAX_WEB_SEARCH_QUERY_LEN);
    assert_eq!(args.query, boundary);
}

#[test]
fn clamps_num_results_to_1_10_range() {
    let too_low = parse_and_validate(json!({ "query": "rust", "num_results": 0 })).expect("valid");
    assert_eq!(too_low.num_results, 1, "zero clamps up to 1");

    let too_high =
        parse_and_validate(json!({ "query": "rust", "num_results": 99 })).expect("valid");
    assert_eq!(
        too_high.num_results, MAX_WEB_SEARCH_LIMIT,
        "excess clamps down to the provider max"
    );

    let in_range = parse_and_validate(json!({ "query": "rust", "num_results": 7 })).expect("valid");
    assert_eq!(in_range.num_results, 7, "in-range values are preserved");
}

#[test]
fn accepts_optional_language_tag() {
    let args = parse_and_validate(json!({
        "query": "svelte",
        "language": "pt-BR",
    }))
    .expect("valid");
    assert_eq!(args.language.as_deref(), Some("pt-BR"));
}

#[test]
fn normalizes_empty_language_to_none() {
    let args = parse_and_validate(json!({ "query": "svelte", "language": "   " })).expect("valid");
    assert_eq!(args.language, None);
}

#[test]
fn accepts_optional_site_restriction() {
    let args = parse_and_validate(json!({
        "query": "tauri",
        "site": "github.com",
    }))
    .expect("valid");
    assert_eq!(args.site.as_deref(), Some("github.com"));
}

#[test]
fn rejects_unknown_additional_property() {
    let err = parse_and_validate(json!({
        "query": "svelte",
        "mystery": true,
    }))
    .expect_err("unknown keys must be rejected");
    match err {
        ToolError::InvalidArguments(msg) => {
            assert!(
                msg.to_lowercase().contains("mystery")
                    || msg.to_lowercase().contains("unknown field"),
                "error should surface the unknown key: {msg}"
            );
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
}

#[test]
fn query_is_trimmed_in_place() {
    let args = parse_and_validate(json!({ "query": "   hello world   " })).expect("valid");
    assert_eq!(args.query, "hello world");
}

#[test]
fn counts_query_length_in_characters_not_bytes() {
    // 512 é́ composed codepoints ≈ a reasonable multibyte check. We pick a
    // non-ASCII character so the byte length would be > 512 but the char
    // count sits exactly at MAX_WEB_SEARCH_QUERY_LEN.
    let multibyte: String = "é".repeat(MAX_WEB_SEARCH_QUERY_LEN);
    let args = parse_and_validate(json!({ "query": multibyte.clone() })).expect("valid");
    assert_eq!(args.query.chars().count(), MAX_WEB_SEARCH_QUERY_LEN);

    let oversized: String = "é".repeat(MAX_WEB_SEARCH_QUERY_LEN + 1);
    assert!(
        parse_and_validate(json!({ "query": oversized })).is_err(),
        "one-over-the-limit must fail"
    );
}
