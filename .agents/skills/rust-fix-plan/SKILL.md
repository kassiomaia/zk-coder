---
name: rust-fix-plan
description: "Plans concrete fixes for Rust code review findings with confidence-aware triage. Reads FINDINGS.md and UNSAFE_AUDIT.md, traces root causes through call chains, produces idempotent FIX_PLAN.md with proposed changes, affected code paths, edge cases, test plans, and risk assessments including confidence and missing-context notes. Skips already-resolved findings."
---

# Rust Fix Plan Skill

> Deep-dive into findings, trace root causes, produce precise executable fix plans.

## What This Skill Does

Takes review output (FINDINGS.md, UNSAFE_AUDIT.md) and produces concrete fix plans. Respects finding confidence levels — skips low-confidence findings unless Critical. Plans are persistent, idempotent, and skip already-resolved findings.

## Operating Principles

1. Consistent across runs — same findings produce same plans
2. Correctness over speed — read code, trace call chains, understand before proposing
3. No speculative fixes — flag unknowns explicitly
4. Concrete plans — "change unwrap() on line 142 to map_err(...)?" not "refactor error handling"
5. Idempotent updates — don't re-plan resolved findings

## Prerequisites

Requires existing review output:
- `docs/code-review/FINDINGS.md`
- `docs/code-review/UNSAFE_AUDIT.md`
- `docs/code-review/SUMMARY.md`

## Output

| File | Purpose |
|------|---------|
| `docs/code-review/FIX_PLAN.md` | Root cause analysis, proposed changes, edge cases, test plans, risk with confidence |

## Triage Criteria

Selects at most 5 findings based on:
- Severity (Critical and High first)
- Finding confidence (skip Low unless Critical)
- Feasibility of a clean fix
- Skips already-resolved findings

## Per-Fix Structure

Root Cause Analysis, Proposed Change, Affected Code Paths, Edge Cases, Test Plan, Risk Assessment (confidence, blast radius, breaking change, missing context)

## Pipeline Position

Phase 2 of the Rust audit pipeline:
1. **rust-review** → 2. **rust-fix-plan** (this) → 3. **rust-fix-exec** → or all at once: **rust-audit**
