---
name: rust-review
description: "Autonomous Rust code review agent that maintains persistent, evolving assessments. Triages for high-risk areas (unsafe, FFI, concurrency), picks at most 5 files for deep review, produces max 7 evidence-based findings with confidence levels. Reports are idempotent and diffable. Classifies unsafe as Sound/Likely Sound/Risky/Dangerous. Outputs SUMMARY.md, FINDINGS.md, UNSAFE_AUDIT.md under docs/code-review/."
---

# Rust Code Review Skill

> Autonomous, state-aware Rust code review — persistent reports that evolve over time.

## What This Skill Does

Maintains an accurate, evolving assessment of code safety, correctness, and maintainability. Not a one-off scanner — produces persistent, diffable reports that future runs and engineers build upon.

## Operating Principles

1. Consistent across runs — same code, same findings, same titles
2. Correctness over coverage — deep on fewer files, not shallow on many
3. No speculative claims — only report what the code supports
4. Signal quality — no noise, no generic advice, no cosmetic findings
5. Idempotent updates — running twice on unchanged code produces the same result

## Outputs

| File | Purpose |
|------|---------|
| `docs/code-review/SUMMARY.md` | Scope, totals, top risks |
| `docs/code-review/FINDINGS.md` | Detailed findings (max 7) with confidence levels |
| `docs/code-review/UNSAFE_AUDIT.md` | Unsafe site audit with Sound/Likely Sound/Risky/Dangerous verdicts |

## Key Models

**Severity:** Critical > High > Medium > Low

**Confidence:** High (code-supported) > Medium (assumption-dependent) > Low (needs confirmation)

**Unsafe Verdicts:** Sound | Likely Sound | Risky | Dangerous

**Finding Discipline** — only create a finding if:
- Correctness or safety risk exists
- Missing or weak invariant in unsafe code
- Code hides critical behavior or assumptions
- Design significantly increases future bug risk

## Pipeline Position

Phase 1 of the Rust audit pipeline:
1. **rust-review** (this) → 2. **rust-fix-plan** → 3. **rust-fix-exec** → or all at once: **rust-audit**
