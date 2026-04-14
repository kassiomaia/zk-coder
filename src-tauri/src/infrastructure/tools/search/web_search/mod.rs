//! Production web search tool backed by the Google Custom Search JSON API.
//!
//! The feature contract lives in `docs/tools/web_search/spec.md`. The
//! original TDD implementation plan has been archived at
//! `docs/archive/tools/web_search/plan.md`.
//!
//! Phase 3 lands the full `Tool` impl against the injected [`SearchProvider`]
//! seam. Real HTTP, DB credentials, workspace policy, and result caching are
//! bolted on in later phases through the same entry point — this file's
//! structure does not need to change when they arrive.

pub mod cache;
pub mod credentials;
pub mod error;
pub mod locale;
pub mod policy;
pub mod provider;
pub mod sanitize;
pub mod secret;
pub mod types;
pub mod workspace_key;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value as JsonValue;
use tauri::{AppHandle, Manager};

use crate::commands::ConfigState;
use crate::core::config::WebSearchCacheRepository;
use crate::infrastructure::tools::constants::{is_env_enabled, ENV_WEB_SEARCH_ENABLED};
use crate::infrastructure::tools::context::ToolExecutionContext;
use crate::infrastructure::tools::error::{parse_args, ToolError};
use crate::infrastructure::tools::traits::{Tool, ToolFuture, ToolId, ToolMeta};

use self::cache::{cache_key, CacheHandle, CacheKeyInput, DEFAULT_CACHE_TTL_SECS};
use self::error::WebSearchFailure;
use self::locale::{resolve_hl, LocaleProvider, SystemLocale};
use self::policy::Policy;
use self::provider::google::GoogleProvider;
use self::provider::{RawResult, SearchProvider, SearchRequest};
use self::sanitize::{sanitize_snippet, source_domain, strip_tracking_params};
use self::types::{
    WebSearchArgs, WebSearchOutput, WebSearchResult, PROVIDER_GOOGLE_CUSTOM_SEARCH,
};
use self::workspace_key::canonical_workspace_key;

/// Dependency bundle passed into [`execute_inner`]. Makes it possible to
/// layer on future concerns (credentials resolver, telemetry sink) without
/// growing the function signature. All fields are owned values or `Arc`s so
/// the bundle is cheap to clone into the async closure of `Tool::execute`.
pub struct ExecuteDeps {
    /// Search backend. Phase 4 `GoogleProvider` in production, `MockProvider`
    /// in tests.
    pub provider: Arc<dyn SearchProvider>,
    /// Workspace domain policy. `Policy::empty()` means no filter.
    pub policy: Policy,
    /// Optional result cache. When `None`, every call hits the provider.
    pub cache: Option<CacheHandle>,
    /// Canonical workspace path. Empty string = global bucket.
    pub workspace_key: String,
    /// Provider of the host system locale (for `hl` fallback). In production
    /// this is `SystemLocale`; tests usually inject a `FixedLocale`.
    pub locale: Arc<dyn LocaleProvider>,
}

impl ExecuteDeps {
    /// Minimal dependency bundle used by most offline tests. Defaults to
    /// "no system locale" so tests that don't exercise the `hl` fallback
    /// stay deterministic regardless of host configuration.
    #[cfg(test)]
    pub(crate) fn test_minimal(provider: Arc<dyn SearchProvider>) -> Self {
        Self {
            provider,
            policy: Policy::empty(),
            cache: None,
            workspace_key: String::new(),
            locale: Arc::new(self::locale::FixedLocale(None)),
        }
    }
}

/// Production-grade web search tool.
///
/// Stateless on purpose: credentials, workspace policy, the cache handle,
/// and the locale provider are all resolved from `ConfigState` on every
/// `execute` call. This means Settings-UI changes (rotated API key, updated
/// allow/deny lists, cleared cache) take effect on the next tool call
/// without re-registering the tool.
pub struct WebSearchTool;

impl WebSearchTool {
    /// Constructs a production-mode tool. Zero work at construction time —
    /// everything happens per-call in [`build_production_deps`].
    #[must_use]
    pub const fn new_production() -> Self {
        Self
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new_production()
    }
}

impl Tool for WebSearchTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("search", "web_search"),
            description: "Searches the public web via the Google Custom Search JSON API. \
                          Returns normalized title/url/snippet/source_domain results. \
                          Gated by the CODER_LLM_TOOL_WEB_SEARCH flag and DB-stored credentials.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string.",
                        "minLength": 1,
                        "maxLength": 512
                    },
                    "num_results": {
                        "type": "integer",
                        "description": "Number of results to return (default 5, max 10).",
                        "minimum": 1,
                        "maximum": 10
                    },
                    "language": {
                        "type": "string",
                        "description": "Optional BCP-47 language tag (e.g., 'en', 'pt-BR'). \
                                        Defaults to the host system locale."
                    },
                    "site": {
                        "type": "string",
                        "description": "Optional site restriction (e.g., 'github.com'). \
                                        Must pass the active workspace's domain policy."
                    }
                },
                "required": ["query"]
            }),
            requires_workspace: false,
            experimental: false,
            feature_flag: Some(ENV_WEB_SEARCH_ENABLED),
        }
    }

    fn execute(
        &self,
        _ctx: ToolExecutionContext,
        arguments: JsonValue,
        app_handle: AppHandle,
        workspace_root: Option<std::path::PathBuf>,
    ) -> ToolFuture {
        // Resolve all dependencies from the current app state. Any failure
        // here (missing credentials) is surfaced to the LLM/frontend as a
        // structured error body with a stable `error_code`.
        let deps_result = build_production_deps(&app_handle, workspace_root.as_deref());
        Box::pin(async move {
            let started = Instant::now();
            let result = match deps_result {
                Ok(deps) => execute_inner(deps, arguments).await,
                Err(failure) => Err(failure),
            };
            let elapsed_ms = started.elapsed().as_millis();

            match result {
                Ok(output) => {
                    log::info!(
                        target: "web_search",
                        "search ok query={:?} count={} cached={} elapsed_ms={}",
                        output.query,
                        output.count,
                        output.cached,
                        elapsed_ms
                    );
                    Ok(serde_json::to_value(output).map_err(ToolError::from)?)
                }
                Err(failure) => {
                    let body = failure.into_body();
                    log::warn!(
                        target: "web_search",
                        "search failed error_code={} elapsed_ms={}",
                        body.error_code.as_str(),
                        elapsed_ms
                    );
                    Ok(serde_json::to_value(body).map_err(ToolError::from)?)
                }
            }
        })
    }
}

/// Builds a fully wired [`ExecuteDeps`] from the live app state. Resolves
/// credentials, looks up the workspace policy, and constructs the cache
/// handle against the SQLite-backed repository.
fn build_production_deps(
    app_handle: &AppHandle,
    workspace_root: Option<&Path>,
) -> Result<ExecuteDeps, WebSearchFailure> {
    let config_state = app_handle.state::<ConfigState>();
    let repo = config_state.repository.clone();

    // Credentials → GoogleProvider. Missing / malformed row surfaces as
    // `not_configured` via `WebSearchFailure::NotConfigured`.
    let resolved = credentials::resolve(&*repo)?;
    let provider: Arc<dyn SearchProvider> =
        Arc::new(GoogleProvider::new(resolved.api_key, resolved.cx));

    // Workspace key + policy.
    let workspace_key = canonical_workspace_key(workspace_root);
    let stored_policy = repo
        .get_workspace_search_policy(&workspace_key)
        .ok()
        .flatten();
    let policy = Policy::from_stored(stored_policy.as_ref());

    // Cache — `ConfigRepository` super-traits `WebSearchCacheRepository`, so
    // the Arc upcasts cleanly on current Rust.
    let cache_repo: Arc<dyn WebSearchCacheRepository> = repo;
    let cache = Some(CacheHandle {
        repo: cache_repo,
        ttl: Duration::from_secs(DEFAULT_CACHE_TTL_SECS),
    });

    Ok(ExecuteDeps {
        provider,
        policy,
        cache,
        workspace_key,
        locale: Arc::new(SystemLocale),
    })
}

/// Pure (aside from the provider call) implementation of the tool's business
/// logic. Split out so unit tests can compose it with `MockProvider` and a
/// synthetic `Policy` / `CacheHandle` without reaching through the Tauri
/// `Tool` trait.
pub(crate) async fn execute_inner(
    deps: ExecuteDeps,
    arguments: JsonValue,
) -> Result<WebSearchOutput, WebSearchFailure> {
    let ExecuteDeps {
        provider,
        policy,
        cache,
        workspace_key,
        locale,
    } = deps;

    // 1. Feature flag (belt + suspenders: registry also enforces this, but the
    //    tool checks again so direct callers and tests get a structured code).
    if !is_env_enabled(ENV_WEB_SEARCH_ENABLED) {
        return Err(WebSearchFailure::DisabledByFlag);
    }

    // 2. Parse and validate arguments.
    let raw: WebSearchArgs = parse_args("search_web_search", arguments)
        .map_err(|e| WebSearchFailure::InvalidArguments(e.to_string()))?;
    let args = raw
        .into_validated()
        .map_err(|e| WebSearchFailure::InvalidArguments(e.to_string()))?;

    // 3. Policy: an explicit `site` argument must survive the allow/deny lists.
    if let Some(site) = &args.site {
        if !policy.permits_site(site) {
            return Err(WebSearchFailure::FilteredByPolicy(format!(
                "Site '{site}' is not permitted by this workspace's policy."
            )));
        }
    }

    // 4. Resolve the language tag. An explicit `language` argument wins;
    //    otherwise we fall back to the host system locale. The resolved tag
    //    drives both the provider request and the cache key, so two calls
    //    that would have produced identical Google queries hit the same
    //    cache row regardless of how the language was supplied.
    let resolved_language = resolve_hl(args.language.clone(), locale.as_ref());

    // 5. Compute the cache key. Keying on post-validation args keeps the cache
    //    stable across redundant whitespace in user queries while still
    //    partitioning per workspace (see spec.md §4.10).
    let key_input = CacheKeyInput {
        query: &args.query,
        num_results: args.num_results,
        language: resolved_language.as_deref(),
        site: args.site.as_deref(),
        workspace_key: &workspace_key,
    };
    let key = cache_key(&key_input);

    // 5. Cache read. Policy is re-applied on read so a tighter policy drops
    //    entries that were originally stored when they were permitted.
    if let Some(handle) = cache.as_ref() {
        if let Some(cached_results) = handle.read(&key) {
            let (filtered, dropped_by_policy) = policy.filter_results(cached_results);
            let capped: Vec<WebSearchResult> =
                filtered.into_iter().take(args.num_results).collect();
            return Ok(WebSearchOutput {
                query: args.query,
                count: capped.len(),
                results: capped,
                provider: PROVIDER_GOOGLE_CUSTOM_SEARCH,
                cached: true,
                filtered_by_policy: if dropped_by_policy { Some(true) } else { None },
            });
        }
    }

    // 6. When no `site` is given, append a `site:` OR-expansion of the
    //    allowlist so the provider searches the permitted subset directly.
    //    With an explicit site the user already narrowed the scope.
    let provider_query = if args.site.is_none() {
        match policy.site_rewrite_query_suffix() {
            Some(suffix) => format!("{} {suffix}", args.query),
            None => args.query.clone(),
        }
    } else {
        args.query.clone()
    };

    // 7. Issue the provider request using the resolved language.
    let request = SearchRequest {
        query: provider_query,
        num_results: args.num_results,
        language: resolved_language.clone(),
        site: args.site.clone(),
    };
    let raws = provider.search(request).await?;

    // 8. Sanitize + reshape. Results that lack a usable domain are dropped.
    let shaped: Vec<WebSearchResult> = raws.into_iter().filter_map(normalize_raw_result).collect();

    // 9. Apply the policy filter, then cap to the requested size.
    let (filtered, dropped_by_policy) = policy.filter_results(shaped);
    let capped: Vec<WebSearchResult> = filtered.into_iter().take(args.num_results).collect();

    // 10. Persist the cache entry (post-policy-filter). Failures are logged
    //     but swallowed — a cache-put failure must not break the tool path.
    if let Some(handle) = cache.as_ref() {
        handle.write(&key, &key_input, PROVIDER_GOOGLE_CUSTOM_SEARCH, &capped);
    }

    Ok(WebSearchOutput {
        query: args.query,
        count: capped.len(),
        results: capped,
        provider: PROVIDER_GOOGLE_CUSTOM_SEARCH,
        cached: false,
        filtered_by_policy: if dropped_by_policy { Some(true) } else { None },
    })
}

fn normalize_raw_result(raw: RawResult) -> Option<WebSearchResult> {
    let url = strip_tracking_params(&raw.url);
    let source_domain = source_domain(&url)?;
    Some(WebSearchResult {
        title: sanitize_snippet(&raw.title),
        url,
        snippet: sanitize_snippet(&raw.snippet_html),
        source_domain,
    })
}

#[cfg(test)]
pub(crate) use execute_inner as execute_inner_for_tests;
