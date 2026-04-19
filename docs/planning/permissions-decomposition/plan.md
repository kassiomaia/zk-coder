# Permissions.rs Decomposition — Implementation Plan

**Status**: DRAFT
**Last reviewed**: 2026-04-19
**Purpose**: Split `src-tauri/src/infrastructure/agent/runtime/permissions.rs` (4507 lines, ~5× over the 800-LoC hard limit) into focused modules, move domain rules to `core/`, and replace four process-global `OnceLock<Mutex<_>>` trackers with a session-scoped context. Fixes a confirmed multi-session correctness bug as a side effect.

---

## Context

`permissions.rs` mixes five responsibilities:

1. Permission check + dispatch to `ToolRegistry`
2. Agent meta-tool handlers: `handle_plan_subtasks`, `merge_model_plan_into_existing`, `handle_advance_subtask`, `handle_mark_subtask_status`, `handle_narrate`
3. Plan/contract domain rules: `validate_plan_subtasks`, `detect_overlapping_contracts`, `detect_noop_final_subtask`, `detect_code_change_intent`, `patterns_overlap`, `literal_prefix`, `is_path_prefix_of`, `literal_matches_glob`
4. Filesystem artifact probing: `pattern_is_satisfied_on_disk`, `walk_workspace_for_match`, `pattern_matches_touched_overlay`
5. Session-scoped tracker state held in process-global statics: `PLAN_RETRY_TRACKER`, `SUBTASK_RATIONALES`, `SUBTASK_FAILURE_COUNTS`, `DECOMPOSED_SUBTASKS`

### Confirmed multi-session bug

Three of the four statics key on bare `subtask_id` (no session prefix). `DECOMPOSED_SUBTASKS::mark_subtask_decomposed_once` returns `false` forever once a UUID has been decomposed, so subsequent sessions cannot auto-decompose a subtask even when decomposition is warranted. `SUBTASK_RATIONALES` and `SUBTASK_FAILURE_COUNTS` have the same unbounded lifetime and cross-session key collision risk.

### Validated constraints

- Only two callers of `process_tool_calls_with_permissions`: `application/agent/execution/tool_phase.rs` and the re-export in `infrastructure/agent/runtime/mod.rs`.
- Existing tests in `permissions/permissions_tests.rs` (39 tests) only exercise types — none touch handlers or validators.
- No duplicates of the pattern-matching helpers elsewhere in the tree.

---

## Target end-state

```
core/agent/planning/
  mod.rs
  plan_validation.rs                 ← pure domain rules, fully unit-testable

infrastructure/agent/runtime/
  permissions.rs                     ← ≤500 LoC: dispatch loop, malformed-args path, execute_tool, execute_with_approval
  session_scope.rs                   ← RuntimeSessionScope (replaces the four statics)
  artifact_probe.rs                  ← filesystem probing
  tool_handlers/
    mod.rs                           ← handle_agent_tool_call dispatcher
    plan_subtasks.rs                 ← handle_plan_subtasks + merge_model_plan_into_existing
    advance_subtask.rs
    mark_subtask_status.rs           ← handle_mark_subtask_status + auto_decompose_failed_subtask
    narrate.rs
    contract_verification.rs
```

---

## Phase ordering

Each phase is one commit. `cargo build && cargo test -p coder_lib && cargo clippy -p coder_lib -- -D warnings` must pass at every boundary.

### Phase 0 — Characterization tests

Per `docs/quality/guidelines/rust-backend.md` §"Refactor process" step 2. Add before any code motion:

- `validate_plan_subtasks` accept/reject cases — empty plan, overlapping contracts, no-op final subtask, `detect_code_change_intent` true/false
- `patterns_overlap`, `literal_prefix`, `is_path_prefix_of`, `literal_matches_glob` — 3–5 inputs each
- `pattern_matches_touched_overlay` — pure string-list check
- `RuntimeSessionScope` equivalents of the four statics written against the **target** API (will fail to compile until Phase 3 — natural sequencing)

No characterization tests for handlers themselves (their `AppHandle`/`ApprovalQueue`/`Repository` dependencies are too broad to stand up cheaply). Rely on existing integration tests (`integration_tests.rs`, `runtime_enforcement_tests.rs`) to catch regressions.

### Phase 1 — `core/agent/planning/` (pure validators)

- Create `core/agent/planning/mod.rs` and `core/agent/planning/plan_validation.rs`
- Move: `validate_plan_subtasks`, `detect_overlapping_contracts`, `detect_noop_final_subtask`, `detect_code_change_intent`, `patterns_overlap`, `literal_prefix`, `is_path_prefix_of`, `literal_matches_glob`
- Register `pub mod planning;` in `core/agent/mod.rs`; visibility `pub(crate)`
- Update `permissions.rs` imports to `use crate::core::agent::planning::…`

### Phase 2 — `artifact_probe.rs`

- Create `infrastructure/agent/runtime/artifact_probe.rs`
- Move: `pattern_is_satisfied_on_disk`, `walk_workspace_for_match`, `pattern_matches_touched_overlay`
- Register `pub(crate) mod artifact_probe;` in `runtime/mod.rs`

### Phase 3 — `session_scope.rs` + lift scope into `ExecutionContext`

Critical phase: collapses the four statics and adds the one justified signature change.

- Create `RuntimeSessionScope` struct with per-session-local maps/sets (no `Mutex`, no `OnceLock`):

  ```rust
  pub(crate) struct RuntimeSessionScope {
      session_id: SessionId,
      plan_retries: HashSet<String>,
      subtask_rationales: HashMap<String, String>,
      subtask_failure_counts: HashMap<String, u32>,
      decomposed_subtasks: HashSet<String>,
  }
  ```

- Methods replace free helpers:
  - `remember_subtask_rationale` → `scope.record_rationale(&subtask_id, text)`
  - `has_subtask_rationale`       → `scope.has_rationale(&subtask_id)`
  - `increment_subtask_failure_count` → `scope.increment_failure(&subtask_id) -> u32`
  - `mark_subtask_decomposed_once` → `scope.try_mark_decomposed(&subtask_id) -> bool`
  - `PLAN_RETRY_TRACKER` membership → `scope.mark_plan_retry(&fingerprint) -> bool`

- **Scope lifetime = session, not call.** Instantiate in `ExecutionContext::new` (application layer), thread `&mut` through `run_tools` → `process_tool_calls_with_permissions` → handlers. Justification: `SUBTASK_RATIONALES` is written by `handle_mark_subtask_status` and read later by `handle_advance_subtask`; those run in different tool batches within the same session. Per-call scope would be strictly narrower than current observable behavior and break the read-across-batches invariant.

- `process_tool_calls_with_permissions` gains one parameter: `scope: &mut RuntimeSessionScope`. Both callers (`tool_phase.rs` and the `runtime/mod.rs` re-export) get a mechanical update in the same commit so no tree state is broken.

- Remove the four `OnceLock<Mutex<_>>` statics.

### Phase 4 — `tool_handlers/contract_verification.rs`

- Create `tool_handlers/mod.rs` stub and `tool_handlers/contract_verification.rs`
- Move: `verify_contract_artifacts`, `ContractVerificationFailure`, its impl, `parse_status` (if only used here), `has_subtask_evidence`
- Register `pub(crate) mod tool_handlers;` in `runtime/mod.rs`

### Phase 5 — Handler extraction

- `plan_subtasks.rs`           ← `handle_plan_subtasks` + `merge_model_plan_into_existing`
- `advance_subtask.rs`         ← `handle_advance_subtask`
- `mark_subtask_status.rs`     ← `handle_mark_subtask_status` + `auto_decompose_failed_subtask`
- `narrate.rs`                 ← `handle_narrate`
- Each handler gains `scope: &mut RuntimeSessionScope`
- Handlers are `pub(super)`; no public re-exports change

### Phase 6 — Dispatcher move + slim `permissions.rs`

- Move `handle_agent_tool_call` into `tool_handlers/mod.rs`
- Relocate `core_task_to_event_subtask`, `emit_file_events`, `parse_and_validate_framing` to the best-fitting handler module
- Final `permissions.rs`: module doc, imports, `YoloModeProvider`, `PendingActionExt`, `emit_persisted_streaming_message`, `process_tool_calls_with_permissions`, `execute_tool`, `execute_with_approval`, malformed-args fast path, constants — target ≤500 LoC
- `permissions_tests.rs` keeps working (only references `ActionType`/`ActionPreview`/`PendingAction`/`PendingActionExt`)

### Phase 7 — Tidy

- Drop `#[allow(unused_imports)]`/`#[allow(clippy::too_many_arguments)]` annotations no longer needed
- `cargo clippy -p coder_lib -- -D warnings`

---

## Bug-fix side effects

### Multi-session state leak

Fixed by Phase 3 construction: per-session `&mut`-borrowed scope cannot be polluted across sessions.

### `DECOMPOSED_SUBTASKS` invariant drift

Even within Phase 3's scope, the "decompose once per failed subtask" invariant is better expressed as a lookup against persisted data. Replace `scope.try_mark_decomposed(&subtask_id)` with a guard that additionally checks `task_repo.get_subtasks(task_id)` for children whose description matches the `"Auto-decomposed from failed subtask '...'"` pattern. One-function guard; lands in Phase 5 alongside `auto_decompose_failed_subtask`.

---

## Behavior-preservation flags

- **`PLAN_RETRY_TRACKER`** — today's semantics persist process-wide across sessions; after refactor they persist per session only. This is strictly a tightening (the cross-session leak was unintentional). Retries still work within any single session because scope lives in `ExecutionContext`.
- **`SUBTASK_RATIONALES`** — read/write across handler invocations within a session. Preserved by per-session `ExecutionContext` scope.
- **`SUBTASK_FAILURE_COUNTS`** — same as rationales. Preserved.
- **`DECOMPOSED_SUBTASKS`** — replaced by the task-repo-based guard above.

No `println!` introduced; all logging continues through `log::{info,warn}`. No `unwrap`/`expect`/`panic` — the poisoned-mutex handling disappears entirely with the `Mutex` removal. Typed-error conversion stays out of scope; handlers keep their current `Result<…, String>` shapes where they exist.

---

## Test strategy

### Before code motion (Phase 0)

See list above. Characterization coverage focuses on pure logic that can be tested without `AppHandle`/SQLite.

### Enabled after extraction

- `core/agent/planning/plan_validation.rs` becomes fully unit-testable in `core` with zero infrastructure deps. Target: table-driven suite covering every permutation of overlapping contracts, every no-op final signal, every `detect_code_change_intent` branch.
- `RuntimeSessionScope` has a deterministic local test surface (no `OnceLock` state pollution between tests).
- `artifact_probe::pattern_matches_touched_overlay` gains direct unit-test coverage (already pure — was just buried).

---

## Rollback plan

- Each phase = one commit on a topic branch.
- If Phase N breaks CI: `git reset --hard HEAD~1` restores the state at end of Phase N-1, which is fully valid.
- Late-phase rollbacks (5–7) leave a partially-decomposed but correct tree; `core/agent/planning/` and `session_scope.rs` remain landed on their own.
- Public surface (`process_tool_calls_with_permissions`, `YoloModeProvider`, `AgentRuntimeError`) is unchanged except for the single Phase-3 signature change, which lands atomically with its call-site update.
- Phase 0 characterization tests stay valid across all rollbacks.

---

## Out of scope

- Typed error migration for handlers (separate change)
- `OpenAiClient::chat_stream_internal` decomposition (separate audit finding)
- `execution/` phase-file breakup (`termination.rs`, `context_phase.rs`, `memory_refresh.rs`) — separate audit finding
- Second `tokio::runtime::Runtime` in `AgentServiceImpl` — separate audit finding

---

## Critical files

- `src-tauri/src/infrastructure/agent/runtime/permissions.rs`
- `src-tauri/src/application/agent/execution/tool_phase.rs`
- `src-tauri/src/application/agent/execution/state.rs`
- `src-tauri/src/infrastructure/agent/runtime/mod.rs`
- `src-tauri/src/core/agent/mod.rs`
