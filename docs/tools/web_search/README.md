# Web Search Tool — Planning Index

**Status**: DRAFT
**Last reviewed**: 2026-04-14
**Purpose**: Entry point for the `search_web_search` tool feature. Collects spec, codebase investigation, and the TDD task plan in one place.

---

## Documents in this folder

| Document | Status | Purpose |
|---|---|---|
| `spec.md` | DRAFT | Refined feature specification derived from the original PRD. Source of truth for scope, schema, and non-goals. |
| `investigation.md` | DRAFT | Codebase investigation with file/line evidence. Maps the tool to the existing tool system, persistence, UI, and tests. |
| `plan.md` | DRAFT | Test-first implementation plan, broken into phases with concrete test lists per step. |
| `mockups.html` | DRAFT | Self-contained visual reference for the chat-area and settings components. Open directly in a browser — no build step. Matches `design-system.css` dark-theme tokens. |

The source PRD is preserved verbatim in `spec.md → Appendix A` so that scope changes can be traced back to the original ask.

---

## MVP summary

> One sentence: expose a `search_web_search` Rust tool backed by the Google Custom Search JSON API, gated by a feature flag, with DB-stored credentials, per-workspace domain policies, SQLite-backed result caching, system-locale defaults, and clickable results that open in the user's default browser.

Out of scope for MVP (see `spec.md → Non-goals`):

- Image search, news search, semantic rerank
- Multi-provider dispatch
- In-app rendering of pages (fetch-and-read is a separate `fetch_url` tool, already hinted at in the PRD)

All seven open questions from the original PRD are resolved in `spec.md §9`.

---

## Required reading for anyone picking this up

1. `AGENTS.md`
2. `RULES.md`
3. `docs/quality/guidelines/README.md` and each touched-area guide (`rust-backend.md`, `typescript.md`, `svelte.md`, `configuration.md`)
4. `docs/tools-and-agent-integration.md` — existing tool system reference
5. This folder: `spec.md` → `investigation.md` → `plan.md`

---

## Status transitions

| Phase | Action |
|---|---|
| Open questions resolved | Move all three docs from DRAFT → PLAN, start implementation |
| Implementation complete | Mark `plan.md` as COMPLETE with date, move to `docs/archive/`; promote `spec.md` to ACTIVE as living reference; update `docs/tools-and-agent-integration.md` catalog |
| Superseded or abandoned | Archive the whole folder under `docs/archive/tools/web_search/` |
