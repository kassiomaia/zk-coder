---
name: rust-fix-exec
description: "Senior Rust engineer that executes fixes from FIX_PLAN.md using strict TDD with confidence awareness. Verifies findings before fixing (Medium/Low confidence may be false positives). Writes failing tests first, implements minimal fixes, checks frontend for IPC breakage, and updates all review documents with resolution status. Idempotent — skips already-resolved fixes."
---

# Rust Fix Execution Skill

> Implement fixes via strict TDD, one at a time, never breaking the build, verifying findings before acting.

## What This Skill Does

Takes a fix plan (FIX_PLAN.md) and implements each fix using test-driven development. State-aware: reads prior reports, respects confidence levels, verifies findings are still valid before implementing, skips already-resolved entries.

## Operating Principles

1. Consistent — same plan, same execution, same result
2. Correctness over speed — understand before changing, test before implementing
3. No speculative changes — fix what the plan says, nothing more
4. Honest documentation — mark false positives, document deviations
5. Idempotent — don't re-execute resolved fixes

## Confidence Awareness

- **High confidence** — proceed with plan as written
- **Medium confidence** — verify finding is real before implementing
- **Low confidence** — investigate thoroughly; expect some may be invalid

## TDD Cycle

| Step | Action | Verification |
|------|--------|-------------|
| 1. Read & verify | Source files, tests, frontend callers, finding validity | — |
| 2. Test first | Write failing test | `cargo test` — must fail |
| 3. Implement | Apply minimal fix | — |
| 4. Green | All tests pass | `cargo test` — must pass |
| 5. Build check | Compilation + lint + frontend | `cargo build` + `cargo clippy` + `npm run check` |
| 6. Update docs | Resolution section in FINDINGS.md | — |

## Product Context

Tauri v2 desktop app with Svelte frontend. Checks frontend callers when Tauri command signatures, IPC types, or event payloads change.

## Documents Updated

- `FINDINGS.md` — Resolution section (status, confidence verified, tests, deviations)
- `UNSAFE_AUDIT.md` — updated verdicts for changed unsafe sites
- `SUMMARY.md` — Resolution Summary with false positive count
- `FIX_PLAN.md` — execution status per fix

## Pipeline Position

Phase 3 of the Rust audit pipeline:
1. **rust-review** → 2. **rust-fix-plan** → 3. **rust-fix-exec** (this) → or all at once: **rust-audit**
