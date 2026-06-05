//! Tool-level tests for the locale fallback. Pure resolver behavior is
//! covered by `locale.rs`; this file verifies how `execute_inner` threads a
//! resolved `hl` through to the provider request.

use std::sync::{Arc, MutexGuard};

use serde_json::json;

use crate::infrastructure::tools::constants::{web_search_test_env_lock, ENV_WEB_SEARCH_ENABLED};
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

fn one_raw() -> Vec<RawResult> {
    vec![RawResult {
        title: "t".to_string(),
        url: "https://example.com/".to_string(),
        snippet_html: "s".to_string(),
    }]
}

fn deps_with_locale(provider: Arc<dyn SearchProvider>, locale: Option<&str>) -> ExecuteDeps {
    ExecuteDeps {
        provider,
        policy: Policy::empty(),
        cache: None,
        workspace_key: String::new(),
        locale: Arc::new(FixedLocale(locale.map(String::from))),
    }
}

#[tokio::test]
async fn execute_passes_system_locale_when_args_language_is_none() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(one_raw()));
    let deps = deps_with_locale(provider.clone(), Some("en-US"));
    let _ = execute_inner_for_tests(deps, json!({ "query": "rust" }))
        .await
        .expect("ok");
    let observed = provider.observed_requests();
    assert_eq!(observed[0].language.as_deref(), Some("en-us"));
}

#[tokio::test]
async fn execute_prefers_explicit_language_over_system_locale() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(one_raw()));
    let deps = deps_with_locale(provider.clone(), Some("en-US"));
    let _ = execute_inner_for_tests(deps, json!({ "query": "rust", "language": "pt-BR" }))
        .await
        .expect("ok");
    let observed = provider.observed_requests();
    assert_eq!(
        observed[0].language.as_deref(),
        Some("pt-br"),
        "explicit language wins, normalized to lowercase"
    );
}

#[tokio::test]
async fn execute_omits_hl_when_no_language_and_no_system_locale() {
    let _guard = flag_on();
    let provider = Arc::new(MockProvider::with_results(one_raw()));
    let deps = deps_with_locale(provider.clone(), None);
    let _ = execute_inner_for_tests(deps, json!({ "query": "rust" }))
        .await
        .expect("ok");
    let observed = provider.observed_requests();
    assert_eq!(observed[0].language, None);
}
