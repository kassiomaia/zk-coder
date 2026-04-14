# Web Search Tool — Living Index

**Status**: ACTIVE
**Last reviewed**: 2026-04-14
**Shipped**: 2026-04-14 on branch `feature/web-search-tool` (Phases 0 – 10)

The `search_web_search` tool is a production Google Custom Search JSON API
wrapper with per-workspace domain policy, SQLite-backed result caching, and
structured error bodies. Gated by the `CODER_LLM_TOOL_WEB_SEARCH` feature
flag and DB-stored credentials managed through **Settings → Tools → Web Search**.

---

## Documents in this folder

| Document | Status | Purpose |
|---|---|---|
| `spec.md` | ACTIVE | Living contract: scope, input/output schemas, error taxonomy, per-workspace policy rules, cache semantics. |
| `investigation.md` | PLAN (frozen) | Codebase investigation produced before implementation; retained for traceability. |
| `mockups.html` | ACTIVE | Self-contained visual reference for every chat-area and settings component. Open directly in a browser. Mirrors `design-system.css` dark-theme tokens. |
| `plan.md` | ARCHIVED — see `docs/archive/tools/web_search/plan.md` | The original TDD phase plan. Kept for history; should not be used to drive new work. |

The source PRD is preserved verbatim in `spec.md → Appendix A`.

---

## What shipped

- **Rust tool** (`src-tauri/src/infrastructure/tools/search/web_search/`) — registered in `SearchCategory::tools()` when the flag is on.
- **DB schema** — three new tables in `coder_config.db`: `tool_credentials`, `workspace_search_policy`, `web_search_cache`. Added via an additive `CREATE TABLE IF NOT EXISTS` in `config_schema::init_schema`; existing user databases pick them up on the next launch with no manual migration.
- **Tauri commands** — 11 commands under `commands/config/` (tool credentials, workspace policy, cache stats/clear, live-test credentials).
- **Frontend** — Settings UI (`src-ui/src/lib/components/settings/web-search/`) and the `search_web_search` renderer inside `ToolCallMessage` (`src-ui/src/lib/components/chat/messages/tool-renderers/`).
- **Startup sweep** — `lib.rs` purges expired cache entries on every app launch; failures log and never block the app.

---

## Operational notes

- The tool is hidden from the LLM catalog until both the feature flag is on **and** credentials are saved — this is intentional: see `spec.md §4.2`.
- Settings UI changes (rotated API key, updated allow/deny lists, cleared cache) take effect on the **very next tool call** without re-registering the tool; credentials are resolved per-execute.
- Cache entries are re-filtered through the **current** policy on read. A tightened denylist drops previously-allowed cached results; a loosened policy does **not** resurrect results that were filtered out at write time.

---

## Required reading before editing this area

1. `AGENTS.md`
2. `RULES.md`
3. `docs/quality/guidelines/README.md` and the touched-area guide (Rust / TypeScript / Svelte)
4. `docs/tools-and-agent-integration.md` — general tool-system reference
5. `spec.md` in this folder — the living contract

---

## Superseded / abandoned path

If this tool is ever replaced or removed, archive the whole folder under `docs/archive/tools/web_search/` (the `plan.md` file is already there).
