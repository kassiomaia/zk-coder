# Fix Plan — `src-tauri/src/core/`

**Date:** 2026-04-16
**Based on:** `FINDINGS.md` from Phase 1

---

## Selection Table

| # | Finding | Selected | Rationale |
|---|---------|----------|-----------|
| F1 | Duplicate type system in agent module | Deferred | Requires major cross-layer refactoring (~70 files affected); needs dedicated branch and migration strategy |
| F2 | No state machine validation | **Yes** | Small, focused change in one file; prevents invalid state transitions that could corrupt sessions | **EXECUTED** |
| F3 | Wildcard re-export of chat entities | **Yes** | One-line change with bounded blast radius; prevents name collisions | **EXECUTED** |
| F4 | Regex unwrap() in production code | **Yes** | Replaces ~50 panicking unwraps with compile-once lazy initialization; prevents future regex typo panics | **EXECUTED** |
| F5 | `from_str` shadows std trait | Deferred | Low severity; cosmetic improvement that touches stable APIs with many callers |
| F6 | TaskMode vs SubTaskMode divergence | Deferred | Part of F1 — duplicate type system; fixing it in isolation would just increase confusion |

## Deferred Table

| # | Finding | Reason |
|---|---------|--------|
| F1 | Duplicate type system | Blast radius exceeds ~70 files across core/application/infrastructure layers. Requires: (1) choosing canonical types, (2) migration plan for each layer, (3) serde compatibility strategy. Recommend a dedicated `chore/consolidate-agent-types` branch. |
| F5 | `from_str` shadows | Low impact. Would require updating callers and is cosmetic. |
| F6 | Tool list divergence | Symptom of F1. Fixing the divergence without consolidating the types just adds another thing to keep in sync. |

---

## Fix 1 — State Machine Validation (F2)

### Root Cause

`AgentSession::set_state()` in `session.rs:99` is a simple setter with no transition guard. Any caller can set any state regardless of the current state.

### Proposed Change

Add a `can_transition_to(&self, target: &SessionState) -> bool` method that encodes the valid transitions, and make `set_state` return `Result<(), &'static str>` when the transition is invalid.

**Valid transitions:**
- `Launching` -> `Running`, `Error`, `Cancelled`
- `Running` -> `WaitForRequest`, `UserInputRequired`, `Finished`, `Error`, `Cancelled`, `Paused`
- `WaitForRequest` -> `Running`, `Finished`, `Error`, `Cancelled`, `Paused`
- `UserInputRequired` -> `Running`, `Error`, `Cancelled`
- `Paused` -> `Running`, `Error`, `Cancelled`
- `Error { recoverable: true }` -> `Running`, `Cancelled`
- `Error { recoverable: false }` -> (terminal, no transitions)
- `Finished` -> (terminal, no transitions)
- `Cancelled` -> (terminal, no transitions)

**Affected code paths:** Callers of `set_state` in the infrastructure and application layers. These must be audited to confirm they already follow valid transitions (they should, since the FSM existed conceptually).

### Edge Cases

- A recoverable error should allow re-entering `Running`; an unrecoverable one should not.
- `Paused` sessions should be resumable to `Running`.

### Test Plan

1. Test that all valid transitions succeed.
2. Test that invalid transitions (e.g., `Finished` -> `Running`, `Cancelled` -> `Launching`) return an error.
3. Test the `recoverable` flag distinction for error states.

### Risk Assessment

**Low risk.** The change makes invalid transitions fail rather than silently succeed. Existing callers that already follow the valid FSM will not be affected. Any callers making invalid transitions will surface as test failures, which is the desired outcome.

---

## Fix 2 — Replace Wildcard Re-export (F3)

### Root Cause

`core/mod.rs:15` uses `pub use chat::entities::*` which dumps all chat types into the `core` namespace, creating name collision potential with `llm::entities`.

### Proposed Change

Replace the wildcard with explicit re-exports of the types that are actually used externally:

```rust
// Before
pub use chat::entities::*;

// After
pub use chat::entities::{
    ChatDomainError, ChatMessage, Conversation, ConversationSettings,
    MessageFeedback, MessageRole, MessageStatus, Thread, ThreadMetadata,
    ThreadPriority,
};
```

### Affected Code Paths

Any code that imports from `crate::core::` and relies on a chat entity being available without qualification. A `cargo build` will reveal any missing re-exports.

### Test Plan

1. `cargo build` succeeds.
2. `cargo test` passes.

### Risk Assessment

**Very low risk.** If a type is missing from the explicit list, the compiler will tell us immediately.

---

## Fix 3 — Regex LazyLock Initialization (F4)

### Root Cause

`IntentPatterns::build_patterns()` in `nlp/patterns.rs:25` constructs ~50 `Regex` objects via `Regex::new(...).unwrap()` every time `IntentPatterns::new()` is called. This is both a panic risk and a performance waste (regex compilation is expensive, and the patterns are static).

### Proposed Change

Use `std::sync::LazyLock` to compile the patterns exactly once:

1. Define a `static PATTERNS: LazyLock<Vec<(Intent, Vec<Regex>)>>` that calls `build_patterns()`.
2. Change `IntentPatterns::new()` to reference the static patterns via `&'static`.
3. Change `IntentPatterns.patterns` from `Vec<(Intent, Vec<Regex>)>` to `&'static [(Intent, Vec<Regex>)]` (or keep the struct as a thin wrapper).
4. Keep `unwrap()` inside the `LazyLock` closure — it runs exactly once and panics on first use if any regex is invalid, which is acceptable for static patterns.

### Edge Cases

- `LazyLock` is `Send + Sync`, so this is safe for multi-threaded Tauri contexts.
- Tests that construct `IntentPatterns` repeatedly will now share the same compiled patterns, which is correct behavior.

### Test Plan

1. All existing tests in `nlp/patterns.rs` pass.
2. Add a test that constructs `IntentPatterns` twice to verify the shared static works.
3. `cargo clippy` clean.

### Risk Assessment

**Low risk.** The behavior is identical — patterns are compiled and matched the same way. The only change is when compilation happens (once at first use vs. every construction).
