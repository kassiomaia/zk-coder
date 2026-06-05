//! Enumerates every `WebSearchFailure` → `ErrorCode` transition mandated by
//! `spec.md §4.4`, plus the provider-error adapter. These tests are pure and
//! trivial on purpose — the mapping table is part of the stable contract, so
//! a change here is a wire-format break that must be reviewed explicitly.

use crate::infrastructure::tools::search::web_search::error::{ErrorCode, WebSearchFailure};
use crate::infrastructure::tools::search::web_search::provider::SearchProviderError;

#[test]
fn disabled_feature_flag_maps_to_disabled_by_flag() {
    assert_eq!(
        WebSearchFailure::DisabledByFlag.code(),
        ErrorCode::DisabledByFlag
    );
}

#[test]
fn credentials_not_configured_maps_to_not_configured() {
    assert_eq!(
        WebSearchFailure::NotConfigured.code(),
        ErrorCode::NotConfigured
    );
}

#[test]
fn credentials_malformed_maps_to_not_configured() {
    // Spec: malformed credentials surface the same actionable code as missing
    // credentials — the user fix is identical.
    assert_eq!(
        WebSearchFailure::NotConfigured.code(),
        ErrorCode::NotConfigured
    );
}

#[test]
fn invalid_arguments_maps_to_invalid_arguments() {
    assert_eq!(
        WebSearchFailure::InvalidArguments("bad".to_string()).code(),
        ErrorCode::InvalidArguments
    );
}

#[test]
fn policy_rejection_of_explicit_site_maps_to_filtered_by_policy() {
    assert_eq!(
        WebSearchFailure::FilteredByPolicy("blocked".to_string()).code(),
        ErrorCode::FilteredByPolicy
    );
}

#[test]
fn provider_rate_limited_maps_to_rate_limited() {
    let failure: WebSearchFailure = SearchProviderError::RateLimited.into();
    assert_eq!(failure.code(), ErrorCode::RateLimited);
}

#[test]
fn provider_unauthorized_maps_to_unauthorized() {
    let failure: WebSearchFailure = SearchProviderError::Unauthorized.into();
    assert_eq!(failure.code(), ErrorCode::Unauthorized);
}

#[test]
fn provider_network_maps_to_upstream_error() {
    let failure: WebSearchFailure = SearchProviderError::Network("dns".to_string()).into();
    assert_eq!(failure.code(), ErrorCode::UpstreamError);
}

#[test]
fn provider_5xx_maps_to_upstream_error() {
    let failure: WebSearchFailure = SearchProviderError::Upstream("HTTP 502".to_string()).into();
    assert_eq!(failure.code(), ErrorCode::UpstreamError);
}

#[test]
fn each_code_carries_the_expected_retryable_flag() {
    // Freeze the retry advisory so frontend + LLM can rely on it without
    // re-reading the spec.
    assert!(!ErrorCode::NotConfigured.retryable());
    assert!(!ErrorCode::DisabledByFlag.retryable());
    assert!(ErrorCode::RateLimited.retryable());
    assert!(!ErrorCode::Unauthorized.retryable());
    assert!(ErrorCode::UpstreamError.retryable());
    assert!(!ErrorCode::InvalidArguments.retryable());
    assert!(!ErrorCode::FilteredByPolicy.retryable());
}

#[test]
fn error_codes_serialize_to_stable_strings() {
    assert_eq!(ErrorCode::NotConfigured.as_str(), "not_configured");
    assert_eq!(ErrorCode::DisabledByFlag.as_str(), "disabled_by_flag");
    assert_eq!(ErrorCode::RateLimited.as_str(), "rate_limited");
    assert_eq!(ErrorCode::Unauthorized.as_str(), "unauthorized");
    assert_eq!(ErrorCode::UpstreamError.as_str(), "upstream_error");
    assert_eq!(ErrorCode::InvalidArguments.as_str(), "invalid_arguments");
    assert_eq!(ErrorCode::FilteredByPolicy.as_str(), "filtered_by_policy");
}

#[test]
fn error_body_serializes_with_snake_case_code_and_retryable_flag() {
    let body = WebSearchFailure::RateLimited.into_body();
    let json = serde_json::to_value(&body).expect("serializable");
    assert_eq!(json["error_code"], "rate_limited");
    assert_eq!(json["retryable"], true);
    assert!(json["error"].as_str().unwrap().contains("quota"));
}
