# Web Search Tool — Implementation Plan (TDD)

**Status**: PLAN
**Last reviewed**: 2026-04-14
**Purpose**: Test-first, phase-by-phase plan for shipping `search_web_search`. Each step lists the failing test(s) to write before any production code is touched.

Read `spec.md` and `investigation.md` first. Do not start Phase 1 until the Open Questions in `spec.md §9` have been answered.

**Visual reference**: `mockups.html` (open directly in a browser). Every Svelte component below has a numbered panel in the mockup (panels 01–09). When implementing, open the panel and lift class names, spacing, and structure straight into the component. The mockup's design tokens are a subset of the production `design-system.css`, so token substitution is one-for-one — see Appendix B ("Component specifications") for the full mapping.

---

## Ground rules

1. **Red → green → refactor**, per step. No production code without a failing test in the same commit (or preceding commit on the same branch).
2. **One concern per commit**. The steps below are commit boundaries.
3. **File size discipline**: no file in `src-tauri/src/infrastructure/tools/search/web_search/` exceeds 300 lines while the feature is being built; the soft repo limit is 500.
4. **Run locally before each push**: `cargo test -p app --lib`, `cargo clippy -- -D warnings`, `cd src-ui && npm run check`, `cd src-ui && npx vitest run`.
5. **Env var for live tests**: the HTTP-touching tests are behind the existing `integration-test` cargo feature (`cargo test --features integration-test`). They are not run in CI by default; we add them explicitly to the plan's exit criteria.

---

## Phase 0 — Alignment (prerequisite)

- [x] Open questions resolved — see `spec.md §9` Resolution Log (2026-04-14).
- [x] Promote `spec.md` status DRAFT → PLAN (kept DRAFT until this plan is also promoted).
- [x] Add new crate dependencies to `src-tauri/Cargo.toml`:
  - `async-trait = "0.1"` — for the `SearchProvider` trait.
  - `sys-locale = "0.3"` — for system locale detection.
  - `sha2 = "0.10"` — for the cache key hash.
  - `tauri-plugin-opener = "2"` under `[dependencies]` and registered the plugin in `lib.rs` with `http(s)://*` capability scope.
  - `wiremock = "0.6"` under `[dev-dependencies]` — for offline Google provider tests.
- [x] Create a feature branch `feature/web-search-tool`.
- [x] Register this plan: added a _planned_ row in `docs/tools-and-agent-integration.md` and a changelog entry under Unreleased/Added.

Exit: dependencies added, branch created, spec promoted.

---

## Phase 1 — Types and pure argument validation

Goal: have the `WebSearchArgs` / `WebSearchOutput` / `WebSearchResult` structs compile and be validated, without any HTTP or DB code.

### Tests first (all unit, no HTTP, no DB)

Place in `src-tauri/src/infrastructure/tools/search/web_search/tests/args_tests.rs`:

- `parses_minimal_args_with_only_query`
- `applies_default_num_results_when_missing`
- `rejects_empty_query_after_trim`
- `rejects_query_longer_than_MAX_WEB_SEARCH_QUERY_LEN`
- `clamps_num_results_to_1_10_range`
- `accepts_optional_language_tag`
- `accepts_optional_site_restriction`
- `rejects_unknown_additional_property` (via `additionalProperties: false`)

### Code

- `web_search/mod.rs` — empty skeleton exposing `WebSearchTool` with `todo!()` in execute (not to be merged; just lets the tests compile; replaced in Phase 3).
- `web_search/types.rs` — `WebSearchArgs`, `WebSearchResult`, `WebSearchOutput`, `default_num_results`, constants added to `constants.rs`.

### Constants delta

In `src-tauri/src/infrastructure/tools/constants.rs`:

```rust
pub const MAX_WEB_SEARCH_LIMIT: usize = 10;
pub const MAX_WEB_SEARCH_QUERY_LEN: usize = 512;
pub const ENV_WEB_SEARCH_ENABLED: &str = "CODER_LLM_TOOL_WEB_SEARCH";
pub const ENV_WEB_SEARCH_API_KEY: &str = "CODER_LLM_TOOL_WEB_SEARCH_API_KEY";
pub const ENV_WEB_SEARCH_CX: &str = "CODER_LLM_TOOL_WEB_SEARCH_CX";
```

Exit: `cargo test -p app --lib` green; `cargo clippy` clean.

---

## Phase 2 — Provider trait and mock

Goal: define the seam that makes the tool provider-agnostic and testable offline.

### Tests first

In `web_search/tests/provider_mock_tests.rs`:

- `mock_provider_returns_canned_results`
- `mock_provider_records_observed_requests`
- `mock_provider_returns_rate_limit_error_on_demand`
- `mock_provider_returns_network_error_on_demand`

### Code

`web_search/provider/mod.rs`:

```rust
#[async_trait::async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, req: SearchRequest) -> Result<Vec<RawResult>, SearchProviderError>;
}

pub struct SearchRequest { pub query: String, pub num_results: usize,
                           pub language: Option<String>, pub site: Option<String> }
pub struct RawResult { pub title: String, pub url: String, pub snippet_html: String }
#[derive(Debug, thiserror::Error)]
pub enum SearchProviderError { RateLimited, Unauthorized, Upstream(String), Network(String) }
```

Place `MockProvider` under `#[cfg(test)]` in `provider/mod.rs` (or a dedicated `test_support` module if it grows).

Exit: trait compiles, mock passes its tests, no production impact yet.

---

## Phase 3 — `WebSearchTool::execute` wired to a provider (no real HTTP)

Goal: full `Tool` impl covered by tests using `MockProvider`.

### Tests first

In `web_search/tests/tool_execute_tests.rs`:

- `execute_returns_disabled_error_when_feature_flag_off`
- `execute_returns_invalid_args_on_empty_query`
- `execute_returns_invalid_args_on_query_too_long`
- `execute_strips_html_from_snippets`
- `execute_decodes_entities_in_snippets`
- `execute_removes_utm_tracking_params_from_urls`
- `execute_derives_source_domain_from_url`
- `execute_clamps_num_results_to_max_10`
- `execute_returns_structured_error_on_provider_rate_limit`
- `execute_returns_structured_error_on_provider_unauthorized`
- `execute_returns_empty_results_as_success`
- `execute_passes_site_restriction_to_provider`
- `execute_passes_language_to_provider`
- `execute_output_excludes_html_and_tracking_params`
- `execute_sets_filtered_by_policy_when_any_result_is_dropped` (covered end-to-end once Phase 5a lands; stubbed with a passthrough policy in this phase)

Plus a dedicated mapping test file — `web_search/tests/error_mapping_tests.rs` — enumerating every `ToolError` → `error_code` transition declared in `spec.md §4.4`:

- `disabled_feature_flag_maps_to_disabled_by_flag`
- `credentials_not_configured_maps_to_not_configured`
- `credentials_malformed_maps_to_not_configured`
- `invalid_arguments_maps_to_invalid_arguments`
- `policy_rejection_of_explicit_site_maps_to_filtered_by_policy`
- `provider_rate_limited_maps_to_rate_limited`
- `provider_unauthorized_maps_to_unauthorized`
- `provider_network_maps_to_upstream_error`
- `provider_5xx_maps_to_upstream_error`
- `each_code_carries_the_expected_retryable_flag`

All tests construct a `WebSearchTool` with an injected `Arc<dyn SearchProvider>` using the mock. The feature flag is set by the test with `std::env::set_var` and removed in teardown (same pattern as `google_scrape.rs`).

### Code

- `web_search/mod.rs`:
  - `pub struct WebSearchTool { provider: Arc<dyn SearchProvider> }`
  - `pub fn new_with_provider(provider: Arc<dyn SearchProvider>) -> Self` (for tests and registry)
  - `impl Tool for WebSearchTool` — full implementation.
- `web_search/sanitize.rs` — tiny helpers: `strip_html`, `decode_entities`, `strip_tracking_params`, `source_domain`. Pure; reuse logic from `google_scrape.rs` (consider extracting to a shared `infrastructure/tools/html.rs` if worthwhile — a focused refactor commit, separate from this feature).

Exit: offline test suite green, covers every failure branch.

---

## Phase 4 — Google provider (real reqwest client)

Goal: `GoogleProvider` issues real Custom Search JSON API calls and maps responses to `RawResult`.

### Tests first

In `web_search/tests/provider_google_tests.rs`:

Offline tests (always run):

- `maps_response_items_to_raw_results`
- `returns_rate_limited_on_http_429`
- `returns_unauthorized_on_http_401_or_403`
- `returns_upstream_on_http_5xx_with_status_in_message`
- `returns_empty_vec_when_api_returns_zero_items`
- `builds_query_string_with_required_google_params` (verify `key`, `cx`, `q`, `num` are all present)
- `adds_hl_when_language_is_set`
- `adds_site_search_when_site_is_set`

These use `wiremock` or hand-rolled `mockito` to stand in for `googleapis.com`. Add `wiremock` as a `[dev-dependencies]` entry if not present.

Live tests (`#[cfg(all(test, feature = "integration-test"))]`):

- `live_search_returns_nonempty_results_for_stable_query` — skipped unless `CODER_LLM_TOOL_WEB_SEARCH_API_KEY` and `CODER_LLM_TOOL_WEB_SEARCH_CX` are present in the env.

### Code

`web_search/provider/google.rs`:

```rust
pub struct GoogleProvider {
    client: reqwest::Client,
    api_key: SecretString,
    cx: String,
}

impl GoogleProvider {
    pub fn new(api_key: SecretString, cx: String) -> Self { /* default client */ }
    pub fn with_client(api_key: SecretString, cx: String, client: reqwest::Client) -> Self { ... }
}

#[async_trait::async_trait]
impl SearchProvider for GoogleProvider {
    async fn search(&self, req: SearchRequest) -> Result<Vec<RawResult>, SearchProviderError> {
        // GET https://www.googleapis.com/customsearch/v1
        // params: key, cx, q, num, (hl), (siteSearch)
        // timeout: 10s
        // map response -> RawResult (use `items` array; title/link/htmlSnippet)
    }
}
```

The `api_key` is held in a `SecretString` (crate `secrecy`) so it cannot accidentally be printed via `Debug`. If introducing `secrecy` is out of scope, use a small newtype `RedactedApiKey(String)` with a manual `Debug` impl that prints `"<redacted>"`.

Exit: offline tests green; one developer runs the live test locally with real creds as a smoke check.

---

## Phase 5 — Credentials: repository, migration, resolver (DB only)

Goal: persist `(api_key, cx)` in `coder_config.db` and load them at tool-execute time. No environment-variable fallback.

### Tests first

Repository tests — `src-tauri/src/infrastructure/sqlite/repositories/config/tool_credentials_tests.rs`:

- `set_then_get_roundtrips_the_credentials_blob`
- `get_returns_none_when_row_absent`
- `delete_removes_the_row`
- `has_reflects_presence`
- `set_upserts_on_duplicate_tool_id`
- `updated_at_is_refreshed_on_set`

Resolver tests — `web_search/tests/credentials_tests.rs`:

- `returns_credentials_when_row_present_and_json_valid`
- `returns_not_configured_when_row_absent`
- `returns_malformed_on_invalid_json_blob`
- `returns_malformed_when_api_key_is_empty`
- `returns_malformed_when_cx_is_empty`

Use `ConfigState::in_memory()` (`src-tauri/src/commands/config/state.rs:30`) to back the repository in tests.

### Code

Add to `core/config/repositories.rs`:

```rust
pub trait ToolCredentialsRepository: Send + Sync {
    fn set_tool_credentials(&self, tool_id: &str, credentials: &str) -> RepositoryResult<()>;
    fn get_tool_credentials(&self, tool_id: &str) -> RepositoryResult<Option<String>>;
    fn delete_tool_credentials(&self, tool_id: &str) -> RepositoryResult<()>;
    fn has_tool_credentials(&self, tool_id: &str) -> RepositoryResult<bool>;
}
```

Include in the combined `ConfigRepository` trait bound.

Add a new migration in `src-tauri/src/infrastructure/sqlite/migrations/` that creates:

```sql
CREATE TABLE IF NOT EXISTS tool_credentials (
    tool_id TEXT PRIMARY KEY,
    credentials TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

Implement on `SqliteConfigRepository`.

Resolver (`web_search/credentials.rs`):

```rust
pub struct ResolvedCredentials { pub api_key: RedactedSecret, pub cx: String }

pub fn resolve(cfg: &dyn ConfigRepository) -> Result<ResolvedCredentials, CredentialsError> {
    let Some(blob) = cfg.get_tool_credentials("search_web_search")? else {
        return Err(CredentialsError::NotConfigured);
    };
    let parsed: StoredCreds = serde_json::from_str(&blob)
        .map_err(|e| CredentialsError::Malformed(e.to_string()))?;
    if parsed.api_key.trim().is_empty() || parsed.cx.trim().is_empty() {
        return Err(CredentialsError::Malformed("empty api_key or cx".into()));
    }
    Ok(ResolvedCredentials { api_key: RedactedSecret::new(parsed.api_key), cx: parsed.cx })
}
```

`RedactedSecret` is a small newtype in `web_search/secret.rs` whose `Debug` / `Display` impls print `<redacted>`.

Exit: repository + resolver tests green; migration runs cleanly on a fresh DB and on an existing one with data.

---

## Phase 5a — Per-workspace domain policy

Goal: persist + enforce an allow/deny list scoped to the canonicalized `workspace_root` path.

### Tests first

Repository tests — `src-tauri/src/infrastructure/sqlite/repositories/config/workspace_policy_tests.rs`:

- `empty_policy_is_returned_when_no_row`
- `set_persists_allowlist_and_denylist`
- `set_upserts_on_duplicate_workspace_path`
- `delete_removes_the_row`

Enforcement tests — `web_search/tests/policy_tests.rs`:

- `allows_all_when_allowlist_empty_and_denylist_empty`
- `rejects_site_argument_not_in_allowlist`
- `rejects_site_argument_in_denylist`
- `accepts_subdomain_when_pattern_has_leading_dot`
- `wildcard_star_in_allowlist_means_any`
- `filters_provider_results_to_allowlist_only`
- `drops_denylisted_urls_from_results`
- `returns_filtered_by_policy_flag_when_filtering_empties_results`
- `rewrites_request_to_or_expanded_site_when_no_site_given_and_allowlist_set`
- `falls_back_to_global_when_workspace_root_is_none`

### Code

- `core/config/repositories.rs` — add `WorkspaceSearchPolicyRepository` trait with `get`, `set`, `delete`.
- `infrastructure/sqlite/migrations/` — migration for `workspace_search_policy` (schema in `spec.md §4.9`).
- `web_search/policy.rs` — pure domain-matching helpers:

```rust
pub struct Policy { pub allowlist: Vec<DomainPattern>, pub denylist: Vec<DomainPattern> }
pub fn permits_site(policy: &Policy, site: &str) -> bool { ... }
pub fn filter_results(policy: &Policy, results: Vec<WebSearchResult>) -> (Vec<WebSearchResult>, bool) { ... }
pub fn site_rewrite(policy: &Policy) -> Option<String> { ... }
```

`DomainPattern` is a small enum: `Any`, `Exact(String)`, `SubdomainOf(String)`.

Path canonicalization helper (`web_search/workspace_key.rs`):

```rust
pub fn canonical_workspace_key(path: &Path) -> String {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    #[cfg(windows)] { canon.to_string_lossy().to_lowercase() }
    #[cfg(not(windows))] { canon.to_string_lossy().into_owned() }
}
```

Exit: enforcement tests green end-to-end against `MockProvider`.

---

## Phase 5b — Result cache (DB-backed with TTL)

Goal: skip the provider call when an identical, non-expired entry exists; respect per-workspace partitioning; re-apply policy on read.

### Tests first

Repository tests — `src-tauri/src/infrastructure/sqlite/repositories/config/web_search_cache_tests.rs`:

- `miss_returns_none_when_key_absent`
- `miss_returns_none_when_entry_expired`
- `hit_returns_results_when_entry_fresh`
- `put_upserts_on_duplicate_key`
- `sweep_expired_removes_only_expired_rows`
- `clear_by_workspace_path_removes_only_that_partition`
- `clear_all_empties_the_table`

Cache key tests — `web_search/tests/cache_key_tests.rs`:

- `same_inputs_produce_same_key`
- `different_query_produces_different_key`
- `different_num_results_produces_different_key`
- `different_language_produces_different_key`
- `different_site_produces_different_key`
- `different_workspace_produces_different_key`
- `normalization_collapses_surrounding_whitespace_in_query`
- `normalization_lowercases_language_tag`

Tool-level integration tests (extend `tool_execute_tests.rs`):

- `serves_repeat_request_from_cache_without_provider_call`
- `sets_cached_true_in_output_on_hit`
- `reapplies_policy_to_cached_results_after_policy_change`
- `bypasses_cache_when_ttl_expired`

### Code

- `core/config/repositories.rs` — add `WebSearchCacheRepository` trait (`get`, `put`, `sweep_expired`, `clear(Option<workspace_path>)`, `count`).
- New migration for `web_search_cache` (schema in `spec.md §4.10`).
- `web_search/cache.rs`:
  - `fn cache_key(req: &NormalizedRequest, workspace_key: &str) -> String` — SHA-256 via `sha2` crate.
  - Lookup + write orchestration; TTL configurable via a simple `CacheConfig { ttl: Duration }` loaded from `tool_credentials` metadata (or a new small `tool_settings` row — decision point during implementation, whichever is simpler).
- Add a startup sweep: `SqliteConfigRepository::sweep_expired_web_search_cache()` called once from `lib.rs` init (non-blocking).

Exit: cache hit/miss paths tested, cache respects policy on read.

---

## Phase 5c — System locale resolver

Goal: default `hl` to the host system locale when the LLM does not specify one.

### Tests first

In `web_search/tests/locale_tests.rs`:

- `uses_system_locale_when_language_is_none`
- `explicit_language_overrides_system_locale`
- `omits_hl_when_system_locale_cannot_be_determined`
- `normalizes_locale_to_lowercase_bcp47` (e.g., `en-US` → `en-us`)

Gate the "real sys-locale" path behind a small `LocaleProvider` trait so unit tests can inject a known value without depending on the host.

### Code

`web_search/locale.rs`:

```rust
pub trait LocaleProvider: Send + Sync { fn locale(&self) -> Option<String>; }
pub struct SystemLocale;
impl LocaleProvider for SystemLocale { fn locale(&self) -> Option<String> { sys_locale::get_locale() } }

pub fn resolve_hl(explicit: Option<String>, provider: &dyn LocaleProvider) -> Option<String> { ... }
```

Exit: locale tests green.

---

## Phase 6 — Register the tool

Goal: flip the switch. `search_web_search` appears in `ToolRegistry::definitions()` when the feature flag is on.

### Tests first

In `src-tauri/src/infrastructure/tools/mod.rs`:

Extend `test_tool_registry_builds_expected_tools` with:

- with `CODER_LLM_TOOL_WEB_SEARCH=0`, `tool_names.contains("search_web_search") == false`
- with `CODER_LLM_TOOL_WEB_SEARCH=1`, `tool_names.contains("search_web_search") == true`

### Code

In `web_search/mod.rs` add a small constructor that wires the real pieces:

```rust
pub fn new_production() -> Self {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("Coder/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client builds with default config");
    Self { client_factory: /* closure to make GoogleProvider per-call with live creds */ }
}
```

Credentials are resolved **inside** `execute` (not at construction) because they may change at runtime via settings UI.

In `search/mod.rs`:

```rust
fn tools(&self) -> Vec<Box<dyn Tool>> {
    vec![Box::new(GrepTool), Box::new(GoogleScrapeTool), Box::new(WebSearchTool::new_production())]
}
```

Exit: registry tests green; manual `cargo tauri dev` with flag on lists the tool to the LLM.

---

## Phase 7 — Tauri commands: credentials, policy, cache

Goal: expose the three new persistence surfaces (credentials, workspace policy, cache) plus a live "Test" endpoint to the frontend.

### Tests first

Backend command tests:

- `src-tauri/src/commands/config/tool_credentials_tests.rs`:
  - `set_tool_credentials_rejects_empty_api_key`
  - `set_tool_credentials_rejects_empty_cx`
  - `get_tool_credentials_returns_none_when_absent`
  - `get_tool_credentials_returns_stored_value`
  - `delete_tool_credentials_removes_the_entry`
  - `has_tool_credentials_reflects_presence`
- `src-tauri/src/commands/config/workspace_policy_tests.rs`:
  - `set_workspace_policy_canonicalizes_path`
  - `get_workspace_policy_returns_empty_policy_for_unknown_path`
  - `delete_workspace_policy_removes_entry`
- `src-tauri/src/commands/config/web_search_cache_tests.rs`:
  - `clear_web_search_cache_all_empties_table`
  - `clear_web_search_cache_by_workspace_removes_partition`
  - `cache_stats_returns_entry_count`

Frontend unit tests:

- `src-ui/src/lib/api/toolCredentials.test.ts`, `workspacePolicy.test.ts`, `webSearchCache.test.ts` — thin IPC wrapper tests verifying command name + argument shape.

### Code

`src-tauri/src/commands/config/tool_credentials.rs`:

```rust
#[serde(tag = "tool_id", rename_all = "snake_case")]
pub enum ToolCredentialsPayload {
    SearchWebSearch { api_key: String, cx: String },
}

#[tauri::command] pub async fn set_tool_credentials(tool_id: String, payload: ToolCredentialsPayload, state: State<ConfigState>) -> Result<(), String>;
#[tauri::command] pub async fn get_tool_credentials(tool_id: String, state: State<ConfigState>) -> Result<Option<ToolCredentialsPayload>, String>;
#[tauri::command] pub async fn delete_tool_credentials(tool_id: String, state: State<ConfigState>) -> Result<(), String>;
#[tauri::command] pub async fn has_tool_credentials(tool_id: String, state: State<ConfigState>) -> Result<bool, String>;
#[tauri::command] pub async fn test_web_search_credentials(state: State<ConfigState>) -> Result<ValidationResult, String>;
```

The `tool_id` tag keeps the payload union extensible without overloading wire formats.

`src-tauri/src/commands/config/workspace_policy.rs`:

```rust
#[tauri::command] pub async fn get_workspace_search_policy(workspace_path: String, state: State<ConfigState>) -> Result<WorkspaceSearchPolicy, String>;
#[tauri::command] pub async fn set_workspace_search_policy(workspace_path: String, allowlist: Vec<String>, denylist: Vec<String>, state: State<ConfigState>) -> Result<(), String>;
#[tauri::command] pub async fn delete_workspace_search_policy(workspace_path: String, state: State<ConfigState>) -> Result<(), String>;
```

`src-tauri/src/commands/config/web_search_cache.rs`:

```rust
#[tauri::command] pub async fn clear_web_search_cache(workspace_path: Option<String>, state: State<ConfigState>) -> Result<usize, String>;
#[tauri::command] pub async fn web_search_cache_stats(state: State<ConfigState>) -> Result<CacheStats, String>;
```

Register all commands in `lib.rs` under `invoke_handler![...]`.

Exit: backend commands tested; TS wrappers in place.

---

## Phase 8 — Settings UI

Goal: one Settings section for Web Search that covers credentials, per-workspace domain policy, and cache controls.

**Visual reference**: `mockups.html` panels 07–09. Detailed per-component specs in Appendix B.3–B.5 below.

### Tests first

Credentials sub-component — `src-ui/src/lib/components/settings/web-search/WebSearchCredentials.test.ts`:

- `renders_empty_form_when_no_credentials_stored`
- `prefills_masked_api_key_when_credentials_exist`
- `toggles_key_visibility_on_reveal_click`
- `shows_validation_error_on_empty_inputs`
- `calls_set_tool_credentials_on_save`
- `disables_save_while_test_is_running`
- `shows_success_toast_on_successful_test`
- `shows_error_message_on_test_failure`

Policy sub-component — `WebSearchPolicy.test.ts`:

- `renders_empty_allowlist_and_denylist_for_new_workspace`
- `adds_domain_to_allowlist_on_submit`
- `removes_domain_on_chip_dismiss`
- `rejects_invalid_domain_syntax`
- `warns_when_denylist_contains_an_entry_also_present_in_allowlist`
- `calls_set_workspace_search_policy_on_save`

Cache sub-component — `WebSearchCacheControls.test.ts`:

- `renders_cache_entry_count`
- `calls_clear_web_search_cache_with_undefined_on_clear_all`
- `calls_clear_web_search_cache_with_workspace_on_clear_workspace`
- `shows_toast_with_removed_count_after_clear`

### Code

New folder: `src-ui/src/lib/components/settings/web-search/`:

- `WebSearchSection.svelte` — container
- `WebSearchCredentials.svelte` — key + CX + Test
- `WebSearchPolicy.svelte` — allow/deny domain editor
- `WebSearchCacheControls.svelte` — stats + clear actions
- Module CSS files for each

Wire the section into the existing Settings page (exact path to be confirmed at implementation time via `Grep -l "store_api_key" src-ui/`; do not create a parallel settings page).

Design tokens only (no hardcoded spacing/colors); respect `RULES.md` CSS rules and the design-system single source of truth.

Exit: `npm run check` clean, Vitest green, manual `npm run dev` smoke test.

---

## Phase 9 — Refactor `ToolCallMessage` to a facade, then add the web_search renderer

Goal: convert the current 297-line component (one file with a hardcoded `toolIconMap` and two linear switch statements that grow with every new tool) into a thin **facade** that delegates to per-tool **renderer configs** looked up through a **factory**. The web_search tool then lands as an isolated renderer file, not as another branch in an already-overloaded switch.

Broken into two sub-phases so the refactor is reviewable, rollback-safe, and independently mergeable:

- **9a** — refactor only, characterization tests first, zero behavior change for any existing tool.
- **9b** — add the `search_web_search` renderer + `WebSearchResults` + `$lib/api/shell.ts`.

**Visual reference**: `mockups.html` panels 01–06 cover the six `ToolCallMessage` states; panel 03 also contains the `WebSearchResults` block. Detailed per-component specs in Appendix B.1–B.2 below.

---

### Phase 9a — Refactor to facade + factory

#### Tests first (characterization)

Before moving any code, expand `src-ui/src/lib/components/chat/messages/ToolCallMessage.test.ts` to exhaustively cover the **current** component's behavior. The tests must pass against the unrefactored component, then stay green through the split. One test per branch in the existing `toolIconMap`, `getActionSummary`, and `getOutcomeSummary`:

Icon branches:

- `renders_FileText_icon_for_fs_read_file_and_read_and_read_file`
- `renders_FilePenLine_icon_for_fs_write_file_and_write_and_write_file`
- `renders_FileSearch_icon_for_fs_glob_and_glob`
- `renders_Search_icon_for_fs_grep_and_grep`
- `renders_Terminal_icon_for_execute_command_and_bash`
- `renders_Globe_icon_for_web_request_and_web_fetch_and_webfetch`
- `renders_Code_icon_for_llm_request`
- `renders_PlayCircle_icon_for_skill`
- `renders_Eye_icon_for_task`
- `renders_Code_fallback_icon_for_unknown_tool`

Summary branches:

- `fs_read_shows_Read_file_title_and_filePath_subtitle`
- `fs_write_shows_Modified_file_title_and_filePath_subtitle`
- `fs_glob_shows_Searched_files_title_and_pattern_subtitle`
- `fs_grep_shows_Searched_content_title_and_pattern_subtitle`
- `bash_shows_Ran_command_title_and_command_subtitle`
- `web_fetch_shows_Fetched_URL_title_and_url_subtitle`
- `skill_shows_Loaded_skill_title_and_name_subtitle`
- `task_shows_Spawned_agent_title_and_description_subtitle`
- `unknown_tool_shows_tool_name_as_title_and_first_arg_value_as_subtitle`

Outcome branches (each for success + pending + error):

- `fs_read_outcome_reports_lines_read_count`
- `fs_write_outcome_reports_Changes_applied_successfully`
- `fs_glob_outcome_reports_Found_N_matching_files`
- `fs_grep_outcome_reports_Found_matches_in_N_files`
- `bash_outcome_reflects_exit_code_zero_vs_nonzero`
- `pending_status_always_shows_Executing`
- `error_status_always_shows_Action_failed`

#### Code — step 1: renderer contract

Create `src-ui/src/lib/components/chat/messages/tool-renderers/types.ts`:

```ts
import type { Component } from 'svelte';

export type ToolCallStatus = 'pending' | 'success' | 'error';

export interface ToolSummary {
    title: string;
    subtitle?: string;
}

export interface ToolRenderer<TArgs = unknown, TResult = unknown> {
    icon: Component;
    getSummary(args: TArgs | undefined): ToolSummary;
    getOutcome(result: TResult | undefined, status: ToolCallStatus): string;
    /**
     * Optional custom body. When provided, replaces the generic Result Overview
     * block in ToolCallBody. Receives the same args/result/status the renderer
     * used to compute its summary and outcome.
     */
    BodyComponent?: Component<{
        args: TArgs | undefined;
        result: TResult | undefined;
        status: ToolCallStatus;
    }>;
}
```

#### Code — step 2: extract existing tools as renderer configs

One renderer per tool (or per tightly-coupled tool family). Files live under `tool-renderers/`:

```
tool-renderers/
├── types.ts
├── defaultRenderer.ts       # fallback for unknown tools
├── fsRenderers.ts           # fsRead, fsWrite, fsGlob, fsGrep
├── bashRenderer.ts          # execute_command + bash aliases
├── webFetchRenderer.ts      # web_request, web_fetch, webfetch
├── llmRequestRenderer.ts
├── skillRenderer.ts
├── taskRenderer.ts
├── registry.ts              # static map + factory function
└── registry.test.ts
```

Example — `fsRenderers.ts`:

```ts
import { FileText, FilePenLine, FileSearch, Search } from 'lucide-svelte';
import type { ToolRenderer } from './types';

interface FsPathArgs { path?: string; filePath?: string; }
interface FsPatternArgs { pattern?: string; }

export const fsReadRenderer: ToolRenderer<FsPathArgs, { content?: string }> = {
    icon: FileText,
    getSummary: (args) => ({ title: 'Read file', subtitle: args?.filePath ?? args?.path }),
    getOutcome: (res, status) => {
        if (status === 'pending') return 'Executing...';
        if (status === 'error') return 'Action failed';
        return `Read ${res?.content?.split('\n').length ?? 0} lines`;
    },
};

export const fsWriteRenderer: ToolRenderer<FsPathArgs> = { /* ... */ };
export const fsGlobRenderer:  ToolRenderer<FsPatternArgs, unknown> = { /* ... */ };
export const fsGrepRenderer:  ToolRenderer<FsPatternArgs, unknown> = { /* ... */ };
```

#### Code — step 3: factory

Static map, no import-time side effects:

```ts
// tool-renderers/registry.ts
import { defaultRenderer } from './defaultRenderer';
import { fsReadRenderer, fsWriteRenderer, fsGlobRenderer, fsGrepRenderer } from './fsRenderers';
import { bashRenderer } from './bashRenderer';
import { webFetchRenderer } from './webFetchRenderer';
import { llmRequestRenderer } from './llmRequestRenderer';
import { skillRenderer } from './skillRenderer';
import { taskRenderer } from './taskRenderer';
import type { ToolRenderer } from './types';

const RENDERERS: Record<string, ToolRenderer> = {
    // fs
    fs_read_file:   fsReadRenderer,
    read:           fsReadRenderer,
    read_file:      fsReadRenderer,
    fs_write_file:  fsWriteRenderer,
    write:          fsWriteRenderer,
    write_file:     fsWriteRenderer,
    fs_glob:        fsGlobRenderer,
    glob:           fsGlobRenderer,
    fs_grep:        fsGrepRenderer,
    grep:           fsGrepRenderer,
    // shell
    execute_command: bashRenderer,
    bash:            bashRenderer,
    // web
    web_request: webFetchRenderer,
    web_fetch:   webFetchRenderer,
    webfetch:    webFetchRenderer,
    // misc
    llm_request: llmRequestRenderer,
    skill:       skillRenderer,
    task:        taskRenderer,
};

export function getToolRenderer(toolName: string): ToolRenderer {
    return RENDERERS[toolName] ?? defaultRenderer;
}
```

Registry unit tests — `registry.test.ts`:

- `returns_default_renderer_for_unknown_tool_name`
- `returns_same_renderer_instance_for_aliases` (so consumers can trust reference equality)
- `every_registered_renderer_has_icon_summary_and_outcome` (iterates the map)
- `no_two_registered_renderers_share_the_same_object_for_different_semantics` (sanity)

#### Code — step 4: split the view

Extract four small components. The facade keeps only composition + state toggle.

- `ToolCallHeader.svelte` — button row: chevron, badge+icon, title/subtitle, outcome text, status indicator. Receives `{ renderer, summary, outcome, status, expanded, onToggle }`.
- `ToolCallBody.svelte` — expanded content. Receives `{ renderer, args, result, status, timestamp }`. Renders the Parameters group, then either `renderer.BodyComponent` (if set) or the generic Result Overview + error banner, then the raw JSON toggle, then the timestamp footer.
- `ToolCallParameters.svelte` — receives `{ args, hiddenKeys }`. Filters out keys already displayed in the header subtitle.
- `ToolCallRawView.svelte` — receives `{ args, result }`. The "View raw JSON" section, already a distinct block today; moves verbatim.

`ToolCallMessage.svelte` (the facade) shrinks to roughly this shape:

```svelte
<script lang="ts">
    import type { Message } from '$lib/types/message';
    import ToolCallHeader from './ToolCallHeader.svelte';
    import ToolCallBody from './ToolCallBody.svelte';
    import { getToolRenderer } from './tool-renderers/registry';
    import styles from './ToolCallMessage.module.css';

    interface Props { message: Message; }
    let { message }: Props = $props();

    const agentData = $derived(message.agentData);
    const toolName  = $derived(agentData?.toolName ?? 'unknown');
    const status    = $derived(agentData?.toolCallStatus ?? 'pending');
    const renderer  = $derived(getToolRenderer(toolName));
    const summary   = $derived(renderer.getSummary(agentData?.toolArgs));
    const outcome   = $derived(renderer.getOutcome(agentData?.toolResult, status));

    let expanded = $state(false);
    const toggle = () => expanded = !expanded;
</script>

<div class={[
    styles.toolCall,
    expanded && styles.expanded,
    status === 'error' && styles.hasError
].filter(Boolean).join(' ')}>
    <ToolCallHeader {renderer} {summary} {outcome} {status} {expanded} onToggle={toggle} />
    {#if expanded}
        <ToolCallBody
            {renderer}
            {status}
            args={agentData?.toolArgs}
            result={agentData?.toolResult}
            timestamp={message.timestamp}
        />
    {/if}
</div>
```

#### File size budget after 9a

| File | Target |
|---|---|
| `ToolCallMessage.svelte` | ≤ 80 lines |
| `ToolCallHeader.svelte` | ≤ 140 lines |
| `ToolCallBody.svelte` | ≤ 120 lines |
| `ToolCallParameters.svelte` | ≤ 80 lines |
| `ToolCallRawView.svelte` | ≤ 100 lines |
| `tool-renderers/types.ts` | ≤ 40 lines |
| `tool-renderers/registry.ts` | ≤ 80 lines |
| Each `*Renderer.ts` | ≤ 60 lines |

Every file sits well below the 500-line soft limit and matches the discipline we already set for the new web_search Rust files.

#### Exit criteria for 9a

- All characterization tests pass against the refactored facade.
- Registry tests pass.
- New component tests cover `ToolCallHeader` and `ToolCallBody` in isolation.
- `npm run check` clean, Vitest green, dev server smoke-tested for every existing tool type (read/write/bash/skill/task/fetch).
- The diff for 9a contains **no web_search references** — it ships on its own.

---

### Phase 9b — Add the web_search renderer + clickable results

Goal: with the refactor in place, adding web_search is a small, additive change: one renderer file, one custom body, one reusable results list, one shell wrapper, one line in the registry.

#### Tests first

Renderer tests — `tool-renderers/webSearchRenderer.test.ts`:

- `renders_Globe_icon`
- `summary_title_is_Searching_web_when_status_is_pending`
- `summary_title_is_Searched_web_when_status_is_success_or_error`
- `summary_subtitle_is_the_query`
- `outcome_is_Executing_when_pending`
- `outcome_is_No_results_when_count_is_zero`
- `outcome_is_Found_N_results_for_non_cached_success`
- `outcome_is_Found_N_results_cached_when_cached_true`
- `outcome_is_Search_failed_when_error`

Body-component tests — `tool-renderers/WebSearchBody.test.ts`:

- `renders_WebSearchResults_on_successful_non_empty_result`
- `renders_policy_banner_when_filtered_by_policy_is_true`
- `renders_not_configured_banner_when_error_code_is_not_configured`
- `renders_generic_error_banner_for_other_error_codes`
- `renders_nothing_extra_when_status_is_pending`

Results component — `WebSearchResults.test.ts`:

- `renders_one_row_per_result`
- `row_shows_title_url_and_snippet`
- `clicking_row_calls_openUrl_with_absolute_https_url`
- `ignores_click_for_non_http_url`
- `keyboard_enter_activates_row_like_click`
- `has_accessible_name_for_each_row`

Shell wrapper — `$lib/api/shell.test.ts`:

- `openUrl_rejects_non_http_scheme`
- `openUrl_invokes_tauri_plugin_opener_for_https`
- `openUrl_invokes_tauri_plugin_opener_for_http`

Registry regression — extend `registry.test.ts`:

- `search_web_search_resolves_to_webSearchRenderer`

#### Code

- `tool-renderers/webSearchRenderer.ts` — exports a `ToolRenderer<WebSearchArgs, WebSearchOutput | ToolErrorBody>`. Assigns `BodyComponent: WebSearchBody`. Consumes the shared types from B.9.
- `tool-renderers/WebSearchBody.svelte` — routes between `WebSearchResults`, the policy banner, the not-configured banner, and the generic error banner based on `status` + `error_code` + `filtered_by_policy`. Receives the same `{ args, result, status }` any body receives.
- `WebSearchResults.svelte` (+ module CSS) — per Appendix B.2. Lives alongside the other chat message components (`src-ui/src/lib/components/chat/messages/`) because it's reusable across any future tool that wants to render web-result-shaped data, not just this renderer.
- `$lib/api/shell.ts` — `openUrl(url: string): Promise<void>` wrapping `@tauri-apps/plugin-opener`. Validates via `URL` parsing; rejects any non-`http(s)` scheme.
- `tool-renderers/registry.ts` — add one line: `search_web_search: webSearchRenderer`.

Tauri plugin registration:

- Add `tauri_plugin_opener` to `src-tauri/Cargo.toml` (already listed in Phase 0).
- `.plugin(tauri_plugin_opener::init())` in `src-tauri/src/lib.rs`.
- Declare the capability in `src-tauri/capabilities/` with scope limited to `http://*` and `https://*`. No file scheme. No shell pass-through.

#### Exit criteria for 9b

- All 9b tests green.
- Manual: open `mockups.html` panel 03 in dev, run a real search, confirm the rendering matches and that a click opens the browser.
- Manual: provoke each error code (disable creds, set denylist, unplug network) and confirm the correct banner variant renders per B.1.
- Diff for 9b touches no existing renderers — the extension point held.

---

## Phase 10 — Documentation, changelog, and catalog

- [ ] Update `docs/tools-and-agent-integration.md` — add `search_web_search` to the `search` catalog table with parameter docs, and increment the category summary count.
- [ ] Add a changelog entry under the `Added` section of `CHANGELOG.md`.
- [ ] Promote `spec.md` status from DRAFT → ACTIVE (this is the living reference now).
- [ ] Mark `plan.md` as `COMPLETE — <date>` and move it to `docs/archive/tools/web_search/plan.md`.
- [ ] Update `docs/tools/web_search/README.md` to reflect final statuses.
- [ ] Verify no `search_google_scrape` references were broken; the dev scrape tool stays (it predates this work).

Exit: documentation governance satisfied.

---

## Phase 11 — End-to-end validation

Manual, before merge:

- [ ] `CODER_LLM_TOOL_WEB_SEARCH=1 cargo tauri dev`.
- [ ] Settings → Tools → Web Search → enter real creds → Test → green.
- [ ] Ask the agent "what's the latest Svelte version?" — tool fires, results visible in `ToolCallMessage`, LLM answer cites one of the URLs.
- [ ] Click a result row — default browser opens the URL.
- [ ] Ask the exact same question again — output has `cached: true`, `(cached)` suffix visible in the outcome, no provider call in logs.
- [ ] Add a denylist domain covering one of the previous results, re-run the query — that result is filtered out (demonstrates policy re-applied on cached reads).
- [ ] Add an allowlist-only entry for `github.com`, ask a broader question — provider is called with `siteSearch=github.com`, results all come from github.com.
- [ ] Clear cache button reports the right entry count and the cache table is empty afterward.
- [ ] Ask the same question with invalid creds — tool returns a clean error, agent degrades gracefully.
- [ ] Turn the flag off — the tool disappears from the LLM-visible tools list.
- [ ] Delete credentials in settings — tool returns the actionable "not configured" error on next call.
- [ ] Grep logs for the API key — it must not appear. Grep for the query string — it **must** appear (per §4.8).
- [ ] With a BCP-47 system locale set (e.g. `pt-BR`), the HTTP request to Google includes `hl=pt-br`.

Automated, before merge:

- [ ] `cargo test -p app --lib`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cd src-ui && npm run check`
- [ ] `cd src-ui && npx vitest run`
- [ ] `cd src-ui && npx vitest run src/lib/styles/css-integrity.test.ts`

---

## Appendix B — Component specifications

Each Svelte component below has a matching panel in `mockups.html`. Open that file in a browser to see the visual, then use this section as the contract for props, states, accessibility, and interaction behavior.

### B.1 `ToolCallMessage.svelte` — facade

**Mockup panels**: 01 Pending · 02 Success collapsed · 04 Cached · 05 No results · 06 Error (rendering driven by per-tool renderer; the facade itself is panel-agnostic).

**Purpose**: thin compositional shell. Reads the message, resolves a `ToolRenderer` from the registry, and slots a header + optional body. Knows nothing about specific tools — adding `search_web_search` (or any future tool) is one entry in the registry, never a new `if`/`switch` here.

**Props**:

```ts
interface Props { message: Message; }
```

**Responsibilities** (and only these):

1. Derive `toolName`, `status`, `args`, `result` from `message.agentData`.
2. Resolve `renderer = getToolRenderer(toolName)`.
3. Compute `summary = renderer.getSummary(args)` and `outcome = renderer.getOutcome(result, status)`.
4. Manage local `expanded` state.
5. Compose the visual root with shared classes (`toolCall`, `expanded`, `hasError`) and slot in `<ToolCallHeader>` and `<ToolCallBody>`.

**What the facade must NOT do**: import any tool-specific component, contain any tool-name string literal beyond the unknown-fallback case, format any per-tool summary text, decide whether to show a custom body. All of that lives in the renderer registered for that tool.

**File budget**: ≤ 80 lines including markup and styles import.

#### B.1.1 Renderer contract — `tool-renderers/types.ts`

```ts
import type { Component } from 'svelte';

export type ToolCallStatus = 'pending' | 'success' | 'error';

export interface ToolSummary {
    title: string;
    subtitle?: string;
}

export interface ToolRenderer<TArgs = unknown, TResult = unknown> {
    icon: Component;
    getSummary(args: TArgs | undefined): ToolSummary;
    getOutcome(result: TResult | undefined, status: ToolCallStatus): string;
    BodyComponent?: Component<{
        args: TArgs | undefined;
        result: TResult | undefined;
        status: ToolCallStatus;
    }>;
}
```

**Contract invariants** (enforced by tests, not just convention):

- `getSummary` and `getOutcome` are **pure**. Same input → same output. No DOM access, no I/O, no `Date.now()`.
- They must tolerate `undefined` args and result without throwing — tool calls in flight have neither yet.
- `icon` is a Svelte component reference, not a string or SVG element. This keeps icon usage consistent with the existing lucide-svelte pattern.
- `BodyComponent`, when present, owns its own render lifecycle and must accept the three documented props by name (no positional). It receives unparsed values; widening to `unknown` then narrowing is the renderer's responsibility.

#### B.1.2 Factory — `tool-renderers/registry.ts`

Static `Record<string, ToolRenderer>`, exported through a `getToolRenderer(name)` function that returns the requested renderer or the default fallback.

**Why a static map and not a mutable `register()` API**:

- No import-time side effects → component tests can import a renderer in isolation without dragging the whole registry into the test bundle.
- The full set of supported tools is grep-able from one file → reviewers see the catalog at a glance.
- Adding a tool is one edit (one import, one map entry) → no temptation to scatter `register(...)` calls across files.

**Aliases**: short legacy names (`read`, `write`, `glob`, `grep`, `bash`, `webfetch`) point at the same renderer instance as the canonical name. Reference equality matters — the registry test asserts it so consumers can rely on it for memoization later if needed.

**File budget**: ≤ 80 lines.

#### B.1.3 Default fallback — `tool-renderers/defaultRenderer.ts`

Catches unknown tools (e.g., a message produced by a build that knew a tool the current frontend doesn't):

```ts
import { Code } from 'lucide-svelte';
import type { ToolRenderer } from './types';

export const defaultRenderer: ToolRenderer = {
    icon: Code,
    getSummary: (args: any) => ({
        title: 'Tool call',
        subtitle: args && typeof args === 'object'
            ? String(Object.values(args)[0] ?? '')
            : undefined,
    }),
    getOutcome: (_, status) => {
        if (status === 'pending') return 'Executing...';
        if (status === 'error')   return 'Action failed';
        return 'Completed';
    },
};
```

#### B.1.4 Header — `ToolCallHeader.svelte`

**Props**:

```ts
interface Props {
    renderer: ToolRenderer;
    summary: ToolSummary;
    outcome: string;
    status: ToolCallStatus;
    expanded: boolean;
    onToggle: () => void;
}
```

**Responsibilities**: render the existing header markup (chevron + badge + title/subtitle + outcome + status indicator). Receives `renderer.icon` as a component and renders it via Svelte's dynamic component syntax. Status indicator is local: a small switch over `status` (this is a UI-only concern, not tool-specific).

**Accessibility**: keeps the existing `aria-expanded` on the root `<button>`. Status icon retains its `aria-label` (`Pending` / `Success` / `Error`).

#### B.1.5 Body — `ToolCallBody.svelte`

**Props**:

```ts
interface Props {
    renderer: ToolRenderer;
    args: unknown;
    result: unknown;
    status: ToolCallStatus;
    timestamp: Date;
}
```

**Responsibilities**:

1. Render the **Parameters** group via `<ToolCallParameters>` (filtering out keys already shown in the summary subtitle — `path`, `filePath`, `pattern`, `command`, `url`, `name`, `query`).
2. Render the **Result Overview**:
   - If `renderer.BodyComponent` is set, render `<svelte:component this={renderer.BodyComponent} {args} {result} {status} />`.
   - Otherwise, render the existing generic success/error blocks.
3. Render the raw-JSON toggle via `<ToolCallRawView>`.
4. Render the timestamp footer.

**Why the body owns this orchestration and not the facade**: the facade should not branch on whether a renderer has a custom body — that branching is a body-rendering concern.

#### B.1.6 Parameters and raw view

Two small extracted components:

- `ToolCallParameters.svelte` — receives `{ args: Record<string, unknown> | undefined; hiddenKeys: Set<string> }`. Renders the existing `parametersList` markup. Pure presentational.
- `ToolCallRawView.svelte` — receives `{ args: unknown; result: unknown }`. Owns the "View raw JSON" toggle and the highlighted JSON `<pre>` blocks. The `hljs` import moves here, off the facade.

Both are unit-tested in isolation.

---

### B.2 Web search renderer — three collaborating pieces

**Mockup panels**: 01–06 (every state is routed by these three files).

The web_search tool lands as a renderer config + a custom body + a reusable results list. No code changes leak outside this trio. This is the pattern every future tool with a custom view will follow.

#### B.2.1 `tool-renderers/webSearchRenderer.ts` (new)

**Purpose**: the registry entry for `search_web_search`. Declares the icon, derives summary text, derives outcome text, and points at its custom body.

```ts
import { Globe } from 'lucide-svelte';
import type { ToolRenderer } from './types';
import type { WebSearchArgs, WebSearchOutput, ToolErrorBody } from '$lib/types/webSearch';
import WebSearchBody from './WebSearchBody.svelte';

type Result = WebSearchOutput | ToolErrorBody;

export const webSearchRenderer: ToolRenderer<WebSearchArgs, Result> = {
    icon: Globe,
    getSummary: (args) => ({
        title: 'Searched web',        // overridden to 'Searching web' while pending (see below)
        subtitle: args?.query,
    }),
    getOutcome: (result, status) => {
        if (status === 'pending') return 'Executing...';
        if (status === 'error')   return 'Search failed';
        const out = result as WebSearchOutput | undefined;
        if (!out || out.count === 0) return 'No results';
        return out.cached ? `Found ${out.count} results (cached)` : `Found ${out.count} results`;
    },
    BodyComponent: WebSearchBody,
};
```

**Pending vs terminal title**: the header computes the summary once per render; to swap `Searched` → `Searching`, the renderer inspects `status`. Since `getSummary` does not receive status today, either (a) change the contract to pass status (breaks other renderers' purity), or (b) keep `Searched web` in both cases and rely on the spinner + `Executing...` outcome to communicate in-flight state. **Choice**: (b). The spinner already carries the "in progress" signal everywhere else; introducing a per-status title swap only for web_search is inconsistent. The mockup (panel 01) is adjusted accordingly during Phase 9b — update the label text there to `Searched web` and drop the `Searching` variant.

#### B.2.2 `tool-renderers/WebSearchBody.svelte` (new)

**Purpose**: the custom expanded body. Routes between four visual states driven by `(status, result)`.

**Props**:

```ts
interface Props {
    args: WebSearchArgs | undefined;
    result: WebSearchOutput | ToolErrorBody | undefined;
    status: ToolCallStatus;
}
```

**Routing table**:

| Condition | Renders |
|---|---|
| `status === 'pending'` | Nothing (header spinner carries the state). |
| `status === 'error'` and `result.error_code === 'not_configured'` | Actionable banner (panel 06) with "Open settings →" button. |
| `status === 'error'` and any other code | Generic error banner with the `error` text. |
| `status === 'success'` and `result.count === 0` and `result.filtered_by_policy` | Neutral policy banner (panel 05). |
| `status === 'success'` and `result.count === 0` (no policy filter) | Neutral "No results" banner with a short hint. |
| `status === 'success'` and `result.count > 0` | `<WebSearchResults results={result.results} />` + optional `filtered_by_policy` note above the list. |
| `status === 'success'` and `result.cached` | Above + the small "Served from local cache" banner (panel 04). |

Everything is dispatched through a single `$derived` state, not nested `{#if}` blocks beyond two levels deep — readability matters since this is the canonical example of a custom body.

**No direct IPC**: settings navigation fires a `navigate('/settings/tools/web-search')` via the existing router helper. The banner does not know how routing works.

#### B.2.3 `WebSearchResults.svelte` (new)

**Purpose**: render the structured result array as a keyboard-accessible, clickable list whose rows each open the target URL in the user's default browser via `tauri-plugin-opener`. Reused by future tools that produce web-result-shaped data.

**Props interface**:

```ts
interface Props {
    results: WebSearchResult[]; // never undefined; for empty, parent renders the policy banner instead
    onOpen?: (url: string) => void; // optional override for tests; defaults to $lib/api/shell.openUrl
}

interface WebSearchResult {
    title: string;
    url: string;            // absolute http:// or https://
    snippet: string;        // plain text, already HTML-stripped by the Rust tool
    source_domain: string;  // unused in render; reserved for future grouping
}
```

**Structure** (`<ul role="list">` of `<button role="listitem">` rows):

1. Each row is a `<button type="button">`, not an `<a>`. This keeps open-path control inside the component (for CSP cleanliness inside the Tauri webview) and sidesteps the middle-click-opens-in-tab ambiguity.
2. Row content:
   - **Title** (`font-medium`, `--text-sm`, `--color-text-primary`) — single line, truncated with ellipsis when overflowing.
   - **URL** (`--font-mono`, `--text-2xs`, `--color-text-tertiary`) — single line, truncated.
   - **Snippet** (`--text-xs`, `--color-text-secondary`) — clamped to 2 lines with `-webkit-line-clamp: 2`.
3. A subtle `↗` glyph in the top-right of each row, decorative (`aria-hidden`), communicates "opens externally."

**States**:

| State | Behavior |
|---|---|
| Idle | Transparent background; bottom divider via `--color-border-subtle`. |
| Hover | Background switches to `--color-surface-hover` in `var(--transition-fast)`. |
| Focus-visible | Same background + a 2 px inset primary accent on the left edge (mirrors the focus ring pattern used in the existing sidebar). |
| Active (mousedown) | Card motion scale is explicitly **not** applied — this is a dense list, not a card. |

**Open behavior**:

- Click or Enter invokes `onOpen(url) ?? openUrl(url)`.
- The shared wrapper lives at `src-ui/src/lib/api/shell.ts` and rejects any URL whose scheme is not `http:` or `https:` — returns a rejected promise that the caller surfaces via a transient toast. Non-http(s) URLs are dropped silently in the URL validator that runs on props (so they never render as clickable).

**Accessibility**:
- Each row has `aria-label={`Open ${title} at ${source_domain}`}`.
- The snippet is inside the button, so screen readers read title + URL + snippet sequentially.
- `tabindex` is handled by the `<button>` default; no manual roving tabindex needed for a short list (MVP caps at 10).

**File size budget**: ≤ 120 lines script + markup, ≤ 80 lines in the module CSS.

**No props bypass**: the parent (`ToolCallMessage`) passes an already-validated `WebSearchResult[]`. The component does not re-sanitize HTML; the Rust tool is the sanitization boundary (see `spec.md §5`).

---

### B.3 `WebSearchCredentials.svelte` (new)

**Mockup panel**: 07.

**Purpose**: capture the Google API key + CX, save to the DB via `set_tool_credentials`, and run a live verification via `test_web_search_credentials`.

**Props interface**:

```ts
interface Props {
    onSaved?: () => void; // notifies parent section to refresh status
}
```

The component reads its own state from the API wrappers; it does not accept the credential payload as props (keeps write paths centralized and avoids stale reads after Save).

**Internal state (Svelte 5 runes)**:
- `apiKey = $state('')` — typed value; masked visually by `type="password"` unless `revealed` is true.
- `cx = $state('')`.
- `revealed = $state(false)`.
- `saving = $state(false)`.
- `testing = $state(false)`.
- `testResult = $state<{ ok: boolean; message: string; at: Date } | null>(null)`.

**Layout** (see panel 07):

1. **Header** — title "Google Custom Search", hint text under it, plus a right-aligned verification status when `testResult` is fresh (`"Verified Nm ago"`, green check). The status pill disappears after 10 minutes or when either input is modified.
2. **API key field** — `<input type="password">` with a trailing `Show` / `Hide` toggle button. On mount, if `has_tool_credentials('search_web_search')` is true, populate with a placeholder string (`••••••`) that the user must clear before entering a new key; never pre-populate the real value through the IPC unless the user clicks reveal.
3. **CX field** — plain text input (CX is not a secret).
4. **Actions row** — `Save` (primary), `Test`, and a right-aligned `Clear` button that calls `delete_tool_credentials`.

**Validation**:
- Both fields are trimmed before submission.
- `Save` is disabled while either field is empty, or while `saving || testing` is true.
- `Test` is enabled only when both fields have values, even before saving — this lets the user validate before persisting.

**Interaction**:
- `Save` → `set_tool_credentials({ tool_id: 'search_web_search', api_key, cx })` → on success `testResult = null`, emit `onSaved()`.
- `Test` → `test_web_search_credentials()` (which reads the currently-persisted row, **not** the form values, so the user must Save first if they want to test new creds) → success sets `testResult = { ok: true, ... }`; failure sets `{ ok: false, message: <human-readable reason> }` and renders the message as a red inline banner below the actions row.
- `Clear` → confirmation via native dialog → `delete_tool_credentials` → clears form + testResult.

**Accessibility**:
- Labels use `<label for>` tied to each input's `id`.
- The reveal toggle is a `<button aria-label="Show API key" aria-pressed={revealed}>`.
- The verification status uses `aria-live="polite"` so screen readers announce a successful test.

**Security invariants**:
- Never log any input value (no `console.log` leftovers anywhere).
- Clear `apiKey` from state on component unmount to minimize JS heap residency.

---

### B.4 `WebSearchPolicy.svelte` (new)

**Mockup panel**: 08.

**Purpose**: edit the per-workspace allow/deny domain lists.

**Props interface**:

```ts
interface Props {
    workspacePath: string;       // canonicalized absolute path, from the active agent session
    workspaceDisplayPath: string;// tilde-compressed for the header
    onSaved?: () => void;
}
```

The component fetches `get_workspace_search_policy(workspacePath)` on mount and persists via `set_workspace_search_policy`. When `workspacePath === ''` (sessions without a workspace), the component renders a disabled state with a hint that domain policy is per-workspace and no workspace is active.

**Internal state**:
- `allowlist = $state<string[]>([])`.
- `denylist  = $state<string[]>([])`.
- `allowDraft = $state('')`, `denyDraft = $state('')` — the in-progress "add domain" inputs.
- `dirty = $derived(...)` — compared against the last loaded snapshot.

**Layout**:

1. **Header** — title "Domain policy" + workspace display path in mono. If no workspace, header shows a muted "Global · no workspace active" label.
2. **Allowlist row** — label + chip container. Each chip is green (`--color-success-*`). Inline input inside the chip container (transparent background, no border) accepts a domain and adds it on Enter.
3. **Denylist row** — same pattern, red chips (`--color-error-*`).
4. **Actions** — `Save policy` (primary, disabled when `!dirty`), right-aligned `Reset` (reverts to last saved state).

**Validation** (pure helper in `$lib/components/settings/web-search/domain.ts`, unit-tested):
- Trim; reject empty, internal whitespace, schemes (`https://`), paths (`/…`), ports (`:443`), query strings.
- Accept: `host.tld`, `sub.host.tld`, `*` (allowlist only), and `.host.tld` (subdomain match).
- Duplicate detection: adding an existing entry is a silent no-op (the input empties, the chip doesn't duplicate).
- Conflict detection: if the same pattern exists in the other list, show an inline warning near the second list — this is a user error signal, not a block. Save proceeds with the list as entered; the enforcement rules in `spec.md §4.9` make denylist win.

**Interaction**:
- Enter in the draft input → add chip, clear draft.
- Click `×` on a chip → remove. No confirmation (chips are cheap to re-add).
- Save → `set_workspace_search_policy(workspacePath, allowlist, denylist)` → re-load to reconcile, emit `onSaved()`.

**Accessibility**:
- Each chip's remove button is a `<button aria-label="Remove github.com">`.
- The chip container carries `role="list"` with an `aria-label` of "Allowlist" / "Denylist"; each chip is `role="listitem"`.
- The draft input has `aria-label` naming which list it adds to.

---

### B.5 `WebSearchCacheControls.svelte` (new)

**Mockup panel**: 09.

**Purpose**: show cache stats and allow the user to clear per-workspace or all entries; configure default TTL.

**Props interface**:

```ts
interface Props {
    workspacePath: string; // same source as WebSearchPolicy
}
```

**Internal state**:
- `stats = $state<CacheStats | null>(null)` — `{ entries: number, size_bytes: number, default_ttl: Duration }`.
- `ttlInput = $state('24h')`.
- `clearing = $state(false)`.

On mount, call `web_search_cache_stats()`; re-fetch after any clear operation.

**Layout**:

1. **Header** — title "Cache" + one-line hint about policy re-application on read.
2. **Stat grid** (3 columns, equal): Entries (integer, mono), Size (KB, mono), TTL (human, mono). Each stat sits in a bordered tile with a very small all-caps label above (`--text-2xs`, `--color-text-tertiary`).
3. **TTL input row** — single text field with a hint "5 min – 7 days". Validation normalizes `"24h"`, `"30m"`, `"2d"` to seconds via a pure helper; invalid input disables Save.
4. **Actions row** — `Clear this workspace` (secondary), right-aligned `Clear all` (danger).

**Interaction**:
- `Clear this workspace` → `clear_web_search_cache(workspace_path)` → toast: `"Removed N entries"`.
- `Clear all` → native confirmation dialog (`This will remove N cached entries from all workspaces.`) → on confirm, call `clear_web_search_cache(undefined)`.
- TTL change → debounced save (300 ms) via the same command that persists credentials-adjacent settings (implementation detail: either a new `tool_settings` row or a JSON field on the existing credentials row; decide during Phase 5b).

**Accessibility**:
- Stat values have `aria-label`s that expand the unit (e.g., `aria-label="128 kilobytes"`).
- Danger button uses a distinct color token but also a leading `Clear all` label — the color alone does not carry meaning.

---

### B.6 Shared types

The frontend types below live in `src-ui/src/lib/types/webSearch.ts` and are consumed by the four components above. Keeping them in one file makes the component props precise without duplicating shape definitions.

```ts
export interface WebSearchResult {
    title: string;
    url: string;
    snippet: string;
    source_domain: string;
}

export interface WebSearchOutput {
    query: string;
    count: number;
    results: WebSearchResult[];
    provider: 'google_custom_search';
    cached: boolean;
    filtered_by_policy?: boolean;
}

export interface ToolErrorBody {
    error: string;        // free-text, already human-readable from Rust
    error_code?:          // stable machine-readable code used for branching in ToolCallMessage
        | 'not_configured'
        | 'disabled_by_flag'
        | 'rate_limited'
        | 'unauthorized'
        | 'upstream_error'
        | 'invalid_arguments'
        | 'filtered_by_policy';
}
```

The Rust side adds an `error_code` field to the `{"error": "..."}` JSON body returned by `WebSearchTool::execute` on failure. The existing `{"error": "Unknown tool name"}` fallback in `ToolRegistry` does not emit `error_code`; `ToolCallMessage` treats the absence as a generic error and renders the default failure banner.

---

## Estimated sizing (for planning only, not a commitment)

| Phase | Rough effort |
|---|---|
| 0 Alignment + dependencies | 0.5 d |
| 1 Types + args | 0.5 d |
| 2 Provider trait + mock | 0.5 d |
| 3 Tool execute (offline) | 1.0 d |
| 4 Google provider (real HTTP) | 1.0 d |
| 5 Credentials repo + resolver | 1.0 d |
| 5a Workspace policy | 1.0 d |
| 5b Result cache | 1.0 d |
| 5c System locale | 0.25 d |
| 6 Registry wiring | 0.25 d |
| 7 Tauri commands (3 surfaces) | 1.0 d |
| 8 Settings UI (3 sub-sections) | 1.5 d |
| 9a Refactor to facade + factory | 1.0 d |
| 9b Web search renderer + clickable results | 0.75 d |
| 10 Docs/changelog | 0.25 d |
| 11 E2E validation | 0.5 d |
| **Total** | **~12 d** |

These are uninterrupted-developer-days at the repository's quality bar (tests-first, no shortcuts). Real calendar time depends on review and integration feedback.
