---
name: rust-review
description: "Focused Rust code review agent. Triages the codebase for high-risk areas (unsafe, FFI, concurrency, error boundaries), picks at most 5 files to review deeply, and produces max 7 high-confidence findings. Outputs SUMMARY.md, FINDINGS.md, and UNSAFE_AUDIT.md under docs/code-review/. Use when you need to identify safety, correctness, or design issues in Rust code."
---

# Rust Code Review Skill

> Focused, deep Rust code review — quality over quantity.

## What This Skill Does

Performs a targeted review of Rust source files, prioritizing high-risk areas. Instead of scanning everything, it triages the codebase, selects the most dangerous files, and goes deep on them.

## Outputs

| File | Purpose |
|------|---------|
| `docs/code-review/SUMMARY.md` | Short overview: scope, totals, top risks |
| `docs/code-review/FINDINGS.md` | Detailed findings (max 7), ordered by severity |
| `docs/code-review/UNSAFE_AUDIT.md` | Audit trail for every unsafe site in reviewed files |

## Budgets

- **Max 5 files** reviewed deeply
- **Max 7 findings** reported
- Only findings directly supported by the code

## Severity Levels

- **Critical** — likely UB, memory corruption, data race, unsound unsafe, invalid FFI
- **High** — likely correctness bug, broken invariant, deadlock risk, dangerous panic
- **Medium** — design issue likely to cause future bugs, pattern misuse, perf smell
- **Low** — minor but real clarity or structure issue

## Categories

Unsafe Risk, Correctness Risk, Maintainability Smell, Performance Smell, API Design Smell

## Usage

Default scope: all `.rs` files under `src/` and `src-tauri/`.

Narrow the scope by passing a path or glob as argument.

## Pipeline

This is Phase 1 of the Rust audit pipeline:

1. **rust-review** (this skill) — find problems
2. **rust-fix-plan** — plan fixes from findings
3. **rust-fix-exec** — implement fixes via TDD
4. **rust-audit** — run all three phases in sequence
