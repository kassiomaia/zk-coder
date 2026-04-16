# Code Review Findings — `src-tauri/src/core/`

**Date:** 2026-04-16
**Reviewer:** Rust Audit Pipeline
**Scope:** `src-tauri/src/core/` (agent, chat, config, documents, llm, nlp modules)

---

## F1 — Duplicate Type System in Agent Module

**Severity:** High
**Category:** Maintainability Smell / Correctness Risk
**Location:** `src-tauri/src/core/agent/` — `entities.rs` vs `types.rs`, `session.rs`, `task.rs`

### Description

The agent module maintains two parallel, divergent type hierarchies for the same domain concepts:

| Concept | `entities.rs` | New location |
|---------|---------------|--------------|
| `AgentId` | `entities.rs:10` | `types.rs:10` |
| `SessionId` | `entities.rs:33` | `session.rs:16` |
| `AgentSession` | `entities.rs:140` | `session.rs:44` |
| `SessionState` | `entities.rs:89` | `session.rs:109` |
| `PendingAction` | `entities.rs:228` | `session.rs:170` |
| `Agent` | `entities.rs:62` | `types.rs:30` |
| `AgentTask` | `entities.rs:268` | `task.rs:47` |
| `TaskStatus` | `entities.rs:291` | `task.rs:108` |
| `TaskMode` / `SubTaskMode` | `entities.rs:309` | `task.rs:200` |
| `UsageStats` | `entities.rs:416` | `task.rs:188` |
| `AgentExecutionResult` | `entities.rs:403` | `task.rs:176` |

The `mod.rs` re-exports exclusively from `entities.rs`, making it the "public" API. However, the newer files (`types.rs`, `session.rs`, `task.rs`) are also `pub mod` and thus accessible.

### Why This Is Dangerous

1. **Serde tag divergence:** `entities::SessionState` uses `#[serde(tag = "state")]` while `session::SessionState` uses `#[serde(tag = "type")]`. If the wrong type is used for IPC or database serialization, data will silently deserialize incorrectly or fail.
2. **Behavioral divergence:** `entities::TaskMode::FullAccess.allowed_tools()` includes `"agent_plan_subtasks"`, but `task::SubTaskMode::FullAccess.allowed_tools()` does not. A consumer using one vs the other gets different permission sets.
3. **Structural divergence:** `entities::AgentSession` has no `parent_session_id` or `updated_at` fields; `session::AgentSession` does. `entities::AgentTask` is a simple struct; `task::AgentTask` has steps, subtasks, and lifecycle methods.
4. **Confusion for contributors:** New code can import from either path with no compile-time error.

---

## F2 — No State Machine Validation for Session Transitions

**Severity:** Medium
**Category:** Correctness Risk
**Location:** `src-tauri/src/core/agent/session.rs:99`

### Description

`AgentSession::set_state(&mut self, state: SessionState)` allows any transition:

```rust
pub fn set_state(&mut self, state: SessionState) {
    self.state = state;
    self.updated_at = Utc::now();
}
```

There is no validation that the transition is legal. For example:
- `Finished` -> `Running` (resurrecting a completed session)
- `Cancelled` -> `Launching` (restarting a cancelled session)
- `Error { recoverable: false }` -> `Running` (ignoring unrecoverable errors)

### Impact

Invalid state transitions can corrupt session state, confuse the frontend, and lead to agents operating in states that should be terminal.

### Resolution
**Status:** Fixed
**Commit:** Add `SessionState::can_transition_to()` and validate in `AgentSession::set_state()`
**Tests added:** 16 tests in `session.rs` (valid/invalid transitions, variant names, recoverable vs unrecoverable error distinction)
**Deviations from plan:** None

---

## F3 — Wildcard Re-export of Chat Entities

**Severity:** Medium
**Category:** API Design Smell
**Location:** `src-tauri/src/core/mod.rs:15`

### Description

```rust
pub use chat::entities::*;
```

This re-exports every type from `chat::entities` at the `core` module level. This means:
- Any new type added to `chat::entities.rs` automatically becomes part of the `core` public API.
- Name collisions are possible (e.g., `core::ChatMessage` from chat vs `core::llm::entities::ChatMessage` from LLM — both exist and represent different things).
- It's unclear to consumers where `core::MessageRole` comes from vs `core::llm::entities::MessageRole` (both exist with different variants).

### Impact

Two separate `ChatMessage` types, two separate `MessageRole` types exist in the codebase under different modules. The wildcard re-export makes `chat::entities::ChatMessage` available as `core::ChatMessage`, while `llm::entities::ChatMessage` requires full-path access. This creates an implicit preference that can cause subtle misuse bugs.

### Resolution
**Status:** Fixed
**Commit:** Replace `pub use chat::entities::*` with explicit type list in `core/mod.rs`
**Tests added:** None (compile-time verification — `cargo build` ensures all needed types are re-exported)
**Deviations from plan:** None

---

## F4 — Regex `unwrap()` in Production Code Path

**Severity:** Medium
**Category:** Correctness Risk
**Location:** `src-tauri/src/core/nlp/patterns.rs:25-221`

### Description

`IntentPatterns::build_patterns()` calls `Regex::new(...).unwrap()` approximately 50 times. This function is invoked by `IntentPatterns::new()`, which is a public constructor called at runtime.

While all patterns are hardcoded string literals and currently valid, any future edit that introduces a regex syntax error will cause a panic at runtime rather than a compile-time error.

### Mitigating Factors

- All patterns are well-tested.
- The panic would occur on first construction (likely app startup), not mid-session.
- The patterns are static — they could be compiled once with `std::sync::LazyLock`.

### Resolution
**Status:** Fixed
**Commit:** Replace per-instance regex compilation with `LazyLock` static initialization in `nlp/patterns.rs`
**Tests added:** None (all 31 existing tests verify behavior; `LazyLock` is a transparent optimization)
**Deviations from plan:** Kept `unwrap()` inside the `LazyLock` closure (acceptable for static string literal patterns, as documented in the function's doc comment)

---

## F5 — Inherent `from_str` Methods Shadow `std::str::FromStr`

**Severity:** Low
**Category:** API Design Smell
**Location:** Multiple files

### Description

Several types define inherent `from_str` methods instead of implementing the `FromStr` trait:

- `ApiType::from_str` — `config/mod.rs:46`
- `ModelsSource::from_str` — `config/mod.rs:80`
- `MessageRole::from_str` — `chat/entities.rs:73`
- `OperationType::from_str` — `config/mod.rs:341`

These inherent methods prevent using the standard `.parse::<T>()` pattern and can cause confusion when consumers expect the trait-based API.

---

## F6 — `TaskMode` vs `SubTaskMode` Tool List Divergence

**Severity:** Low
**Category:** Correctness Risk
**Location:** `entities.rs:361-376` vs `task.rs:252-265`

### Description

`entities::TaskMode::FullAccess.allowed_tools()` includes `"agent_plan_subtasks"` (line 373), but `task::SubTaskMode::FullAccess.allowed_tools()` explicitly excludes it. The test in `task.rs:587` asserts this exclusion.

This means the tool permission set depends on which type the caller uses, leading to inconsistent behavior. A caller using `entities::TaskMode` gets planning capability; one using `task::SubTaskMode` does not.
