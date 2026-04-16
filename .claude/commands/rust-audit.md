You are a senior Rust engineering lead running a full audit-to-fix pipeline.

You will execute three phases in strict sequence. Each phase must complete fully before the next begins. Do not skip phases. Do not parallelize phases.

If `$ARGUMENTS` is provided, pass it as the scope to every phase.

---

## Phase 1 — Review

Execute the review phase. Act as described below.

### Goal
Identify the highest-risk issues in the Rust codebase. Focused, deep review — not exhaustive.

### Steps
1. List the file tree to understand project structure.
2. Triage: grep for `unsafe`, concurrency primitives, FFI markers, complex error handling.
3. Pick at most 5 high-risk files/modules to review deeply.
4. Read those files fully. Analyze for: unsafe risks, correctness bugs, maintainability smells, performance smells, API design problems.
5. For every unsafe site, audit: why unsafe is needed, what invariants must hold, where they are enforced, whether documented, whether scope is minimal.
6. Write `docs/code-review/FINDINGS.md` — max 7 findings, ordered by severity (Critical > High > Medium > Low).
7. Write `docs/code-review/UNSAFE_AUDIT.md` — one entry per unsafe site in reviewed files.
8. Write `docs/code-review/SUMMARY.md` — include which files were reviewed and why.

### Severity levels
- **Critical** — likely UB, memory corruption, data race, unsound unsafe, invalid FFI
- **High** — likely correctness bug, broken invariant, deadlock risk, dangerous panic
- **Medium** — design issue likely to cause future bugs, meaningful pattern misuse, material perf smell
- **Low** — minor but real clarity or structure issue

### Categories
Unsafe Risk, Correctness Risk, Maintainability Smell, Performance Smell, API Design Smell

### Gate
Phase 1 is complete when all three files exist and contain findings (or explicitly state none found). Print:

```
═══ Phase 1 COMPLETE — Review ═══
Findings: N | Critical: N | High: N | Medium: N | Low: N
Proceeding to Phase 2...
```

---

## Phase 2 — Fix Planning

Read the review output and plan concrete fixes.

### Prerequisites
Read before planning:
- `docs/code-review/FINDINGS.md`
- `docs/code-review/UNSAFE_AUDIT.md`
- `docs/code-review/SUMMARY.md`

### Steps
1. Triage findings: select at most 5 for fix planning (Critical and High first).
2. For each selected finding:
   - Read the source file at the reported location
   - Read callers and callees to understand data flow
   - Read related tests if they exist
   - Identify the minimal change set
   - Identify edge cases the fix must handle
3. Write `docs/code-review/FIX_PLAN.md` with:
   - Selection table (which findings, why selected)
   - Deferred table (which findings skipped, why)
   - For each fix: root cause analysis, proposed change, affected code paths, edge cases, test plan, risk assessment

### Quality bar
Every fix plan entry must be concrete enough that an engineer can implement it without re-reading the finding. "Refactor error handling" is not a plan. "Change `unwrap()` on line 142 to `map_err(...)?` and propagate to callers X, Y, Z" is.

### Gate
Phase 2 is complete when `FIX_PLAN.md` exists with at least one fix planned. Print:

```
═══ Phase 2 COMPLETE — Fix Planning ═══
Fixes planned: N | Deferred: N
Proceeding to Phase 3...
```

---

## Phase 3 — Fix Execution (TDD)

Implement the planned fixes using strict TDD.

### Prerequisites
Read before coding:
- `AGENTS.md` — project rules, architecture
- `RULES.md` — coding rules
- `docs/code-review/FIX_PLAN.md` — what to implement

This is a Tauri v2 desktop app with a Svelte frontend. Backend changes can break IPC contracts, command signatures, and event payloads. Be aware of this throughout.

### Per-fix cycle

Execute fixes **one at a time**, in order. For each:

**1. Read** — source files, existing tests, frontend callers of affected Tauri commands

**2. Test first** — write or update a test that captures the problem. Run `cargo test` — the new test must fail (or demonstrate the issue). If it passes, investigate before proceeding.

**3. Implement** — apply the fix. Minimal change. If the plan has gaps, use judgment and document the deviation.

**4. Green** — run `cargo test`. All tests pass. If others break:
  - Expected breakage (old test encoded broken behavior) → update the test
  - Unexpected breakage → investigate, adjust the fix

**5. Build check** — run `cargo build` and `cargo clippy`. If you changed Tauri command signatures, IPC types, or event payloads:
  - Check `src-ui/` for TypeScript callers
  - Update frontend if needed
  - Run `npm run check` from `src-ui/`

**6. Update docs** — add a `### Resolution` section to the finding in `FINDINGS.md`:
  ```
  ### Resolution
  **Status:** Fixed
  **Commit:** <short description>
  **Tests added:** <list>
  **Deviations from plan:** <none, or describe>
  ```
  Update `UNSAFE_AUDIT.md` if the fix involved an unsafe site.

### After all fixes

1. Run full verification:
   ```bash
   cargo build && cargo test && cargo clippy
   cd src-ui && npm run check
   ```

2. Update `docs/code-review/SUMMARY.md` — add:
   ```markdown
   ## Resolution Summary
   - Fixes implemented: N of M planned
   - Fixes deferred: N (with reasons)
   - Tests added: N
   - Frontend changes required: Yes/No

   ### Fix Log
   | # | Finding | Status | Tests Added | Notes |
   |---|---------|--------|-------------|-------|
   ```

3. Update `docs/code-review/FIX_PLAN.md` — mark each fix with execution status.

### Gate
Phase 3 is complete when all planned fixes are implemented (or explicitly deferred with reason), all tests pass, and the build is clean. Print:

```
═══ Phase 3 COMPLETE — Fix Execution ═══
Fixes implemented: N | Deferred: N | Tests added: N
Build: ✓ | Tests: ✓ | Clippy: ✓ | Frontend: ✓
```

---

## Abort Conditions

Stop the pipeline and report to the user if:
- Phase 1 finds 0 issues — nothing to fix, pipeline ends cleanly after Phase 1
- Phase 2 determines all findings are false positives — document why, pipeline ends after Phase 2
- Phase 3 fix causes cascading breakage touching more than 3 unexpected files — stop, document, ask for guidance

## Final Output

After all phases complete (or the pipeline aborts), print:

```
═══ RUST AUDIT PIPELINE COMPLETE ═══

Phase 1 — Review:     N findings (C critical, H high, M medium, L low)
Phase 2 — Planning:   N fixes planned, N deferred
Phase 3 — Execution:  N fixed, N deferred, N tests added

Documents updated:
  docs/code-review/SUMMARY.md
  docs/code-review/FINDINGS.md
  docs/code-review/UNSAFE_AUDIT.md
  docs/code-review/FIX_PLAN.md

Build status: ✓ passing
Test status:  ✓ passing
```
