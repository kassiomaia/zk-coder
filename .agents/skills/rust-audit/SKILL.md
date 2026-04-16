---
name: rust-audit
description: "Full Rust audit-to-fix pipeline that runs review, fix planning, and TDD fix execution in strict sequence. Produces findings, plans concrete fixes, implements them with tests, and verifies the build stays green throughout. Use for end-to-end code quality improvement in a single pass."
---

# Rust Audit Pipeline Skill

> End-to-end: review, plan, fix — in one pass.

## What This Skill Does

Runs three phases in strict sequence, with gates between each:

| Phase | Role | Output |
|-------|------|--------|
| 1. Review | Find highest-risk issues (max 7) | SUMMARY.md, FINDINGS.md, UNSAFE_AUDIT.md |
| 2. Fix Planning | Plan concrete fixes (max 5) | FIX_PLAN.md |
| 3. Fix Execution | Implement via TDD | Updated docs + working code |

Each phase must complete fully before the next begins.

## Phase Gates

```
═══ Phase 1 COMPLETE — Review ═══
Findings: N | Critical: N | High: N | Medium: N | Low: N
Proceeding to Phase 2...

═══ Phase 2 COMPLETE — Fix Planning ═══
Fixes planned: N | Deferred: N
Proceeding to Phase 3...

═══ Phase 3 COMPLETE — Fix Execution ═══
Fixes implemented: N | Deferred: N | Tests added: N
Build: pass | Tests: pass | Clippy: pass | Frontend: pass
```

## Abort Conditions

- Phase 1 finds 0 issues → pipeline ends cleanly
- Phase 2 determines all findings are false positives → pipeline ends after Phase 2
- Phase 3 fix causes cascading breakage (>3 unexpected files) → stop, document, ask for guidance

## Key Constraints

- **Max 5 files** reviewed deeply (Phase 1)
- **Max 7 findings** reported (Phase 1)
- **Max 5 fixes** planned (Phase 2)
- **TDD enforced** — failing test before every fix (Phase 3)
- **Build never breaks** between fixes (Phase 3)
- **Frontend checked** when Tauri IPC contracts change (Phase 3)

## Documents Produced

All under `docs/code-review/`:
- `SUMMARY.md` — overview + resolution summary
- `FINDINGS.md` — detailed findings with resolution status
- `UNSAFE_AUDIT.md` — unsafe site audit trail
- `FIX_PLAN.md` — fix plans with execution status

## Usage

Default scope: all `.rs` files under `src/` and `src-tauri/`.

Narrow scope by passing a path or glob as argument.

## Component Skills

This pipeline composes three standalone skills:
1. **rust-review** — can be run independently for review-only
2. **rust-fix-plan** — can be run independently if findings already exist
3. **rust-fix-exec** — can be run independently if a fix plan already exists
