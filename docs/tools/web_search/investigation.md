# Web Search Tool — Codebase Investigation

**Status**: PLAN
**Last reviewed**: 2026-04-14
**Purpose**: Map the new `search_web_search` tool onto the existing tool system with file/line evidence, so the implementer can extend each touchpoint without hunting for the right seam.

---

## 1. Tool contract and registry

**Trait & metadata**: `src-tauri/src/infrastructure/tools/traits.rs:18-127`

```rust
pub trait Tool: Send + Sync {
    fn meta(&self) -> ToolMeta;
    fn execute(&self, ctx: ToolExecutionContext, args: JsonValue,
               app_handle: AppHandle, workspace_root: Option<PathBuf>) -> ToolFuture;
}
```

`ToolMeta` (line 47-60) carries `id: ToolId`, `description`, `input_schema`, `requires_workspace`, `experimental`, `feature_flag`.

**Category trait**: same file, lines 134-143. `ToolCategory::tools()` returns `Vec<Box<dyn Tool>>`.

**Registry assembly**: `src-tauri/src/infrastructure/tools/mod.rs:49-68`

```rust
registry.register_category(search::SearchCategory);
```

**Gating order in the registry**: `mod.rs:110-172`

1. Lookup by `call.name` — `Unknown tool name` error if absent.
2. `meta.feature_flag` → `is_env_enabled(flag)` → `Tool disabled. Set CODER_...=1` if unset.
3. `meta.requires_workspace` check.
4. `tool.execute(..)` — errors are wrapped in a `ToolResult { is_error: true }` with `{"error": "..."}`.

Implication: we do not need to write our own gating logic; we just declare `feature_flag: Some(ENV_WEB_SEARCH_ENABLED)`.

## 2. Existing search category — direct template

**Category**: `src-tauri/src/infrastructure/tools/search/mod.rs:1-25`

```rust
pub struct SearchCategory;

impl ToolCategory for SearchCategory {
    fn name(&self) -> &'static str { "search" }
    fn description(&self) -> &'static str { "Workspace and web search tools" }
    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![Box::new(GrepTool), Box::new(GoogleScrapeTool)]
    }
}
```

**Closest template**: `src-tauri/src/infrastructure/tools/search/google_scrape.rs`

Relevant building blocks already proven there:

- Argument struct + defaults (`GoogleSearchArgs`, lines 16-30).
- `SearchResult` / `GoogleSearchOutput` shape (lines 33-56) — intentionally similar to the shape we want.
- `ToolMeta` construction (lines 62-91) including `feature_flag: Some(ENV_GOOGLE_SCRAPE_ENABLED)`.
- `reqwest::Client::builder().timeout(Duration::from_secs(12))` pattern (lines 128-135).
- HTTP error → `ToolError::from_reqwest` (line 143).
- HTML entity decoding + tag stripping helpers (lines 177-195) — we can reuse the *decoding* logic; Google's JSON API already returns HTML in snippets via `<b>` highlights, so stripping is still needed.
- Dense unit test module (lines 281-839) gated by `#[cfg(all(test, feature = "integration-test"))]`. Note: at 839 lines this file is past the 500-line soft limit and approaching the 800-line hard limit; our new tool must be split into `web_search/{mod.rs, provider/google.rs, tests.rs}` from day one (see `plan.md`).

**Constants file**: `src-tauri/src/infrastructure/tools/constants.rs:38-92`

Already reserves a Web Search section:

```rust
pub const DEFAULT_WEB_SEARCH_LIMIT: usize = 5;
```

We add:

```rust
pub const MAX_WEB_SEARCH_LIMIT: usize = 10;
pub const MAX_WEB_SEARCH_QUERY_LEN: usize = 512;
pub const ENV_WEB_SEARCH_ENABLED: &str = "CODER_LLM_TOOL_WEB_SEARCH";
pub const ENV_WEB_SEARCH_API_KEY: &str = "CODER_LLM_TOOL_WEB_SEARCH_API_KEY";
pub const ENV_WEB_SEARCH_CX: &str = "CODER_LLM_TOOL_WEB_SEARCH_CX";
```

All in the same existing pattern, no new surface.

## 3. Error model

**Types**: `src-tauri/src/infrastructure/tools/error.rs:8-45`

Already covers what we need:

| Use site | Variant |
|---|---|
| Bad query / bad num_results | `InvalidArguments` |
| Feature flag off | `Disabled` |
| Credentials missing / upstream 4xx | `ExecutionFailed` or `Http` |
| Network / timeout | `Http` (via `ToolError::from_reqwest`) |

No new variants required.

**Argument parsing helper**: `error.rs:89-95`

```rust
let args: WebSearchArgs = parse_args("search_web_search", arguments)?;
```

## 4. Tool execution context

**File**: `src-tauri/src/infrastructure/tools/context.rs:10-54`

`ToolExecutionContext` provides `conversation_id`, `thread_id`, `workspace_root`, `session_id`. Web search does not need any of them for correctness — but we will include `conversation_id` in the telemetry log line for correlation with runtime events.

No changes needed here.

## 5. Credentials storage

**Current pattern (LLM providers)**: `src-tauri/src/core/config/repositories.rs:85-95`

```rust
pub trait ApiKeyRepository: Send + Sync {
    fn store_api_key(&self, provider_id: &str, api_key: &str) -> RepositoryResult<()>;
    fn retrieve_api_key(&self, provider_id: &str) -> RepositoryResult<String>;
    fn delete_api_key(&self, provider_id: &str) -> RepositoryResult<()>;
    fn has_api_key(&self, provider_id: &str) -> RepositoryResult<bool>;
}
```

**Tauri commands**: `src-tauri/src/commands/config/api_keys.rs:103-151`

**State access from tools**: `src-tauri/src/infrastructure/tools/conversation/search_messages.rs:109-113`

```rust
let chat_state = app_handle.state::<ChatState>();
let history = chat_state.service.load_messages(&context.conversation_id)?;
```

This is the pattern we'll use to read `ConfigState` inside `WebSearchTool::execute`:

```rust
let config_state = app_handle.state::<ConfigState>();
let creds = config_state.repository.get_tool_credentials("search_web_search")?;
```

**Gap**: the Google Custom Search JSON API needs **two** secrets (`api_key` + `cx`). The single-string `ApiKeyRepository` is insufficient.

**Decided path (2026-04-14)**: new `ToolCredentialsRepository` trait backed by a new SQLite table `tool_credentials (tool_id PRIMARY KEY, credentials TEXT NOT NULL, updated_at)`. No env-var fallback. The tool returns a structured "not configured" error when the row is missing; the Settings UI is therefore a hard prerequisite for using the tool at runtime. See `spec.md §4.5` and `plan.md Phase 5`.

## 6. Agent/LLM orchestration touchpoints

**Loop runner**: `src-tauri/src/commands/llm/orchestrator/loop_runner.rs:196-224`

Iterates over `llm_tool_calls`, calls `tool_executor.execute(call, app_handle)`, appends `ChatMessage::tool_result(call_id, output, is_error)` back into `core_messages`. The output is exactly what the tool returned. So our output schema is what the LLM sees — we need it to be token-efficient.

**Definitions exposed to LLM**: `src-tauri/src/application/agent/execution/context_phase.rs` — pulls from `ToolRegistry::definitions()` and filters by subtask mode. No change required on our side; the registry does the work.

**Response handling**: `src-tauri/src/application/agent/execution/tool_phase.rs` — permission-aware execution, event emission. Web search is a read-only tool; it will auto-run in all trust modes per the default in `permissions.rs`.

**Runtime telemetry**: `src-tauri/src/infrastructure/agent/events.rs`, `src-tauri/src/application/agent/service/runtime_queries.rs`. Standard `ToolCall` / `ToolResult` events fire for every tool via the permissions layer — no extra work.

## 7. Persistence and chat surfacing

**Chat message storage**: `coder_chat.db` via `src-tauri/src/infrastructure/sqlite/migrations/chat_schema.rs` (tables `conversations`, `chat_messages`). Tool calls ride in the `chat_messages.metadata` JSON column under keys `toolName`, `toolArgs`, `toolResult`, `toolCallStatus` — consumed by `ToolCallMessage.svelte`.

**Agent runtime persistence**: see `docs/features/agent-session-runtime-persistence.md` for the `agent_runtime_*` tables. No schema change needed; web search events look identical to any other tool invocation.

## 8. Frontend integration points

**Component**: `src-ui/src/lib/components/chat/messages/ToolCallMessage.svelte:48-67`

Tool icon map (already imports `Globe` from `lucide-svelte`):

```ts
const toolIconMap: Record<string, any> = {
    fs_read_file: FileText,
    // ...
    web_request: Globe,
    web_fetch: Globe,
    // add:
    search_web_search: Globe,
};
```

Summaries at lines 81-145 — add two cases:

```ts
case 'search_web_search':
    return { title: 'Searched web', subtitle: args.query };
// and for outcomes:
case 'search_web_search':
    const count = res?.count ?? (res?.results?.length ?? 0);
    return count === 0 ? 'No results' : `Found ${count} results`;
```

No new Svelte component needed. The existing expandable raw-JSON view (lines 272-290) already renders the structured output.

**Settings page**: the providers/settings page already exists (LLM API keys use `api_keys.rs` commands). We add a sibling section for "Tools → Web Search" with two inputs (API key, CX) and a Test button. The Settings component file list can be resolved quickly with `Grep -l 'store_api_key' src-ui/`; the plan treats the exact path as an implementation detail because the Settings area is currently evolving.

**Tests**: `src-ui/src/lib/components/chat/messages/ToolCallMessage.test.ts` already exists — add cases for the new tool name and summaries.

## 9. Testing patterns we'll follow

Rust unit tests live in `#[cfg(test)] mod tests` next to the code. HTTP-hitting tests are gated with `#[cfg(all(test, feature = "integration-test"))]` (see `google_scrape.rs:281`). Pure parsing/validation tests stay in the default `cfg(test)` so they always run.

Trait-seam testing: we introduce `SearchProvider` trait so provider-level tests can substitute a mock `reqwest`-free implementation. The `Tool::execute` path is then covered end-to-end without the network:

```rust
trait SearchProvider: Send + Sync {
    async fn search(&self, req: SearchRequest) -> Result<Vec<RawResult>, SearchProviderError>;
}

// real: GoogleProvider { client: reqwest::Client, api_key, cx }
// test: MockProvider { canned_responses, observed_calls }
```

Frontend tests use Vitest + `@testing-library/svelte` — same pattern as `ToolCallMessage.test.ts`.

## 10. Workspace identity (for per-workspace policy and cache partitioning)

The codebase has no separate `Workspace` entity. The identity is the `workspace_root: Option<PathBuf>` carried on:

- `ToolExecutionContext` — `src-tauri/src/infrastructure/tools/context.rs:18`
- Agent session — `src-tauri/src/core/agent/session.rs:60`

For both the `workspace_search_policy` and `web_search_cache` tables we use the **canonicalized absolute path** as the key, lowercased on case-insensitive filesystems. Sessions without a `workspace_root` fall back to the global default policy (no restrictions) and share an unscoped cache partition keyed with the empty string. This keeps the schema simple and avoids introducing a new Workspace entity purely for this feature.

## 11. Dependencies to add

Not currently in `src-tauri/Cargo.toml` (verified by reading Cargo.toml 2026-04-14):

| Crate | Reason |
|---|---|
| `async-trait = "0.1"` | `SearchProvider` trait uses `async fn`. |
| `sys-locale = "0.3"` | Default `hl` to the host system locale. |
| `sha2 = "0.10"` | Cache key hashing. |
| `tauri-plugin-opener = "2"` | Open result URLs in the user's default browser. |
| `wiremock = "0.6"` (dev-dependency) | Offline tests for `GoogleProvider`. |

The plugin must also be declared in the Tauri capabilities file so the webview is allowed to invoke it. Scope capability to `http://*` and `https://*` only; do not expose file/shell schemes.

## 12. New SQLite migrations

Three new tables in `coder_config.db`:

| Table | Purpose | Defined in spec |
|---|---|---|
| `tool_credentials` | API key + CX JSON blob per tool id | §4.5 |
| `workspace_search_policy` | Allow/deny domain lists per canonical workspace path | §4.9 |
| `web_search_cache` | Cached provider results with TTL, partitioned per workspace | §4.10 |

Migration files go under `src-tauri/src/infrastructure/sqlite/migrations/`, following the existing migration module pattern (see `chat_schema.rs` for the shape). Each migration is a pure additive DDL; no backfills required.

## 13. File size planning

`src-tauri/src/infrastructure/tools/search/google_scrape.rs` is 839 lines today (tests included). Our repo soft limit is 500 and hard is 800. We explicitly avoid reproducing that mistake:

```
src-tauri/src/infrastructure/tools/search/web_search/
├── mod.rs                # Tool impl, ~200 lines target
├── types.rs              # Args / Output / RawResult structs, ~80 lines
├── provider/
│   ├── mod.rs            # SearchProvider trait + SearchProviderError, ~60 lines
│   └── google.rs         # GoogleProvider (reqwest), ~180 lines
├── credentials.rs        # DB+env credentials resolver, ~100 lines
└── tests/
    ├── mod.rs
    ├── args_tests.rs
    ├── credentials_tests.rs
    ├── provider_google_tests.rs   # offline parsing; HTTP gated behind feature
    └── tool_execute_tests.rs      # uses MockProvider
```

## 14. Cross-cutting references

| Concern | Authoritative doc |
|---|---|
| Tool system overview | `docs/tools-and-agent-integration.md` |
| Agent runtime persistence | `docs/features/agent-session-runtime-persistence.md` |
| Streaming message IDs (our tool is not a streaming emitter, but responses must respect the chat store contract) | `docs/features/agent-streaming-message-ids.md` |
| SQLite access | `docs/database-reference.md` |
| Rust backend quality bar | `docs/quality/guidelines/rust-backend.md` |
| Svelte/TypeScript quality bar | `docs/quality/guidelines/svelte.md`, `typescript.md` |

Anything not covered above is considered standard repository practice (see `AGENTS.md` reading order).
