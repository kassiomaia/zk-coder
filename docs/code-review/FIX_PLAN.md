# Fix Plan — `src-tauri/src/`

**Date:** 2026-04-16
**Based on:** `FINDINGS.md` from Phase 1

---

## Selection Table

| # | Finding | Confidence | Selected | Rationale |
|---|---------|------------|----------|-----------|
| F1 | Duplicate type system in agent module | High | Deferred | Requires major cross-layer refactoring (~70 files affected); needs dedicated branch and migration strategy |
| F2 | No state machine validation | High | **EXECUTED** | Fixed in prior audit run |
| F3 | Wildcard re-export of chat entities | High | **EXECUTED** | Fixed in prior audit run |
| F4 | Regex unwrap() in production code | High | **EXECUTED** | Fixed in prior audit run |
| F5 | `from_str` shadows std trait | High | Deferred | Low severity; cosmetic improvement that touches stable APIs with many callers |
| F6 | TaskMode vs SubTaskMode divergence | High | Deferred | Part of F1 — duplicate type system; fixing it in isolation would just increase confusion |
| F7 | Inconsistent path traversal validation | High | **EXECUTED** | Small, focused change in one file; aligns validation with spawn engine's security boundary |

## Deferred Table

| # | Finding | Reason |
|---|---------|--------|
| F1 | Duplicate type system | Blast radius exceeds ~70 files across core/application/infrastructure layers. Requires: (1) choosing canonical types, (2) migration plan for each layer, (3) serde compatibility strategy. Recommend a dedicated `chore/consolidate-agent-types` branch. |
| F5 | `from_str` shadows | Low impact. Would require updating callers and is cosmetic. |
| F6 | Tool list divergence | Symptom of F1. Fixing the divergence without consolidating the types just adds another thing to keep in sync. |

---

## Fix 1 — Align Path Traversal Validation (F7)

### Root Cause

`ExecuteCommandTool::execute()` in `execute_command.rs:112-118` validates the `working_dir` argument with only `Path::starts_with()`, which doesn't detect `..` path components. The spawn engine (`spawn.rs:781-789`) performs the correct check — rejecting `Component::ParentDir` before the `starts_with` check.

### Proposed Change

Add the same `Component::ParentDir` rejection check to `execute_command.rs` before the `starts_with` check:

```rust
// Before (current)
let cwd = if let Some(ref dir) = args.working_dir {
    let resolved = workspace.join(dir);
    if !resolved.starts_with(&workspace) {
        return Err(ToolError::PathTraversal(...));
    }
    resolved
} else {
    workspace.clone()
};

// After (proposed)
let cwd = if let Some(ref dir) = args.working_dir {
    let dir_path = std::path::Path::new(dir);
    if dir_path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(ToolError::PathTraversal(format!(
            "Working directory '{dir}' contains parent-directory components"
        )));
    }
    let resolved = workspace.join(dir);
    if !resolved.starts_with(&workspace) {
        return Err(ToolError::PathTraversal(format!(
            "Working directory '{dir}' is outside workspace"
        )));
    }
    resolved
} else {
    workspace.clone()
};
```

### Affected Code Paths

Only `execute_command.rs:112-118`. No callers change. The spawn engine's validation remains as a secondary safety net.

### Edge Cases

- `"./subdir"` — should pass (no `ParentDir` component)
- `"subdir/../other"` — should fail (contains `ParentDir`)
- `"subdir"` — should pass
- `""` — falls through to `workspace.clone()` (the `dir` check is `if let Some(ref dir)`)

### Test Plan

1. Add a test that `"../escape"` is rejected by the tool-level check (not just by spawn engine).
2. Add a test that `"./valid"` passes the check.
3. Existing integration tests pass.

### Risk Assessment

**Very low risk.** The change adds a stricter check in an early-fail path. The spawn engine already catches all `..` paths, so this can only prevent issues, never cause regressions.
