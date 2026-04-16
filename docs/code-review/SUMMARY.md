# Code Review Summary — `src-tauri/src/core/`

**Date:** 2026-04-16
**Reviewer:** Rust Audit Pipeline
**Branch:** `feature/pdf-authoring-tool`

---

## Scope

All 42 Rust source files under `src-tauri/src/core/`, covering:

| Module | Files | Purpose |
|--------|-------|---------|
| `agent/` | 12 files (incl. 3 test files) | Agent sessions, tasks, permissions, execution tracing |
| `chat/` | 4 files | Conversations, threads, messages |
| `config/` | 2 files | Provider/model/API key configuration |
| `documents/pdf/` | 8 files | PDF document model (blocks, sections, themes, geometry) |
| `llm/` | 4 files | LLM entities, client traits, service interfaces |
| `nlp/` | 7 files (incl. 1 test file) | Intent classification, multi-intent, ambiguity |

## Files Reviewed Deeply

| File | Reason Selected |
|------|----------------|
| `agent/entities.rs` | Contains duplicated domain types; re-exported as public API |
| `agent/session.rs` | Parallel session types; state machine with no transition validation |
| `agent/task.rs` | Parallel task types; divergent tool permission logic |
| `agent/types.rs` | Third location for agent types; different structural shape |
| `agent/mod.rs` | Re-export structure determines which duplicate wins |
| `core/mod.rs` | Wildcard re-export creates name collision risk |
| `nlp/patterns.rs` | 50+ `Regex::new().unwrap()` in production code path |
| `chat/entities.rs` | Name collisions with LLM entities; `from_str` shadows |
| `config/mod.rs` | `from_str` shadow pattern; otherwise clean |
| `llm/entities.rs` | Parallel `ChatMessage`/`MessageRole` types |

## Triage Results

- **unsafe blocks:** 0
- **FFI markers:** 0
- **Concurrency primitives (Mutex, RwLock, channels):** 0 (core is a pure domain layer)
- **`unwrap()`/`expect()` in non-test code:** ~50 (all in `nlp/patterns.rs` regex construction)
- **`todo!`/`unimplemented!`:** 0

## Findings Summary

| # | Severity | Category | Title |
|---|----------|----------|-------|
| F1 | High | Maintainability / Correctness | Duplicate type system in agent module |
| F2 | Medium | Correctness Risk | No state machine validation for session transitions |
| F3 | Medium | API Design Smell | Wildcard re-export of chat entities |
| F4 | Medium | Correctness Risk | Regex `unwrap()` in production code path |
| F5 | Low | API Design Smell | Inherent `from_str` shadows `std::str::FromStr` |
| F6 | Low | Correctness Risk | `TaskMode` vs `SubTaskMode` tool list divergence |

## Resolution Summary
- Fixes implemented: 3 of 3 planned
- Fixes deferred: 3 (F1: too large for audit scope, F5/F6: low severity)
- Tests added: 16
- Frontend changes required: No

### Fix Log
| # | Finding | Status | Tests Added | Notes |
|---|---------|--------|-------------|-------|
| 1 | F2 — State machine validation | Fixed | 16 | Added `can_transition_to()` + `variant_name()` on `SessionState`, `set_state()` now returns `Result` |
| 2 | F3 — Wildcard re-export | Fixed | 0 | Replaced `pub use chat::entities::*` with explicit list; compile-time verified |
| 3 | F4 — Regex LazyLock | Fixed | 0 | Patterns compiled once via `std::sync::LazyLock`; all 31 existing tests pass |
