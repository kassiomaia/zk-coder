---
name: rust-fix-exec
description: "Senior Rust engineer that executes fixes from FIX_PLAN.md using strict TDD. Writes failing tests first, implements minimal fixes, verifies build and tests after each change, checks frontend for IPC breakage, and updates review documents with resolution status. Use after rust-fix-plan has produced a plan."
---

# Rust Fix Execution Skill

> Implement fixes via strict TDD, one at a time, never breaking the build.

## What This Skill Does

Takes a fix plan (FIX_PLAN.md) and implements each fix using test-driven development. Understands the Tauri v2 + Svelte product context and checks for frontend breakage when backend contracts change.

## Prerequisites

These files must exist:
- `docs/code-review/FIX_PLAN.md` (produced by rust-fix-plan)
- `docs/code-review/FINDINGS.md` (produced by rust-review)
- `docs/code-review/UNSAFE_AUDIT.md` (produced by rust-review)

Must also read before coding:
- `AGENTS.md` — project rules and architecture
- `RULES.md` — coding rules and style

## Core Principles

- **TDD: test first, code second**
- **One fix at a time** — complete fully before starting the next
- **Never break the build** — `cargo build` + `cargo test` must pass after each fix
- **Minimal changes** — fix what the plan says, nothing more
- **Product-aware** — check frontend when Tauri commands/events change

## TDD Cycle Per Fix

| Step | Action | Verification |
|------|--------|-------------|
| 1. Read | Source files, tests, frontend callers | — |
| 2. Test first | Write failing test | `cargo test` — must fail |
| 3. Implement | Apply minimal fix | — |
| 4. Green | All tests pass | `cargo test` — must pass |
| 5. Build check | Compilation + lint | `cargo build` + `cargo clippy` |
| 6. Frontend check | If IPC changed | `npm run check` from `src-ui/` |
| 7. Update docs | Resolution section in FINDINGS.md | — |

## Documents Updated

- `docs/code-review/FINDINGS.md` — adds Resolution section per finding
- `docs/code-review/UNSAFE_AUDIT.md` — updates unsafe sites if changed
- `docs/code-review/SUMMARY.md` — adds Resolution Summary after all fixes
- `docs/code-review/FIX_PLAN.md` — marks execution status per fix

## Final Verification

After all fixes:
```bash
cargo build && cargo test && cargo clippy
cd src-ui && npm run check
```

## Usage

Default: execute all fixes in FIX_PLAN.md in order.

Filter by passing fix numbers, severity, or category as argument.

## Pipeline

This is Phase 3 of the Rust audit pipeline:

1. **rust-review** — find problems
2. **rust-fix-plan** — plan fixes from findings
3. **rust-fix-exec** (this skill) — implement fixes via TDD
4. **rust-audit** — run all three phases in sequence
