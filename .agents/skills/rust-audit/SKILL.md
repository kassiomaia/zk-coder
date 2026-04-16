---
name: rust-audit
description: "Full autonomous Rust audit-to-fix pipeline: review, plan, fix in strict sequence. State-aware with idempotent, diffable reports. Confidence-aware triage (skips low-confidence unless Critical, verifies before fixing, tracks false positives). TDD execution with frontend IPC awareness. Produces persistent evolving documentation under docs/code-review/."
---

# Rust Audit Pipeline Skill

> End-to-end: review, plan, fix — autonomous, state-aware, idempotent.

## What This Skill Does

Runs three phases in strict sequence with gates between each. Maintains persistent repository-local reports that evolve over time. Confidence-aware throughout: findings carry confidence levels, plans respect them, execution verifies before acting.

## Operating Principles (all phases)

1. Consistent across runs — same code, same results
2. Correctness over coverage — deep on fewer files, not shallow on many
3. No speculative claims — only report what code supports
4. Signal quality — no noise, no generic advice
5. Idempotent — running twice on unchanged code produces same result; resolved findings are not re-processed

## Pipeline

| Phase | Role | Budget | Output |
|-------|------|--------|--------|
| 1. Review | Find highest-risk issues | 5 files, 7 findings | SUMMARY.md, FINDINGS.md, UNSAFE_AUDIT.md |
| 2. Planning | Plan concrete fixes | 5 fixes | FIX_PLAN.md |
| 3. Execution | Implement via TDD | All planned fixes | Updated docs + working code |

## Key Models

**Severity:** Critical > High > Medium > Low
**Confidence:** High > Medium > Low (downgrade when context is missing)
**Unsafe Verdicts:** Sound | Likely Sound | Risky | Dangerous

## Confidence Flow

- Review tags each finding with confidence
- Planning uses confidence for triage (skip Low unless Critical)
- Execution verifies findings before implementing (Medium/Low may be false positives)
- Resolution documents whether confidence was confirmed

## Abort Conditions

- Phase 1: 0 issues → pipeline ends cleanly
- Phase 2: all findings are false positives or low-confidence → ends after Phase 2
- Phase 3: cascading breakage >3 unexpected files → stop, document, ask for guidance

## Product Context

Tauri v2 desktop app with Svelte frontend. Phase 3 checks frontend callers when Tauri IPC contracts change.

## Component Skills

Each phase can run independently:
1. **rust-review** — review only
2. **rust-fix-plan** — plan from existing findings
3. **rust-fix-exec** — execute from existing plan
