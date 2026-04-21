# SOLID Architecture Refactor — zk-coder

**Status**: ACTIVE
**Owner**: —
**Date drafted**: 2026-04-20
**Last reviewed**: 2026-04-21 (Phase 5)

## Why this exists

Exploration of the codebase surfaced four files that exceed the 800-LOC hard limit defined in `RULES.md`, plus several god-objects and tight-coupling patterns that violate SOLID principles. The goal is to improve maintainability and testability by splitting responsibilities and introducing abstractions **only where SOLID clearly demands it** — not speculatively.

The existing 4-layer backend (`core/` → `application/` → `commands/` + `infrastructure/`) and frontend layering (`api/` → `stores/` → `components/`) are correct and must be preserved. The refactor works **within** these boundaries, not against them.

## Scope & approach

- **Scope**: backend + frontend.
- **Merge style**: one PR per phase, each independently shippable.
- **Abstractions**: pragmatic — add traits only where OCP/ISP is clearly violated; keep concrete types where they work.

## Files violating hard limits (verified 2026-04-20)

| File | LOC | Limit |
|---|---|---|
| `src-ui/src/lib/stores/chat.ts` | 1539 | 800 |
| `src-tauri/src/application/agent/execution/termination.rs` | 1295 | 800 |
| `src-tauri/src/application/agent/execution/context_phase.rs` | 1013 | 800 |
| `src-tauri/src/infrastructure/sqlite/repositories/agent/mod.rs` | 1072 | 800 |
| `src-tauri/src/application/agent/read_models.rs` | 610 | 500 soft |

## Patterns to reuse

- **Static renderer registry** — `src-ui/src/lib/components/chat/messages/tool-renderers/registry.ts`. Apply the same shape (static map + fallback) to the backend tool dispatcher.
- **`createInvoker` / `invokeCommand`** — `src-ui/src/lib/api/core.ts`. Every new or migrated IPC call in components must go through this.
- **Derived-store factory** — `src-ui/src/lib/stores/chat/derived-stores.ts`. Mirror this shape when splitting `chat.ts`.
- **LLM handler extraction** — `src-ui/src/lib/stores/chat/llm-handler.ts`. Precedent for extracting stream logic out of the store.

## File-size budget for new modules

- Stores/services: ≤400 LOC target, 500 soft cap.
- Repositories: ≤500 LOC per domain repo.
- Execution phase handlers: ≤400 LOC each.
- Any new file approaching 500 LOC triggers a split review.

## Phases

### Phase 1 — Shared runtime constants (frontend) — **DONE 2026-04-20**

- Created `src-ui/src/lib/config/runtime.ts` exporting `STREAM_INACTIVITY_TIMEOUT_MS`, `STREAM_MAX_BUFFERED_EVENTS`, `DOC_GEN_MAX_AGE_MS`, `NLP_DEBOUNCE_MS`.
- Replaced inline magic numbers in `chat/llm-handler.ts`, `chat.ts`, `documentGenerationStore.ts`.
- SOLID: SRP (config ≠ behavior).

### Phase 2 — Split `termination.rs` (1295 → ≤600 LOC each) — **DONE 2026-04-20**

Shipped layout (simpler than the original four-file plan — "persistence" was a
few interleaved lines per public function, not its own module):

- `src-tauri/src/application/agent/execution/termination.rs` (356 LOC) — entry file; retains the five public orchestration functions (`update_session_state`, `cancel_execution`, `fail_execution`, `mark_step_budget_exhausted`, `complete_execution`) and re-exports `emit_streaming_message`, `auto_complete_in_progress_subtasks`, `advance_next_pending_subtask` for sibling phases that import via `super::termination::…`.
- `src-tauri/src/application/agent/execution/termination/streaming.rs` (451 LOC) — `STREAMING_CHUNK_BYTES`, chunking, `StreamingContent` emission + 10 inline tests.
- `src-tauri/src/application/agent/execution/termination/subtasks.rs` (536 LOC) — `auto_complete_in_progress_subtasks`, `has_remaining_in_progress_subtasks`, `advance_next_pending_subtask`, `emit_subtask_result_with_trace` + the four `subtask_autocompletion_tests`.

Verified: 14 termination tests pass; `cargo test --lib application::agent::execution` → 136 passed, 0 failed. Behaviour unchanged — re-exports preserve the external `super::termination::…` import surface used by `step_gate`, `park`, `bootstrap`, `tool_phase`, `response_phase`.

SOLID: SRP — session-level termination orchestration, streaming-payload construction, and subtask-graph state transitions no longer share a file.

### Phase 3 — Split `context_phase.rs` (1013 → modules) — **DONE 2026-04-20**

Shipped layout (file-module, not directory-module — the entry file stays at
`context_phase.rs` and declares `mod projection; mod resolution;`):

- `src-tauri/src/application/agent/execution/context_phase.rs` (378 LOC) — retains the public `resolve_context` async function and the phase orchestration; imports helpers from the two submodules via `use projection::…` / `use resolution::…`.
- `src-tauri/src/application/agent/execution/context_phase/resolution.rs` (375 LOC) — memory-refresh triggers (`maybe_fire_subtask_completed`, `maybe_fire_retrieval_burst`, `maybe_fire_compaction_boundary`), retrieval pass (`run_retrieval_pass`, `build_retrieval_query`), `identify_messages_to_drop`, compaction recording (`record_compaction`, `should_emit_compaction_event`) + 5 compaction-emit tests.
- `src-tauri/src/application/agent/execution/context_phase/projection.rs` (315 LOC) — pure projection builders that turn durable state into the context-policy value objects (`build_execution_brief`, `blocker_class_label`, `lookup_memory_reference`) + 6 execution-brief tests.

Verified: 11 context_phase tests pass; `cargo test --lib application::agent::execution` → 136 passed, 0 failed; behaviour unchanged (only `resolve_context` is imported by siblings, and that surface is preserved).

SOLID: SRP — phase orchestration, side-effecting resolution steps, and pure projection builders no longer share a file.

### Phase 4 — Context Policy Strategy trait — **DONE 2026-04-20**

Shipped layout:

- New `src-tauri/src/core/agent/context.rs` owns the `ContextCompactionStrategy` trait plus the `ContextPackOutcome`, `CompactionDetails`, and `ContextState<'_>` value objects. Re-exported from `core::agent::*`.
- New `src-tauri/src/infrastructure/agent/runtime/context/strategies/token_threshold.rs` holds `TokenThresholdStrategy` as the sole concrete impl — byte-for-byte identical logic to the former inline `maybe_compact_context` (adaptive threshold gate, first-message-preserving keep-recent rewrite, auto-summary of the dropped range). Three focused unit tests lock in: threshold gating, small-context no-op, and tail-preservation.
- `infrastructure/agent/runtime/context/mod.rs` now re-exports the core DTOs so every existing import (`use infrastructure::agent::runtime::context::{CompactionDetails, ContextPackOutcome}`) keeps compiling, and exposes a thin `maybe_compact_context` dispatcher plus a `maybe_compact_with_strategy` variant taking `&dyn ContextCompactionStrategy`. `policy.rs` lost the duplicated DTO defs and the V1 function body; `classify_outcome` / `build_compaction_summary` widened to `pub(in …::context)` so sibling strategy modules can reach them without leaking to the rest of the crate.
- No caller change required — `apply_context_policy_v2`, `CompactionDetails`, `ContextPackOutcome`, and `maybe_compact_context` all still import from `infrastructure::agent::runtime::context`.

Verified: `cargo test --lib` → 1935 passed, 0 failed; `cargo test --lib application::agent::execution` → 136 passed; `cargo test --lib infrastructure::agent::runtime::context` → 66 passed (includes the 3 new strategy tests).

SOLID: OCP — adding a new compaction strategy is a new file + one line in `strategies/mod.rs`, zero changes to the dispatcher. DIP — the runtime depends on the core `ContextCompactionStrategy` abstraction, not on a concrete infra struct.

### Phase 5 — Tool dispatcher registry — **DONE 2026-04-21**

Shipped layout:

- New `src-tauri/src/infrastructure/agent/runtime/tool_handlers/dispatcher.rs` owns the `ToolHandler` trait (`name`, `handle`) + the `ToolHandlerContext<'a>` bundle that carries every dependency the four meta-tools need (`session`, `task_id`, `current_subtask_id`, `app_handle`, `session_repo`, `task_repo`, `telemetry_repo`, `scope`). **Placement deviation from original plan**: the trait lives in infrastructure rather than `core/agent/tools/` because `ToolHandlerContext` references framework-level types (`tauri::AppHandle`, `RuntimeSessionScope`) that the domain layer must not depend on. The OCP benefit (new tool = new file + one registry line) is realised regardless of which layer holds the trait.
- New `src-tauri/src/infrastructure/agent/runtime/tool_handlers/registry.rs` owns a `OnceLock<HashMap<&'static str, &'static dyn ToolHandler>>` initialised once from an inline handlers array plus a `pub(super) fn lookup(name: &str) -> Option<&'static dyn ToolHandler>`. Three unit tests lock in the catalog enumeration, unknown-tool miss, and known-tool hit.
- Each of the four agent meta-tool modules (`plan_subtasks.rs`, `advance_subtask.rs`, `mark_subtask_status.rs`, `narrate.rs`) now declares a zero-sized unit struct + a `'static` instance (`PLAN_SUBTASKS`, `ADVANCE_SUBTASK`, `MARK_SUBTASK_STATUS`, `NARRATE`) + an `impl ToolHandler` that delegates to the existing free function — so the free-function entry points stay available for direct callers and there is no behaviour change, no allocation on the hot path.
- `tool_handlers/mod.rs` shrank to 64 LOC: the `handle_agent_tool_call` entry point is now `registry::lookup(name)?.handle(call, &mut ctx)` — no hardcoded match arm to edit when a new meta-tool lands.

Verified: `cargo build --lib` green; `cargo test --lib` → **1938 passed, 0 failed** (3 new registry tests on top of the Phase 4 baseline).

SOLID: OCP — adding a new agent meta-tool is one new handler module plus one entry in the `registry.rs` handlers array, zero edits to `handle_agent_tool_call`.

### Phase 6 — Split `SqliteSessionRepository` by domain — **PENDING**

Split the 1072-LOC, 8-trait god-repo into domain-scoped structs sharing one `Arc<Mutex<Connection>>`:
- `SqliteSessionRepository` — `SessionRepository`
- `SqliteTaskRepository` — `TaskRepository`
- `SqliteExecutionRepository` — `ExecutionRepository` + `RuntimeTraceRepository`
- `SqliteClarificationRepository` — `ClarificationRepository` + `ExecutionCheckpointRepository`
- `SqliteMemoryRepository` — `MemoryRepository`
- `SqliteAgentTelemetryRepository` — `AgentTelemetryRepository`

Also fixes the DIP cast at `src-tauri/src/lib.rs:113-122` — commands receive pre-typed `Arc<dyn SessionRepository>` etc., never a concrete SQLite struct.

SOLID: ISP, DIP.

### Phase 7 — Split `chat.ts` store (1539 → domain stores) — **PENDING**

Under `src-ui/src/lib/stores/chat/`:
- `conversations.ts` — conversation CRUD + selection
- `threads.ts` — thread lifecycle
- `messages.ts` — message list + mutation
- `nlp.ts` — classifier / ambiguity detection / resolution
- `orchestrator.ts` — thin facade preserving the public API during migration

Keep `llm-handler.ts` and `derived-stores.ts` untouched. Also clears the pre-existing unused imports (`findById`, `isValidDate`).

SOLID: SRP.

### Phase 8 — Remove direct `invoke()` from components — **PENDING**

Route through `createInvoker` / `invokeCommand`:
- `src-ui/src/lib/components/chat/ChatMessagesList.svelte:327`
- `src-ui/src/lib/components/agent/AgentPanel.svelte:120`
- `src-ui/src/lib/components/ManualModelList.svelte`
- `src-ui/src/lib/components/ArtifactCard.svelte`

SOLID: DIP.

### Phase 9 — Agent ↔ chat typed event bus — **PENDING**

- Add `src-ui/src/lib/stores/agent/events.ts` — minimal typed pub/sub (≤150 LOC, no deps).
- Replace the six `setXxxCallback` methods on `src-ui/src/lib/stores/agent/store.ts:63-94`.
- Chat orchestrator (Phase 7) subscribes.

SOLID: DIP, OCP.

## Out of scope (deferred)

- Shared type generation frontend ↔ backend (ts-rs / specta) — its own initiative.
- `read_models.rs` split (610 LOC) — soft-limit only; opportunistic.
- SQLite connection pooling — risky and orthogonal.
- Tokio runtime ownership inside `AgentServiceImpl` — requires reshaping sync IPC surface.
- Implicit message-kind dispatch in `ChatMessagesList.svelte:514-596` — existing renderer registry covers the important surface.
- `coordinator.rs` (422 LOC) split — under hard limit; revisit only if phases 2–3 expose more seams.
- Typed error channel across the Tauri boundary (`RepositoryResult → AgentDomainError → String`) — larger design call.

## Verification (every phase)

- `cargo clippy --all-targets -- -D warnings`
- `cargo test` (relevant module path)
- `npm run check` (from `src-ui/`)
- `npm run test`
- `npm run lint`
- Manual smoke: start a session, stream a reply, trigger a tool, reach termination — confirm behavior unchanged.

## Docs process (per phase)

Each PR updates:
- `docs/architecture.md` — any new module boundary, trait, or directory layout.
- `CHANGELOG.md` — one line per phase under `## [Unreleased]` → `### Changed`, landed immediately (not batched).
- This plan file — append/update the phase status line as work ships.

When the last phase lands, mark this plan `COMPLETE` and move it to `docs/archive/planning/solid-refactor/plan.md` per the documentation governance in `AGENTS.md`.
