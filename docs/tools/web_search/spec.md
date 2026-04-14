# Web Search Tool — Feature Specification

**Status**: ACTIVE
**Last reviewed**: 2026-04-14
**Shipped**: 2026-04-14 (see `CHANGELOG.md` under `[Unreleased] → Added`)
**Purpose**: Scope, interfaces, and constraints for the `search_web_search` tool that lets the agent query the public web via a Google-provided search API.

---

## 1. Problem and goal

The LLM's internal knowledge is frozen at training time. Without a live-web channel, the agent cannot answer questions about current events, recently-released packages, or cite external sources for factual claims. Users are forced to paste search results manually, or they get confident-but-stale answers.

We already ship a dev-only `search_google_scrape` tool that HTML-scrapes google.com. It is gated behind `CODER_LLM_TOOL_GOOGLE_SCRAPE`, explicitly marked unstable, and warns on every call that the markup is ToS-sensitive. It is not a production answer.

**Goal of this work**: ship a production-grade `search_web_search` tool backed by an official Google search API, exposed through the existing `Tool` trait, with credentials managed through the app's existing config store, and surfaced in chat the same way other tools are.

**Non-goals**: rendering pages, replacing the user's browser, crawling, image/news verticals for MVP, supporting multiple search providers simultaneously.

## 2. Why Google Custom Search JSON API

The PRD says "Google-provided search API." The current Google-provided public search APIs that are appropriate for a desktop app are, in order of fit:

1. **Google Custom Search JSON API** (Programmable Search Engine). Official, paid (free tier: 100 queries/day), stable. Requires an API key and a Search Engine ID (`cx`). Can be configured to search the entire web. Default choice.
2. Google Search via third-party aggregators (Serper, SerpAPI). Not first-party; adds a dependency.
3. Google Search Console / Site Verification. Not a search API.

MVP targets **Google Custom Search JSON API** with a "search the entire web" CX. The tool interface is provider-agnostic, so replacing this with Bing, Brave, or a self-hosted SearxNG later is an infrastructure swap, not a contract change.

## 3. User stories

| Persona | Story |
|---|---|
| End user | "Ask 'what's the latest version of Svelte 5?' and have the agent actually look it up, with linked sources." |
| End user | "Have the agent cite URLs when it claims a fact is current." |
| Agent (system actor) | "Call `search_web_search` to reduce hallucination risk when the user asks about dynamic state." |
| Developer | "Read the tool contract in one place; swap the provider without breaking agent prompts." |
| Operator | "Rotate the Google API key from settings without re-launching the app." |

## 4. Functional requirements

### 4.1 Tool registration

- Full tool name: `search_web_search` (category `search`, name `web_search`).
- Registered in `SearchCategory::tools()` alongside `GrepTool` and `GoogleScrapeTool`.
- Exposed to the LLM via `ToolRegistry::definitions()` **only** when the feature flag is enabled and credentials are configured.

### 4.2 Feature flag and gating

- New env constant: `ENV_WEB_SEARCH_ENABLED = "CODER_LLM_TOOL_WEB_SEARCH"`.
- The tool's `ToolMeta::feature_flag` is `Some(ENV_WEB_SEARCH_ENABLED)`.
- `ToolMeta::experimental` is `false` (it is a production API).
- `requires_workspace` is `false`.
- Gating order, evaluated on every call:
  1. Feature flag set → otherwise return `ToolError::Disabled`.
  2. Credentials present (API key + CX) → otherwise return `ToolError::ExecutionFailed("Web search is not configured")` with actionable next step.
  3. Arguments valid.
  4. Execute HTTP request.

### 4.3 Input schema (LLM-facing)

```json
{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "query": { "type": "string", "description": "Search query string.", "minLength": 1, "maxLength": 512 },
    "num_results": { "type": "integer", "minimum": 1, "maximum": 10, "description": "Number of results to return (default 5)." },
    "language": { "type": "string", "description": "BCP-47 language tag used as Google's `hl` parameter (e.g., 'en', 'pt-BR'). Defaults to the host system locale." },
    "site": { "type": "string", "description": "Optional site restriction (maps to Google's `siteSearch`, e.g., 'github.com'). Must pass the active workspace's domain policy (see §4.9)." }
  },
  "required": ["query"]
}
```

Design notes:

- No `search_type` parameter in MVP. Web-only.
- No `safe_search` in MVP (Google defaults to `off` for Custom Search). Add later if needed.
- All ranges match the Google API's own limits so validation cannot disagree with upstream.
- When `language` is omitted, the backend resolves the host system locale (via `sys-locale`) and passes it as `hl`. If the system locale cannot be determined, `hl` is omitted entirely and Google picks.

### 4.4 Output schema (LLM-facing and persisted JSON)

#### Success body

```json
{
  "query": "string (echoed back, normalized)",
  "count": "integer (number of results actually returned)",
  "results": [
    {
      "title": "string",
      "url": "string (absolute https/http URL)",
      "snippet": "string (plain text, HTML stripped, entities decoded)",
      "source_domain": "string (e.g., 'svelte.dev')"
    }
  ],
  "provider": "google_custom_search",
  "cached": "boolean (true when served from the on-disk cache — see §4.10)",
  "filtered_by_policy": "boolean (optional; true when workspace policy dropped one or more results — see §4.9)"
}
```

Contract:

- `results` is length-bounded by `num_results`.
- Empty results are a valid success (`count: 0`, `results: []`), not an error.
- No raw HTML, no tracking parameters in URLs (`utm_*` stripped).
- No timestamps in MVP (inconsistently available from the API).
- `provider` is included so consumers can distinguish sources later if we add more providers.
- `cached` lets the agent know whether it is reading live data; it is informational only and does not change how the LLM interprets the results.
- `filtered_by_policy` is omitted when no policy filtering occurred. When present and `true`, the frontend renders the neutral "filtered" banner (see `mockups.html` panel 05) and the LLM is free to try a different query.

#### Error body

Every failure path returns a JSON object with a stable machine-readable code so both the LLM and the frontend can branch without parsing free text:

```json
{
  "error": "string (human-readable, already localized where possible)",
  "error_code": "not_configured | disabled_by_flag | rate_limited | unauthorized | upstream_error | invalid_arguments | filtered_by_policy",
  "retryable": "boolean (true iff a retry of the same request could succeed — see table below)"
}
```

| `error_code` | Cause | Human message (example) | `retryable` | Frontend behavior |
|---|---|---|---|---|
| `not_configured` | `ToolCredentialsRepository` returned None (or malformed row) | "Web search is not configured. Add a Google Custom Search API key and Engine ID to enable this tool." | `false` | Render the actionable banner with "Open settings →" (panel 06). |
| `disabled_by_flag` | Feature flag off at registry level | "Web search is disabled. Set `CODER_LLM_TOOL_WEB_SEARCH=1` to enable." | `false` | Generic failure banner. Should be unreachable in user builds where the flag ships on. |
| `rate_limited` | HTTP 429 from Google | "Web search quota exceeded. Try again after the quota resets." | `true` | Generic failure banner; LLM may back off and retry. |
| `unauthorized` | HTTP 401 / 403 | "Web search credentials were rejected by Google. Verify the API key and Engine ID." | `false` | Banner + "Open settings →" action. |
| `upstream_error` | HTTP 5xx, parse failure, or network error | "Web search provider is temporarily unavailable." | `true` | Generic failure banner. |
| `invalid_arguments` | Schema/length/policy violation at the input layer | "Site 'pinterest.com' is not permitted by this workspace's policy." | `false` | Generic failure banner; LLM typically self-corrects on the next turn. |
| `filtered_by_policy` | Explicit `site` argument rejected by workspace policy (distinct from `invalid_arguments` because the request shape was valid) | "Site 'x' is not permitted by this workspace's policy." | `false` | Same as `invalid_arguments` at the UI layer; retained as a separate code so telemetry and LLM can distinguish policy friction from schema errors. |

**Enforcement rules**:

1. Every `Err(ToolError)` produced by `WebSearchTool::execute` maps to exactly one `error_code`. The mapping is a pure function tested in `web_search/tests/error_mapping_tests.rs` (added in Phase 3).
2. The `ToolRegistry` fallback `{"error": "Unknown tool name", "available_tools": [...]}` is **not** produced by this tool and carries no `error_code`; the frontend treats absence as a generic error.
3. Error codes are part of the stable contract. Adding a new code is a backwards-compatible change; renaming or removing one is a breaking change and requires a migration note in the changelog.
4. The human `error` string may be edited for clarity or translation without bumping any version — consumers must branch on `error_code`, never on substring matches of `error`.

### 4.5 Credentials model

The tool needs **two** secrets: the Google API key and the Custom Search Engine ID (`cx`). This is unlike the LLM `ApiKeyRepository` which stores a single key per provider.

**Decided**: credentials live in the database only. No environment-variable fallback.

- New table `tool_credentials` in `coder_config.db`, keyed by `tool_id TEXT PRIMARY KEY`, with a `credentials TEXT NOT NULL` column holding a JSON blob and `updated_at TEXT NOT NULL`.
- New trait `ToolCredentialsRepository` in `core/config`, implemented by `SqliteConfigRepository`, included in the combined `ConfigRepository` bound.
- Exposed via Tauri commands: `set_tool_credentials`, `get_tool_credentials`, `delete_tool_credentials`, `has_tool_credentials`, plus `test_web_search_credentials`.

When the DB row is missing and the feature flag is on, the tool returns the structured error body with `error_code: "not_configured"` (see §4.4 → Error body). The tool remains visible to the LLM (so the LLM can learn from the error), but it is never executed. The frontend reads the code and renders the "Open settings →" banner (`mockups.html` panel 06).

Security:

- API key never logged, never emitted in tool output, never sent to the frontend except through `get_tool_credentials` to pre-fill the settings form.
- The settings form shows the API key masked by default and offers a reveal toggle. The `cx` is not a secret and is shown in clear.
- Inside Rust the key is held in a `RedactedSecret` newtype (manual `Debug` printing `<redacted>`) so it cannot leak through structured logging.

### 4.6 Permission and approval behavior

- Default permission: **auto-run in all trust modes** (consistent with `search_grep` and `conversation_search_messages`; reads are trusted).
- No rollback snapshot (tool has no local side effects).
- Rate limit is enforced provider-side (100/day on free tier); we do not implement client-side quotas in MVP beyond failing cleanly on HTTP 429.
- YOLO mode (the repository's already-shipped fully-trusted mode) does not require any special handling for this tool — the auto-run default covers it.

### 4.7 Frontend surface

- Icon in `ToolCallMessage.svelte`: `Globe` from `lucide-svelte` (already imported).
- `getActionSummary` entry:
  - title: `"Searched web"`
  - subtitle: the `query`
- `getOutcomeSummary` entry:
  - success: `"Found N results"` (or `"No results"` when empty). When `cached` is `true` the outcome reads `"Found N results (cached)"`.
  - error: `"Search failed"`
- Expanded view renders each result as a **clickable row**: title (bold), URL (muted, truncated), snippet. Clicking the row (or pressing Enter when focused) opens the URL in the user's default browser via `tauri-plugin-opener`.
  - Links go through a small `$lib/api/shell.ts` wrapper that validates `http` / `https` before calling the plugin.
  - Respects keyboard accessibility: rows are `<button>` elements (not `<a href>`) so that Svelte controls the open path and we keep CSP-friendly behavior inside the Tauri webview.
- A compact results list component (`WebSearchResults.svelte`) renders inside the raw-data section; the structured parameter view above it remains unchanged.
- Settings UI: a new "Tools → Web Search" section on the existing Settings page with:
  - API key input (masked + reveal toggle), CX input, Save button.
  - "Test" button that runs a canned query via `test_web_search_credentials`.
  - Per-workspace domain policy editor (see §4.9).
  - Cache controls (see §4.10): clear cache, cache entry count, default TTL.

### 4.8 Observability

Use the existing tool telemetry pipeline (`ToolCall` / `ToolResult` runtime events, persisted in `agent_runtime_tool_calls` — see `docs/features/agent-session-runtime-persistence.md`). No new telemetry tables.

Logged at INFO (via `log::info!`) on every execution:

- **Full query string** (the user has chosen full-query logging; keep the query field clearly labeled in structured logs so it can be scrubbed downstream if policy changes).
- Result count.
- Cache hit flag.
- Upstream HTTP status (when a network call was actually made).
- Round-trip latency in milliseconds.

Logged at WARN on HTTP 4xx/5xx or quota exhaustion, with the upstream error code. The API key is **never** written to a log line — it is held in `RedactedSecret` whose `Debug`/`Display` impls print `<redacted>`.

### 4.9 Per-workspace domain policy

The workspace owner can restrict which domains web search may target. Policies are scoped to the **canonicalized `workspace_root` path** of the active agent session (the codebase has no separate Workspace entity today; the path is the identity). Sessions without a workspace root fall back to the global default policy (no restrictions).

**Data model** (new table in `coder_config.db`):

```sql
CREATE TABLE workspace_search_policy (
    workspace_path TEXT PRIMARY KEY,     -- canonicalized absolute path, lowercased on case-insensitive filesystems
    allowlist TEXT NOT NULL,             -- JSON array of domain patterns, empty = no allowlist
    denylist  TEXT NOT NULL,             -- JSON array of domain patterns, empty = no denylist
    updated_at TEXT NOT NULL
);
```

Domain patterns support:

- Exact host match (`github.com`)
- Subdomain match with leading dot (`.example.com` matches any subdomain including the bare host)
- `*` as the full pattern means "any domain" (only meaningful in an allowlist)

**Enforcement rules** (applied inside the tool before the provider is called, and again when filtering results):

1. If the LLM passed an explicit `site`, it must match the allowlist (when allowlist is non-empty) and must not match the denylist. Otherwise the tool returns an error body with `error_code: "filtered_by_policy"` (see §4.4 → Error body). This is distinct from `invalid_arguments` so telemetry and the LLM can distinguish policy friction from schema errors.
2. When no `site` is given and the allowlist is non-empty, the tool rewrites the provider request to a `site:` OR-expansion of all allowlisted domains.
3. After results come back, any URL whose `source_domain` is on the denylist (or not on a non-empty allowlist) is dropped. `count` reflects the filtered list. When one or more results are dropped, the success body sets `filtered_by_policy: true`. If filtering empties the list, the tool still returns success (with `count: 0` and `filtered_by_policy: true`), not an error — the LLM decides whether to retry with a different query.
4. The policy is consulted **before** the cache layer, so cache hits also respect current policy (a policy change invalidates their visibility even if the cache entry remains).

Policies are managed through new Tauri commands:

- `get_workspace_search_policy(workspace_path)`
- `set_workspace_search_policy(workspace_path, allowlist, denylist)`
- `delete_workspace_search_policy(workspace_path)`

The Settings UI exposes the policy editor only when the Settings page is opened from within a workspace-scoped context.

### 4.10 Search result caching (DB-backed)

Repeat queries within a short window are served from a local cache to save quota and latency.

**Data model** (new table in `coder_config.db`, kept with the other tool-config rows for lifecycle simplicity):

```sql
CREATE TABLE web_search_cache (
    cache_key TEXT PRIMARY KEY,          -- sha256 of the normalized request
    query TEXT NOT NULL,
    num_results INTEGER NOT NULL,
    language TEXT,
    site TEXT,
    workspace_path TEXT,                 -- scopes cache per workspace (policy-sensitive)
    provider TEXT NOT NULL,              -- "google_custom_search"
    results TEXT NOT NULL,               -- JSON array of WebSearchResult
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX idx_web_search_cache_expires ON web_search_cache(expires_at);
```

**Cache key**: SHA-256 of `format!("{}\n{}\n{}\n{}\n{}", query_normalized, num_results, language_or_empty, site_or_empty, workspace_path_or_empty)`. Querying from a different workspace yields a different key even if the text matches; this keeps per-workspace policy filtering sound without cross-workspace bleed.

**TTL**: default 24 hours, configurable in Settings (range 5 min – 7 days). Expired rows are lazily dropped on read and swept on app startup.

**Lookup flow** inside `WebSearchTool::execute`:

1. Normalize request.
2. Resolve workspace policy; reject or rewrite the request per §4.9.
3. Build cache key and look up.
4. If a live (non-expired) entry exists, return it with `cached: true`; skip the provider call.
5. Otherwise call the provider, filter by policy, persist the cache entry, return with `cached: false`.

**Cache management**:

- Settings UI shows entry count + a "Clear cache" action.
- Tauri command `clear_web_search_cache(workspace_path: Option<String>)` — clears per-workspace or the whole cache.
- No background sync or prefetch; the cache is strictly reactive.

**Cache and policy interaction**: when a workspace policy is updated, the per-workspace cache partition for that path is **not** invalidated automatically. Policy is always re-applied to cached results on read (rule 4 in §4.9), so stale cache entries are filtered through the current policy just like live results. This avoids thrashing the cache when the user is editing policy interactively.

## 5. Non-functional requirements

| Area | Requirement |
|---|---|
| Latency | p95 < 2 s. Implemented with a 10 s hard request timeout; surface timeout errors cleanly. |
| Availability | On API failure, return a structured `ToolError` — the agent loop already handles tool failures without crashing the conversation. |
| Security | API key stored via SQLite config repo; never traverses the IPC boundary to frontend chat; never persisted in tool output payloads. |
| Token efficiency | Snippets capped at ~300 chars each after HTML stripping; `num_results` hard-capped at 10; no raw HTML leaves the tool. |
| Provider portability | Everything API-specific lives in `infrastructure/tools/search/web_search/provider/google.rs`; the `Tool` impl depends only on a trait. |

## 6. Architecture (brief)

```
LLM
 └── ToolRegistry.definitions()          src-tauri/src/infrastructure/tools/mod.rs
 └── ToolRegistry.execute("search_web_search")
       └── WebSearchTool                 src-tauri/src/infrastructure/tools/search/web_search/mod.rs
             ├── load credentials        ToolCredentialsRepository (core/config)
             └── SearchProvider trait    provider/mod.rs
                   └── GoogleProvider    provider/google.rs  (reqwest → Custom Search JSON API)
```

The `SearchProvider` trait is the seam that satisfies PRD §11 ("must allow swapping the search provider").

## 7. Success metrics

| Metric | Target | Measured via |
|---|---|---|
| Agent answers requiring fresh info actually call the tool | > 60% of such turns | Runtime tool-call telemetry + manual evals |
| Median tool latency | < 1.2 s | `log::info!` latency field |
| Tool-call error rate | < 5% excluding intentional quota exhaustion | `is_error` flag in `ToolResult` persistence |
| Hallucination reduction on current-events prompts | Qualitative improvement | Internal eval set before/after |

## 8. Risks and mitigations

| Risk | Mitigation |
|---|---|
| API quota exhausted mid-session | Surface HTTP 429 as a clear `ToolError` so the agent can degrade gracefully. Show remaining-quota message in settings when possible. |
| Prompt injection via snippets | Treat results as untrusted data in the LLM context. Strip HTML and scripts at the tool boundary. Never let snippet content modify system prompts. |
| User commits credentials accidentally | DB-backed storage lives under app data dir; no file in the repo. Env-var fallback is documented as dev-only. |
| Provider deprecation | Trait-based abstraction isolates the Google-specific code. A future `BingProvider` impl is a drop-in. |
| ToS for result redistribution | Results are shown to the individual user in their own session; not cached, not republished. Keep documentation explicit about this. |

## 9. Resolution log (decisions from the user, 2026-04-14)

All open questions from the original draft have been answered. The decisions are captured here in the spec body above; this section is the historical trail.

| # | Question | Decision |
|---|---|---|
| 1 | Credentials storage: DB, env-var, or both? | **DB only.** No env fallback. Tool errors clearly when unconfigured. (§4.5) |
| 2 | Domain filtering: LLM-controlled or policy-enforced? | **Per-workspace policy** (allowlist + denylist), keyed by canonicalized `workspace_root`. (§4.9) |
| 3 | Approval on first use | **Auto-run** — covered by existing YOLO / trust mode system; no special handling. (§4.6) |
| 4 | Language default | **System locale** (via `sys-locale` crate) when `language` is omitted; fall through to Google default only if locale is unavailable. (§4.3) |
| 5 | Clickable result links in chat | **Yes**, via `tauri-plugin-opener`. Results become clickable rows in the `ToolCallMessage` expanded view. (§4.7) |
| 6 | Query in logs | **Full query logged** at INFO. API keys never logged. (§4.8) |
| 7 | Caching | **Yes, in SQLite.** Default 24h TTL, per-workspace partitioning, policy re-applied on read. (§4.10) |

If any of these decisions are revisited, update the decision row here and the referenced section above in the same change; do not leave the spec and the log out of sync.

## 10. Acceptance criteria

The feature is ready to merge when **all** are true:

- [ ] `search_web_search` is registered and gated by `CODER_LLM_TOOL_WEB_SEARCH`.
- [ ] With flag on and credentials present, the LLM can call it and receive the documented JSON output.
- [ ] With flag off, the tool is invisible to `ToolRegistry::definitions()`.
- [ ] With flag on but credentials missing, a clear actionable error is returned; the tool is still listed so the LLM can reason about its unavailability.
- [ ] Credentials are persisted in `coder_config.db` (`tool_credentials` table) and are the only source of truth — there is no env-var fallback.
- [ ] Per-workspace domain policy is enforced for both explicit `site` arguments and post-hoc result filtering; a policy update is visible on the very next tool call (applied to cached results too).
- [ ] System locale is used as the default `hl`; explicit `language` argument overrides it.
- [ ] All argument validation paths have unit tests.
- [ ] Provider layer is abstracted behind a `SearchProvider` trait with a mock implementation used in tests.
- [ ] HTTP errors (4xx, 5xx, 429, network timeout) surface as structured error bodies with the correct stable `error_code` (`rate_limited`, `unauthorized`, `upstream_error`, etc.) per the mapping table in §4.4.
- [ ] Unit test `web_search/tests/error_mapping_tests.rs` covers every `ToolError` → `error_code` transition and is green.
- [ ] Result caching is exercised: identical request within TTL skips the network call and sets `cached: true`.
- [ ] Frontend renders `Searched web` summary and the correct outcome text (including `(cached)` suffix and `No results` variant).
- [ ] Clicking a result row in the chat opens the URL via `tauri-plugin-opener`; non-http(s) URLs are refused by the wrapper.
- [ ] Settings UI saves, retrieves, and clears credentials; edits policy; clears the cache. "Test" button calls the live API successfully.
- [ ] API key never appears in logs, chat messages, persisted telemetry, or frontend state beyond the masked settings input.
- [ ] Query string appears in INFO logs (per decision §4.8). Verified by grepping a dev run.
- [ ] `docs/tools-and-agent-integration.md` tool catalog is updated.
- [ ] `CHANGELOG.md` entry added.

---

## Appendix A — Original PRD (as received)

> Product Requirements Document (PRD) — Web Search Tool for LLM Agent.
>
> The original PRD is preserved below verbatim for traceability.

### 1. Overview

This feature enables an LLM-powered agent to retrieve up-to-date information from the public web by invoking a structured "web search" tool. The tool integrates with a Google-provided search API and returns normalized search results that the agent can use to answer user queries.

### 2. Goals

- Allow the LLM to perform real-time web searches via tool calling.
- Provide structured, reliable search results for downstream reasoning.
- Maintain a clean abstraction so the underlying search provider can be replaced.
- Secondary: filtering by domain/language, safe access, observability.

### 3. Non-goals

- Rendering full web pages or rich media in the UI.
- Acting as a general-purpose browser.
- Replacing dedicated search engines for end users.
- Deep crawling or indexing.

### 5. Functional requirements (summarized)

- Tool name: `web_search`.
- Inputs: query (required), number of results (bounded), optional domain restriction, optional language, optional search type (web/image).
- Validation before execution.
- Backend receives request → validates/sanitizes → translates to Google API call → executes server-side → handles errors/retries.
- Response normalization: title, URL, snippet, source domain. Limit number of results. Consistent formatting. No HTML.

### 6-12. Architecture, safety, performance, observability, scalability, extensibility, risks

- Isolate external provider behind abstraction.
- Limit query length, reject malformed input, enforce rate limits.
- Treat external data as untrusted; avoid HTML rendering; prevent injection.
- API keys never exposed to client or LLM; all calls server-side.
- Target latency under 1–2 seconds; graceful degradation; timeout handling.
- Log invocation frequency, sanitized parameters, latency, error rate, result count.
- Caching optional; concurrent calls supported; provider-swappable.

### 13. Original open questions

- Should results be reranked internally before returning?
- Should the tool include metadata like timestamps?
- Should domain filtering be controlled by the LLM or restricted by policy?
- Should we support multiple providers simultaneously?

*These questions are carried forward into §9 above with recommendations.*
