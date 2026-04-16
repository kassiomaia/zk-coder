# Code Review Summary — `src-tauri/src/`

**Date:** 2026-04-16 (last full audit: 2026-04-16, revalidated: 2026-04-16)
**Reviewer:** Rust Audit Pipeline
**Branch:** `feature/pdf-authoring-tool`

---

## Scope

All ~309 Rust source files under `src-tauri/src/`, covering:

| Layer | Module | Files | Purpose |
|-------|--------|-------|---------|
| Core | `agent/` | 12 | Agent sessions, tasks, permissions, execution tracing |
| Core | `chat/` | 4 | Conversations, threads, messages |
| Core | `config/` | 2 | Provider/model/API key configuration |
| Core | `documents/pdf/` | 8 | PDF document model (blocks, sections, themes, geometry) |
| Core | `llm/` | 4 | LLM entities, client traits, service interfaces |
| Core | `nlp/` | 7 | Intent classification, multi-intent, ambiguity |
| Application | `agent/execution/` | 12 | Execution coordinator, bootstrap, phases, step gate |
| Application | `agent/service/` | 10 | Agent service operations (sessions, tasks, spawn, recovery) |
| Application | `chat/` | 4 | Chat service operations |
| Application | `llm/` | 5 | LLM service, client factory, conversions |
| Application | `nlp/` | 8 | Classifier, code analyzer, context manager, gateway |
| Application | `tools/pdf/` | 3 | PDF adapter, finalize service |
| Commands | `agent/` | 8 | Agent Tauri IPC handlers |
| Commands | `chat/` | 5 | Chat Tauri IPC handlers |
| Commands | `config/` | 10 | Config Tauri IPC handlers |
| Commands | `llm/` | 7 | LLM orchestrator, streaming |
| Commands | `documents/` | 1 | Document artifact commands |
| Infrastructure | `agent/runtime/` | 15 | Agent runtime, spawn engine, permissions, tool execution |
| Infrastructure | `llm/` | 8 | Anthropic/OpenAI clients, streaming batcher |
| Infrastructure | `sqlite/` | 15 | SQLite repositories, migrations |
| Infrastructure | `tools/` | ~35 | Tool implementations (fs, search, documents, system) |
| Root | `lib.rs`, `main.rs` | 2 | Application entry point, state initialization |

## Files Reviewed Deeply

| File | Reason Selected |
|------|----------------|
| `core/agent/entities.rs` | Duplicate domain types; re-exported as public API |
| `core/agent/session.rs` | State machine with transition validation (verified fix) |
| `core/agent/task.rs` | Parallel task types; divergent tool permission logic |
| `core/mod.rs` | Explicit re-export (verified fix) |
| `core/nlp/patterns.rs` | LazyLock regex initialization (verified fix) |
| `infrastructure/agent/runtime/spawn.rs` | Heavy concurrency (Semaphore, Mutex, channels); path traversal security |
| `infrastructure/agent/runtime/mod.rs` | Agent runtime orchestration; Arc-based DI |
| `infrastructure/agent/runtime/execution.rs` | Infrastructure execution entrypoint |
| `infrastructure/agent/approval.rs` | Concurrency (RwLock, broadcast channels) |
| `infrastructure/tools/documents/pdf/render/printpdf_renderer.rs` | 204 unwrap/expect (all in tests) |
| `infrastructure/tools/documents/pdf/final.rs` | Production panic check (was test-only) |
| `infrastructure/tools/system/execute_command.rs` | Command execution; path traversal check |
| `infrastructure/tools/documents/shared/draft_store.rs` | Global Mutex singleton |
| `infrastructure/agent/file_snapshot_registry.rs` | Mutex-guarded file snapshots |
| `infrastructure/llm/anthropic/client.rs` | HTTP client initialization |
| `infrastructure/llm/openai/client.rs` | HTTP client initialization |
| `infrastructure/llm/streaming/batcher.rs` | Chunk batching logic |
| `application/agent/execution/coordinator.rs` | Main execution loop |
| `application/agent/execution/step_gate.rs` | Step limit enforcement |
| `application/tools/pdf/adapter.rs` | Markdown-to-block conversion |
| `commands/documents/mod.rs` | Artifact store Mutex handling |
| `lib.rs` | Application initialization, state management |
| `application/tools/pdf/adapter.rs` | Markdown-to-block conversion; image file read with no path validation |
| `application/tools/pdf/finalize.rs` | Draft-to-PDF build; render pipeline |
| `infrastructure/tools/documents/pdf/final.rs` | PDF finalize tool; artifact write with unsanitized filename |
| `infrastructure/tools/documents/pdf/start.rs` | PDF start tool; filename input normalization |
| `infrastructure/tools/documents/pdf/write.rs` | Atomic write with temp file guard; cancellation |
| `infrastructure/tools/documents/pdf/validate.rs` | Structural PDF validation via lopdf |
| `infrastructure/tools/documents/pdf/render/font_cache.rs` | Font cache; mono slot assignment |
| `infrastructure/tools/documents/pdf/render/layout.rs` | Page-break-aware layout engine |
| `infrastructure/tools/documents/pdf/render/printpdf_renderer.rs` | Core renderer; image decode |
| `infrastructure/tools/documents/pdf/artifacts.rs` | Artifact path resolution and legacy migration |
| `infrastructure/tools/documents/pdf/list.rs` | List/get page tools |
| `infrastructure/tools/documents/pdf/add_page.rs` | Add page tool; content input conversion |
| `infrastructure/tools/documents/shared/draft_store.rs` | In-memory draft store; filename storage |
| `infrastructure/tools/documents/pptx/start.rs` | PPTX start tool; filename not sanitized |
| `infrastructure/tools/documents/pptx/final.rs` | PPTX finalize; path unwrap, hardcoded HOME |
| `infrastructure/tools/documents/pptx/add_slide.rs` | Add slide tool; unwrap after mutation |
| `infrastructure/tools/documents/docx/start.rs` | DOCX start tool; filename not sanitized |
| `infrastructure/tools/documents/docx/final.rs` | DOCX finalize; path unwrap, hardcoded HOME |
| `infrastructure/tools/documents/docx/add_section.rs` | Add section tool; unwrap after mutation |
| `infrastructure/tools/documents/shared/validation.rs` | Shared validation types; clean |
| `infrastructure/llm/streaming.rs` | Stream-to-events conversion; batcher finish bug |
| `infrastructure/llm/streaming/batcher.rs` | Chunk batching logic; clean |
| `infrastructure/llm/anthropic/stream.rs` | SSE parsing, stream state; clean |
| `commands/llm/orchestrator/streaming.rs` | Orchestrated streaming entry point; emit error handling |
| `commands/llm/orchestrator/loop_runner.rs` | Bounded tool loop; serialization swallow, string-match retry |
| `infrastructure/agent/approval.rs` | Approval queue; broadcast sends; clean |
| `infrastructure/tools/fs/write_file.rs` | Path traversal validation; clean |
| `infrastructure/tools/fs/apply_patch.rs` | canonicalize + starts_with path validation; clean |
| `infrastructure/tools/fs/replace_in_file.rs` | canonicalize + starts_with path validation; clean |
| `infrastructure/tools/fs/read_file.rs` | canonicalize + starts_with path validation; clean |
| `infrastructure/tools/fs/fuzzy_find_files.rs` | Bounded traversal; clean |
| `infrastructure/tools/fs/glob_files.rs` | Bounded traversal; clean |
| `infrastructure/tools/fs/stat_path.rs` | canonicalize + starts_with; clean |
| `infrastructure/tools/fs/list_files.rs` | Bounded traversal; clean |
| `infrastructure/tools/search/google_scrape.rs` | Network HTML handling; no production unwraps; clean |
| `infrastructure/tools/search/grep.rs` | Regex + bounded search; clean |
| `infrastructure/tools/search/web_search/` | Full module; proper error handling; clean |
| `infrastructure/tools/agent/plan_subtasks.rs` | Agent plan input; no production unwraps; clean |
| `infrastructure/tools/agent/mark_subtask_status.rs` | Status validation; clean |
| `infrastructure/tools/agent/narrate.rs` | Agent narrative; clean |
| `infrastructure/tools/conversation/search_messages.rs` | Bounded search; clean |
| `infrastructure/agent/runtime/loop_detection.rs` | Hash-based loop detection; clean |
| `infrastructure/agent/runtime/context.rs` | Context compaction; bounded cloning; clean |
| `infrastructure/agent/runtime/client.rs` | Agent LLM client; clean |
| `application/agent/planner.rs` | Agent planner; clean |
| `commands/file.rs` | File content IPC; path traversal, URL encoding issues |
| `commands/nlp.rs` | NLP commands; apply_resolutions has known simplification |
| `commands/config/workspace_policy.rs` | Workspace policy; clean (no prod unwraps) |
| `commands/config/web_search_cache.rs` | Web search cache; clean |
| `commands/config/tool_credentials.rs` | Tool credentials; clean |
| `commands/agent/sessions.rs` | Agent session commands; clean |
| `commands/agent/approvals.rs` | Approval commands; clean |
| `commands/agent/rollback.rs` | Rollback commands; clean |
| `commands/chat/conversations.rs` | Chat conversation commands; clean |
| `application/agent/execution/termination.rs` | Termination paths; `let _ =` on task status updates (acceptable cleanup pattern) |
| `application/agent/execution/bootstrap.rs` | Bootstrap phase; clean error propagation |
| `application/agent/execution/tool_phase.rs` | Tool phase; clean cancellation checks |
| `application/agent/execution/llm_phase.rs` | LLM phase; bounded retries, proper token tracking |
| `application/agent/execution/context_phase.rs` | Context phase; safe unwrap_or usage |
| `application/agent/execution/response_phase.rs` | Response phase; narration loop bounded by MAX_EXECUTION_STEPS |
| `application/agent/service/recovery.rs` | Recovery; proper error handling with warnings |
| `infrastructure/agent/runtime/permissions.rs` | Permissions; approval loop properly bounded with timeout |
| `infrastructure/sqlite/` (all 32 files) | SQL: 400+ queries all parameterized; 1 PRAGMA format! (hardcoded names, safe) |
| `infrastructure/sqlite/migrations/` | SQL queries; all parameterized; clean |

## Triage Results

- **unsafe blocks:** 0
- **FFI markers:** 0
- **Concurrency primitives:** 155 refs across 30 files (all using safe primitives)
- **`unwrap()`/`expect()` in non-test code:** ~10 (HTTP client init, app startup, hardcoded fallbacks)
- **`panic!` in non-test code:** 0 (all `panic!` calls are in `#[cfg(test)]` modules)
- **`todo!`/`unimplemented!`:** 0
- **Production `unwrap()`/`expect()` acceptable:** Yes — all are either initialization-time (HTTP client, DB, state) or hardcoded-value fallbacks

## Findings Summary

| # | Severity | Confidence | Category | Title | Status |
|---|----------|------------|----------|-------|--------|
| F1 | High | High | Maintainability / Correctness | Duplicate type system in agent module | **Fixed** |
| F2 | Medium | High | Correctness Risk | No state machine validation for session transitions | **Fixed** |
| F3 | Medium | High | API Design Smell | Wildcard re-export of chat entities | **Fixed** |
| F4 | Medium | High | Correctness Risk | Regex `unwrap()` in production code path | **Fixed** |
| F5 | Low | High | API Design Smell | Inherent `from_str` shadows `std::str::FromStr` | **Fixed** |
| F6 | Low | High | Correctness Risk | `TaskMode` vs `SubTaskMode` tool list divergence | **Fixed** |
| F7 | Medium | High | Correctness Risk | Inconsistent path traversal validation | **Fixed** |
| F8 | High | High | Correctness Risk / Security | Arbitrary file read via image path in PDF adapter | **Fixed** |
| F9 | High | High | Correctness Risk / Security | Filename path traversal in PDF finalize | **Fixed** |
| F10 | Medium | High | Correctness Risk | Mono font slot uses proportional font | **Fixed** |
| F11 | Medium | Medium | Performance Smell / Correctness Risk | No image size limit before decode | **Fixed** |
| F12 | Medium | High | Correctness Risk | Streaming batcher final flush event dropped | **Fixed** |
| F13 | Medium | High | Correctness Risk | Tool output serialization silently swallowed | **Fixed** |
| F14 | Low | High | Maintainability Smell | Retry classification uses fragile string matching | **Fixed** |
| F15 | High | High | Correctness Risk / Security | Filename path traversal in PPTX/DOCX start tools | **Fixed** |
| F16 | Medium | High | Correctness Risk | `path.to_str().unwrap()` panic on non-UTF-8 paths | **Fixed** |
| F17 | Medium | High | Correctness Risk / API Design | PPTX/DOCX artifact dir bypasses Tauri app data path | **Fixed** |
| F18 | Low | High | Correctness Risk | `unwrap()` after draft store mutation | **Fixed** |
| F19 | Medium | High | Correctness Risk / Security | No path validation in `get_file_content` IPC command | **Fixed** |
| F20 | Medium | High | Correctness Risk | Unencoded URL params in `open_file_diff_window` with `eval()` | **Fixed** |
| F21 | Medium | High | Correctness Risk | No HTTP client timeout on LLM API requests | **Fixed** |
| F22 | Low | High | Performance Smell | Unbounded SSE stream buffer in LLM clients | **Fixed** |

## Resolution Summary
- Fixes implemented: 22 of 22 findings (F1–F22)
- Fixes deferred: 0
- False positives found: 0
- Tests added: 49
- Frontend changes required: No — IPC contracts byte-identical

### Fix Log
| # | Finding | Status | Confidence Verified | Tests Added | Notes |
|---|---------|--------|---------------------|-------------|-------|
| 1 | F2 — State machine validation | Fixed | Yes | 16 | Added `can_transition_to()` + `variant_name()` on `SessionState`, `set_state()` now returns `Result` |
| 2 | F3 — Wildcard re-export | Fixed | Yes | 0 | Replaced `pub use chat::entities::*` with explicit list; compile-time verified |
| 3 | F4 — Regex LazyLock | Fixed | Yes | 0 | Patterns compiled once via `std::sync::LazyLock`; all 31 existing tests pass |
| 4 | F7 — Path traversal inconsistency | Fixed | Yes | 0 (1 updated) | Added `Component::ParentDir` rejection in `execute_command.rs` to match `spawn.rs` |
| 5 | F1 — Duplicate type system | Fixed | Yes | 13 | 4-phase consolidation: serde compat shim, flip AgentService trait, flip repository traits, delete entities.rs. ~45 files touched. |
| 6 | F6 — TaskMode tool list divergence | Fixed | Yes | 0 | Resolved automatically by F1 Phase 3 — `entities::TaskMode` no longer exists; only canonical `SubTaskMode` remains |
| 7 | F5 — `FromStr` shadowing | Fixed | Yes | 5 | Added `impl FromStr` for 4 types; renamed inherent fallback-returning methods to `from_str_or_default`; `OperationType` inherent deleted entirely |
| 8 | F8 — Arbitrary file read via image path | Fixed | Yes | 2 | Added `Component::ParentDir` rejection + 50MB size limit in `image_to_blocks()`; added `InvalidImagePath` and `ImageTooLarge` error variants |
| 9 | F9 — Filename path traversal | Fixed | Yes | 5 | Added `sanitize_filename()` in `pdf_start` — strips directory components, rejects degenerate inputs |
| 10 | F10 — Mono font slot uses proportional font | Fixed | Yes | 2 | Bundled `LiberationMono-Regular.ttf`; updated `bundled_report_cache()` to use it for `FontSlot::Mono` |
| 11 | F11 — No image size limit before decode | Fixed | Yes | 0 | Combined with F8 — `MAX_IMAGE_FILE_BYTES` (50MB) check before `std::fs::read()` |
| 12 | F12 — Streaming batcher final flush dropped | Fixed | Yes | 1 (1 updated) | Changed `_final_event` → `final_event` + `yield` in `streaming.rs:90` |
| 13 | F13 — Tool output serialization swallowed | Fixed | Yes | 0 | Replaced `unwrap_or_default()` with `unwrap_or_else` + `log::warn!` in `loop_runner.rs:215` |
| 14 | F14 — Retry string matching | Fixed | Yes | 0 | Replaced 3-way string matching with `err.is_retryable()` in `loop_runner.rs:264` |
| 15 | F15 — PPTX/DOCX filename path traversal | Fixed | Yes | 5 | Shared `sanitize_filename(raw, ext)` in `shared/validation.rs`; applied to PPTX, DOCX start; consolidated PDF's local copy |
| 16 | F16 — `path.to_str().unwrap()` panic | Fixed | Yes | 0 | Replaced with `to_string_lossy().into_owned()` in PPTX and DOCX finalize |
| 17 | F17 — Artifact dir bypasses Tauri path | Fixed | Yes | 0 | Replaced `$HOME/.coder/artifacts` with `resolve_artifacts_dir(app_handle)` in PPTX/DOCX finalize |
| 18 | F18 — `unwrap()` after draft mutation | Fixed | Yes | 0 | Replaced 3 `.unwrap()` sites with `.ok_or_else(|| ToolError::failed(...))` |
| 19 | F19 — `get_file_content` path traversal | Fixed | Yes | 0 | Added `Component::ParentDir` rejection + `canonicalize + starts_with` in `commands/file.rs` |
| 20 | F20 — Unencoded URL params + `eval()` | Fixed | Yes | 0 | Applied `encode_url()` to all three URL parameters in `open_file_diff_window` |
| 21 | F21 — No HTTP client timeout | Fixed | Yes | 0 | Added `.connect_timeout(30s)` to all 3 `Client::builder()` sites in Anthropic + OpenAI clients |
| 22 | F22 — Unbounded SSE buffer | Fixed | Yes | 0 | Added 50MB buffer cap in SSE parsing loops; yields `LlmError::streaming` on overflow |

### Pre-existing Test Failure
- `test_derive_thread_title_long` in `sqlite/migrations/chat.rs` — assertion `result.len() <= 54` fails. Unrelated to audit findings.

## Expanded Scope Assessment

The expanded review covered `application/`, `commands/`, `infrastructure/`, and root files. Key observations:

**Strong areas:**
- **No unsafe code** across the entire codebase (~170 files)
- **Clean concurrency patterns** — proper use of Semaphore, Mutex, RwLock, channels; no lock ordering issues
- **Proper error handling** — production code consistently uses `Result`, `map_err`, `?`; no panics
- **Well-structured spawn engine** — path traversal protection, concurrency limits, cancellation, telemetry
- **Execution step limits** — `MAX_EXECUTION_STEPS` prevents runaway agent loops
- **Defense in depth** — multiple validation layers for command execution

**Expanded coverage (2026-04-16 full audit):**
- All FS tools (`apply_patch`, `replace_in_file`, `read_file`, `fuzzy_find`, `glob`, `stat`, `list`) use `canonicalize()` + `starts_with(workspace_root)` — correct path traversal prevention
- Web search module (`google_scrape`, `web_search/`) — no production unwraps, proper network error handling
- Agent tools (`plan_subtasks`, `mark_subtask_status`, `narrate`) — no production unwraps
- Agent runtime (`loop_detection`, `context`, `client`) — bounded algorithms, safe arithmetic
- PDF adapter markdown parser — all production `unwrap()`s confirmed safe (guarded by prior checks or on hardcoded constants)

**Full-coverage revalidation (2026-04-16):**
- All 309 Rust source files accounted for — zero unreviewed files remain
- 57 additional files verified in gap-closure pass (mod.rs wiring, tool framework, events, telemetry, utils, read_models, API validation)
- Zero new findings — codebase is in clean state across all four layers
- All 22 prior findings (F1–F22) remain fixed; uncommitted fixes for F19–F22 verified intact

**Areas with minor concerns (all acceptable):**
- HTTP client `expect()` in LLM clients (initialization-time only)
- `CodeAnalyzer::default()` uses `expect()` (tree-sitter is bundled; failure is extremely rare)
- `InlineRun::new("\u{00A0}").unwrap()` in adapter fallback paths (hardcoded non-empty constant — cannot fail)
