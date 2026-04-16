---
name: rust-fix-plan
description: "Plans concrete fixes for Rust code review findings. Reads docs/code-review/FINDINGS.md and UNSAFE_AUDIT.md, traces root causes through call chains, and produces a detailed FIX_PLAN.md with proposed changes, affected code paths, edge cases, test plans, and risk assessments. Use after rust-review has produced findings."
---

# Rust Fix Plan Skill

> Deep-dive into findings and produce precise, executable fix plans.

## What This Skill Does

Takes the output of a code review (FINDINGS.md, UNSAFE_AUDIT.md) and produces a concrete fix plan. For each selected finding, it reads the source, traces callers and callees, identifies the minimal change set, and documents everything an engineer needs to implement the fix.

## Prerequisites

These files must exist (produced by rust-review):
- `docs/code-review/FINDINGS.md`
- `docs/code-review/UNSAFE_AUDIT.md`
- `docs/code-review/SUMMARY.md`

## Output

| File | Purpose |
|------|---------|
| `docs/code-review/FIX_PLAN.md` | Detailed fix plans with root cause, proposed change, edge cases, test plan, risk |

## Budget

- **Max 5 fixes** planned (highest severity first)
- Every plan must be concrete enough to implement without re-reading the finding

## Per-Fix Structure

Each fix includes:
- **Root Cause Analysis** — what is wrong and why
- **Proposed Change** — files, diff sketch, what stays the same
- **Affected Code Paths** — functions, modules, callers
- **Edge Cases** — what the fix must handle
- **Test Plan** — existing tests, new tests needed, regression risks
- **Risk Assessment** — confidence, blast radius, breaking change

## Quality Bar

"Refactor error handling" is not a plan. "Change `unwrap()` on line 142 of `agent.rs` to `map_err(|e| AgentError::SessionInit(e))?` and propagate to callers X, Y, Z" is a plan.

## Usage

Default: plan fixes for all findings.

Filter by passing severity, category, or path as argument.

## Pipeline

This is Phase 2 of the Rust audit pipeline:

1. **rust-review** — find problems
2. **rust-fix-plan** (this skill) — plan fixes from findings
3. **rust-fix-exec** — implement fixes via TDD
4. **rust-audit** — run all three phases in sequence
