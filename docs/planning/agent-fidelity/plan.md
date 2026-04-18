# Agent Fidelity — Execution Plan

**Status**: IN PROGRESS
**Last reviewed**: 2026-04-18

## Progress

- **Phase 1 — DELIVERED (commit `7df7589`)**: A (rehydration fidelity: title + description round-trip), H.1 (prompt split into harness_contract + quality_baseline + default_code_framing with SystemPromptBuilder), H.4 (prompt stability attribution helpers), H.5 (runtime telemetry emits three section hashes alongside systemPromptHash).
- Phase 2 — pending: C + D + H.6
- Phase 3 — pending: B
- Phase 4 — pending: E + H.2
- Phase 5 — pending: F + H.3
- Phase 6 — pending: G
**Purpose**: Close the gap between the harness's mechanical correctness and the agent's actual fidelity to the user-authored plan. The context-engineering work in `../agent-context-engineering/` delivered stable prompts, deterministic packing, and measurement signals. This plan addresses the behaviors those signals exposed: subtask scope drift, stale failure memory, reload rehydration loss, and fabrication in the absence of user replies.

---

## Summary

This is a follow-on to the agent-context-engineering plan. It converts the findings captured during the M4.3 characterization baseline and the `a597debb-c5b7-49eb-913e-7b11c7e4ac84` trace review into a concrete, sliced workstream. Each workstream ships independently, preserves existing invariants (deterministic packing, stable catalog, FullAccess unrestricted), and adds a regression test before touching runtime.

Evidence sources:

1. Session `a597debb-c5b7-49eb-913e-7b11c7e4ac84` (2026-04-18) — "create release communication files" workflow on a cross-platform agent-coder launch. Read-only trace review in `~/.local/share/com.coder.app/coder_agents.db`.
2. M4.3 characterization baseline (commit `888c6ea`) — captured in `../agent-context-engineering/measurement-harness.md`.
3. User-reported reload bug on 2026-04-18 — truncated-title + full-content double-render after application restart.
4. Review of `src-tauri/src/infrastructure/agent/registry.rs:112` `code_agent_system_prompt()` on 2026-04-18 — code-specific persona fused with harness-contract language, no scope-drift consequence language, no narration-intent semantics, no compaction or contract awareness.

Companion design docs:

- [System Prompt Architecture](./system-prompt-architecture.md) — what a harness system prompt must contain, organized by layer. Source of truth for workstream H wording.

---

## Problem inventory

| ID | Problem | Evidence | Fix workstream |
|----|---------|----------|----------------|
| P1 | Task title/description dropped on reload; UI falls back to 80-char truncation + full content, producing a double-render | `src-tauri/src/infrastructure/sqlite/repositories/agent/tasks.rs:63-68` SELECT omits `title, description`; line 118 hardcodes `title: None` | A |
| P2 | Subtask scope drift unchecked — agent wrote 8 email files under "social media copy snippets" subtask 3; 0 social-media artifacts produced | `agent_tool_calls.subtask_id` maps 8 `email/*.md` writes to subtask 3 (`3544a3b3-d152-4ea8-8a86-4565825a5a52`) | B |
| P3 | Failure memory leaks across subtasks as stale blocker | Step 35 (subtask-3 start) prompt still injects `fs_read_file README.md file_not_found` from step 22 (subtask-2) | C |
| P4 | Execution brief `next:` field encourages lookahead blending | Model began subtask-4 work during subtask-3 window | D |
| P5 | `agent_narrate` clarification non-blocking — model fabricates when user doesn't answer | Step 5 asked for product info via `agent_narrate`; no reply; agent wrote `[Product Name]`/`[COMPANY NAME]`/`[Date]` placeholders for the next 40 steps | E |
| P6 | Clean-exit criterion doesn't verify planned artifacts were produced | Captured M4.3 finding; subtask 3 marked Completed with 0 social-media artifacts | F (depends on B) |
| P7 | `FailureClass::NotAllowed` injects misleading "change args before retry" guidance for mode-disallowed failures | Captured M4.3 finding | C |
| P8 | System prompt embeds a code-specific persona in the agent itself. Because one agent serves all task types, the persona is either too specific (today: "senior full-stack engineer" — wrong for communication work) or too generic (and then supplies no useful framing). Persona belongs with the task, not the agent. The prompt also uses descriptive rather than constraining language around subtasks and lacks vocabulary for contracts, scope drift, narration-as-question, failure memory, or clean exit. | `src-tauri/src/infrastructure/agent/registry.rs:112-314` is a single `const fn -> &'static str` fusing code-work defaults (full-stack stack, file-size limits, typing rules) with harness steering. 2026-04-18 communication-workflow session inherited code-engineer framing and produced code-engineer-flavored fabrications (IDE plugins, LSP, JetBrains). | H |

---

## Core rules

Inherited from `../agent-context-engineering/plan.md`:

1. One stable source of truth — this file for this plan.
2. Each workstream must be shippable alone.
3. Characterization or regression tests precede behavior changes.
4. Determinism over cleverness.
5. Runtime feedback mandatory — reuse the five M4.3 signals; add `scope_drift_event_count` for workstream B.
6. Preserve existing invariants: stable catalog, deterministic packing, FullAccess unrestricted.

---

## Workstream A — Rehydration fidelity

**Fixes**: P1
**Ships alone**: yes
**Risk**: low
**Blast radius**: DB read path + one struct field

### A.1 — Hydrate `title` and `description` on DB load

- **Deliverable**: after reload, `get_session_tasks` returns the LLM-generated title and description.
- **Build**:
  - extend SELECT in `get_session_tasks` (`tasks.rs:63-68`) to include `title, description`
  - read columns into the struct; replace `title: None` on line 118 with the loaded value
- **Tests (first)**:
  - create a task with title+description, reload via `get_session_tasks`, assert both preserved
  - task with neither field stored preserves `None` on load
- **Done when**: reload of session `a597debb…ac84` shows "Create release communication files and copy" as the task title, no 80-char truncation artifact.

### A.2 — Add `description` field to `AgentTask`

- **Deliverable**: `AgentTask` carries `description: Option<String>` alongside the existing `title`.
- **Build**:
  - add field on `src-tauri/src/core/agent/task.rs:49`
  - propagate through `BackendAgentTask` on the frontend side (`src-ui/src/lib/types/agent.ts:202`) — already expects optional `description`, no change needed there
  - write path already records description (`tasks.rs:137-152`); verify end-to-end
- **Done when**: the DB column round-trips without manual JSON munging.

### A.3 — Regression test

- Round-trip test in `infrastructure/sqlite/repositories/agent/tasks.rs` test module that creates → loads → asserts.
- **Done when**: test is green and would have caught the original regression.

---

## Workstream B — Subtask contracts

**Fixes**: P2 (foundation for P6, F)
**Ships alone**: yes, but behavior-changing
**Risk**: medium — touches a core entity and the `agent_plan_subtasks` schema
**Blast radius**: subtask entity, planning tool schema, runtime enforcement, DB migration

### B.1 — Add `SubTaskContract` to the entity

- **Deliverable**: each subtask optionally declares an expected-output shape.
- **Build**:
  - add `ArtifactSpec { path_pattern: String, kind: ArtifactKind }` where `ArtifactKind ∈ {File, Document, Command, Other}`
  - add `contract: Option<SubTaskContract>` to `SubTask` in `task.rs:324`
  - DB migration: `ALTER TABLE agent_subtasks ADD COLUMN contract TEXT`
- **Tests (first)**: serialize/deserialize round-trip; DB round-trip.
- **Done when**: entity + DB reflect optional contracts; no existing behavior changes when contracts are absent.

### B.2 — Extend `agent_plan_subtasks` to emit contracts

- **Deliverable**: planning tool schema requires a contract per subtask (optional field, but described as strongly recommended in the tool doc).
- **Build**:
  - update tool input schema; add guidance in tool description: "for each subtask, specify where its outputs must live (e.g. `docs/communications/social/twitter-*.md`)"
  - planning tests assert subtasks carry contracts
- **Done when**: future plans land with contracts; old plans (contract-free) continue to work.

### B.3 — Runtime drift detection in `tool_phase`

- **Deliverable**: write-class tool calls whose output path doesn't match the active subtask's contract emit a `subtask_scope_drift` runtime trace.
- **Build**:
  - in `tool_phase`, after a write-class tool call succeeds, compare its effect path against `ctx.active_subtask.contract`
  - on mismatch, emit `agent_runtime_traces` row with type `subtask_scope_drift`, payload `{tool_name, written_path, expected_patterns}`
  - **soft enforcement first**: also inject a correction message into the next step's context: "The last write went to X, which is outside this subtask's expected outputs (Y). Either rescope or explicitly mark the subtask Failed with reason."
- **Tests (first)**: synthetic subtask + contract + drifting tool call → trace emitted + correction appended.
- **Done when**: re-running the 2026-04-18 workflow shows ≥1 `subtask_scope_drift` trace for the email-under-social-media writes.

### B.4 — Hard enforcement switch (optional, later)

- Gate behind a feature flag: when enabled, drifting write-class tool calls are rejected with `subtask_scope_drift` error (same code path as `subtask_mode_disallowed`).
- Ship disabled by default until soft enforcement shows the soft correction is being respected.

---

## Workstream C — Failure memory hygiene

**Fixes**: P3, P7
**Ships alone**: yes
**Risk**: low — prompt-assembly changes with existing test scaffolding in `prompt_stability_tests.rs`
**Blast radius**: `ContextPolicyV2` failure-memory field

### C.1 — Scope failures by subtask

- **Deliverable**: each memorized failure carries its originating `subtask_id`.
- **Build**:
  - extend the failure-memory entry struct with `subtask_id: Option<String>`
  - capture the current subtask at record time
- **Tests (first)**: record a failure under subtask X; transition to subtask Y; assert failure is not surfaced in the Y prompt unless explicitly unscoped.

### C.2 — Drop stale failures on subtask transition

- **Deliverable**: the execution brief's `blocker:` line and the `failure memory` system message only surface failures belonging to the current subtask (or unscoped).
- **Build**: filter on assembly; drop failures whose `subtask_id != current`.
- **Tests (first)**: regression test using the exact sequence from session `a597debb…ac84` (step 22 fs_read_file failure → step 35 subtask-3 prompt should not include it).

### C.3 — Refine `FailureClass::NotAllowed` guidance

- **Deliverable**: distinguish mode-disallowed failures from argument-level rejections in the memory copy.
- **Build**: classifier already emits `NotAllowed` for both cases; split into `NotAllowedByMode` and `NotAllowedByArgs`; tailor the `retry:` guidance accordingly.
- **Tests (first)**: unit tests for both branches.

---

## Workstream D — Execution-brief discipline

**Fixes**: P4
**Ships alone**: yes
**Risk**: low — one-field prompt tweak
**Blast radius**: execution-brief assembly

### D.1 — Gate `next:` behind transition readiness

- **Deliverable**: `next: <subtask_title>` appears only when the current subtask's contract is satisfied or the model has explicitly signaled readiness via `agent_advance_to_next_subtask` (which is already a terminal signal).
- **Build**:
  - execution-brief assembly checks `ctx.current_subtask.contract_satisfied()` (requires B.3) or falls back to always-hide until B lands
  - interim implementation (pre-B): hide `next:` unconditionally; models were not using it productively in the observed session
- **Tests (first)**: characterization test captures current brief shape; updated test captures the gated shape.

---

## Workstream E — Clarification blocking

**Fixes**: P5
**Ships alone**: no — touches UI + session lifecycle
**Risk**: medium — new `HarnessMode` variant, new UI affordance
**Blast radius**: `agent_narrate` tool, `HarnessMode` enum, harness state store, AgentPanel

### E.1 — Typed narration intent

- **Deliverable**: `agent_narrate` gains `intent: "narration" | "question"`.
- **Build**: tool-input schema change; downstream `NarrationEvent` carries the intent.

### E.2 — `AwaitingClarification` harness mode

- **Deliverable**: when `intent == "question"`, the harness state store transitions to `HarnessMode::AwaitingClarification { prompt, asked_at }`; the main loop pauses.
- **Build**:
  - add variant to `HarnessMode` (`src-tauri/src/application/agent/read_models.rs`)
  - main loop checks harness mode before each LLM call; returns control if awaiting
  - user reply delivered via existing message-input path clears the state and resumes

### E.3 — Timeout fallback

- **Deliverable**: after N minutes (configurable, default 30) without reply, the current subtask fails with reason `unanswered_clarification`.
- **Done when**: the regression reproducer (empty workspace + narration question + no reply) fails the subtask cleanly instead of fabricating.

### E.4 — Frontend affordance

- **Deliverable**: AgentPanel surfaces the clarification question inline with a "reply" composer bound to the session; re-uses existing message-input plumbing.

---

## Workstream F — Clean-exit contract verification

**Fixes**: P6
**Depends on**: B.1–B.3
**Ships alone**: after B
**Risk**: medium — changes the semantics of "Completed"

### F.1 — Verify artifacts before `Completed`

- **Deliverable**: `agent_advance_to_next_subtask` and `agent_mark_subtask_status(Completed)` both verify the current subtask's contract before succeeding.
- **Build**:
  - check expected artifacts exist (paths on disk, or recorded in `agent_files_touched`)
  - on mismatch, reject with `contract_unsatisfied` and a list of missing artifacts
- **Tests (first)**: subtask with contract requiring `docs/communications/social/*.md` + no such files → advance rejected.

### F.2 — Needs-review flag for mismatch

- **Deliverable**: if the model persists in marking the subtask Completed (e.g. via explicit `Failed` with override), the session flips `needsReview = true`.

---

## Workstream G — Plan quality validation (optional)

**Fixes**: plan hygiene issues adjacent to B
**Ships alone**: yes
**Risk**: low

### G.1 — Reject overlapping or redundant plans

- **Deliverable**: `agent_plan_subtasks` post-validates that no two subtasks declare overlapping contract paths, and that the final subtask isn't a no-op index of prior work.
- **Done when**: the original session's plan (subtask 5 "Generate documentation files in ./docs/communications/" — redundant with subtasks 0-4) would be rejected with a correction message.

---

## Workstream H — System-prompt alignment

**Fixes**: P8 (enables every other workstream to land with language the model can act on)
**Ships alone**: partially — H.1 ships alone; H.3 lands *with* B; H.4 tracks prompt evolution continuously
**Risk**: medium — changes the most-hashed string in the runtime; must preserve cache-prefix stability
**Blast radius**: `code_agent_system_prompt()`, any agent registered in `AgentRegistry`, prompt-stability tests

Full design rationale and template wording live in [`./system-prompt-architecture.md`](./system-prompt-architecture.md). This workstream is the execution plan for landing that design. Wording details belong in the companion doc; here we track deliverables, tests, and sequencing.

### H.1 — Remove persona from the agent; keep a genre-neutral quality baseline

- **Deliverable**: `code_agent_system_prompt()` decomposes into two sections assembled once at agent construction, not per step. Neither section carries code-specific persona.
  - `harness_contract_prompt()` — invariant. Teaches the execution model, subtask contracts, scope-drift consequences, narration-intent semantics, failure-memory scoping rules, clean-exit verification, compaction awareness.
  - `quality_baseline_prompt()` — invariant, genre-neutral. 5–8 lines that set a professional bar without domain: verify before asserting, write in the voice the task calls for, stop rather than fabricate, treat output as shippable.
- **Moved out of the agent prompt**: full-stack stack defaults, typing rules, file-size limits, code Definition of Done. These are *code-work framing* and belong with the task (see H.2), not baked into the engine.
- **Build**:
  - introduce a `SystemPromptBuilder` in `src-tauri/src/infrastructure/agent/registry.rs` that concatenates `harness_contract + quality_baseline`
  - freeze the result in the `Agent` struct at construction (no recomputation per call — keep the prefix cacheable)
  - preserve the current `derive_system_prompt()` appendix structure for active-subtask context
- **Tests (first)**:
  - `harness_contract_prompt()` and `quality_baseline_prompt()` are byte-identical across runs
  - composed result contains no code-specific tokens (`Bun`, `Elysia`, `Drizzle`, `React`, `TypeScript`, `900 lines`, etc.)
  - `derive_system_prompt(base, None)` returns `base` unchanged — no appendix when no active subtask

### H.2 — Task-declared framing

- **Deliverable**: persona arrives with the task as a `TaskFraming` record, not baked into the agent. The record supplies role, voice constraints, and domain priors for *this* task only.
- **Shape** (`TaskFraming`):
  - `role` — short descriptor: "senior full-stack engineer", "senior communication professional", "technical writer"
  - `voice_constraints` — opinionated list: e.g. *"one claim per paragraph; no adjectives without a concrete number; no filler openers"*. Bland one-line declarations are rejected.
  - `domain_priors` — optional, structured: stack defaults for code work, style references for writing work, artifact conventions. This is where the old code-agent text lives when framing is coding work.
  - `forbidden_patterns` — optional regex list: e.g. *"the future of", "pioneering", "empower", "unprecedented"* for launch copy.
- **How it arrives**: extend `agent_plan_subtasks` to emit a `TaskFraming` alongside the subtask list as the task's first public commitment. The model declares the framing; the user sees and can override before execution. (User-supplied framing and inferred-framing variants are deferred — pick up if the model-declared path proves insufficient.)
- **How it's used**: framing becomes a third concatenated section between harness contract and subtask appendix. Hashes as `framingHash` (H.5). Stable for the duration of the task.
- **Tests (first)**:
  - framing with fewer than N `voice_constraints` is rejected at planning time
  - framing hash stable across subtask transitions within one task; changes on task boundary
  - a re-run of the 2026-04-18 workflow produces a `communication` framing, not a `code` one, and the composed prompt contains no Bun/Elysia defaults
- **Done when**: no task-type-specific language lives inside the agent prompt; every task has a declared framing visible to the user before execution.

### H.3 — Contract-aware active-subtask appendix

- **Deliverable**: `derive_system_prompt()` appendix surfaces the active subtask's contract when present, using wording that matches the runtime enforcement in workstream B.
- **Build**:
  - extend the appendix to render expected-artifact patterns: "Expected outputs for this subtask: `docs/communications/social/*.md`. Writes outside this set will be flagged as scope drift."
  - fall back to the current appendix wording when no contract is attached (backward-compatible with pre-B plans)
- **Tests (first)**: appendix-with-contract renders the pattern list; appendix-without-contract matches the current wording exactly (no regression for legacy plans).
- **Dependency**: B.1 (`SubTaskContract` on entity) must be merged.

### H.4 — Prompt-section attribution in stability tests

- **Deliverable**: `prompt_stability_tests.rs` gains assertions that expose *which section* moved when a hash changes.
- **Build**:
  - add helpers `harness_section_hash()`, `quality_baseline_hash()`, `framing_hash()`
  - assert harness and quality-baseline hashes are stable across subtask transitions, mode changes, and tasks
  - assert framing hash changes only at task boundaries (not mid-task)
- **Done when**: a future PR that edits the harness section forces a deliberate hash-move; one that edits only a task's framing doesn't ripple through harness tests.

### H.5 — Prompt-section attribution in runtime telemetry

- **Deliverable**: `llm_request_prepared` trace payload carries `harnessSectionHash`, `qualityBaselineHash`, and `framingHash` separately, in addition to the existing `systemPromptHash`.
- **Build**: extend `request_shape::system_prompt_hash` with sibling helpers; emit all four in the trace.
- **Done when**: measurement runs can attribute prompt-prefix churn to either the invariant half or the per-task framing, giving clean before/after comparisons per workstream.

### H.6 — Remove descriptive language; add consequence language

- **Deliverable**: every harness-contract sentence states a runtime consequence, not a suggestion. Concrete rewrites live in `system-prompt-architecture.md`; this deliverable is the line-by-line replacement in `harness_contract_prompt()` once H.1 lands.
- **Examples** (not exhaustive — see companion doc):
  - *"Focus on completing this task"* → *"You are working on subtask N/M: <title>. Writing outputs that do not match this subtask's contract will be flagged as scope drift and corrected before the next step."*
  - *"Use at meaningful moments"* (narrate) → *"`agent_narrate` accepts `intent: question | narration`. When you send a question, the runtime pauses execution until the user replies. Use `question` when you genuinely cannot proceed. Never fabricate missing information."*
  - *"When ALL subtasks are completed, provide a summary"* → *"Advancing past a subtask requires its declared artifacts to exist on disk. If they do not, either produce them or call `agent_mark_subtask_status` with `Failed` and a reason. Do not silently advance."*
- **Tests (first)**: golden-file test on the final harness contract string. New wording lands only when the golden file is intentionally updated.

---

## Sequencing

```
A, H.1, H.4, H.5  ──────┐              (ship first; UX fix + prompt split + telemetry)
                        │
C, D, H.6  ─────────────┤              (ship in parallel; prompt/assembly changes)
                        │
B  ─────────────────────┼────────────┐ (design lift; unlocks F and H.3)
                        │            │
E, H.2  ─────────────── ┘            │ (independent; UI + lifecycle / role plug-in)
                                     │
                        F, H.3  ─────┘ (depends on B)
                        G              (optional; any time after B)
```

**Recommended order**: A + H.1 + H.4 + H.5 → C + D + H.6 parallel → B → E + H.2 parallel to B → F + H.3 → G.

The prompt-split (H.1) and telemetry (H.4, H.5) ship alongside the rehydration fix because they're preconditions for attributing any future behavior change to its cause. H.6 consequence-language rewrites land with C+D because those workstreams change *what* the prompt must describe. H.3 contract-aware appendix lands *with* B (contracts don't exist before B.1). H.2 role plug-in lands whenever the second non-code workflow is real; don't ship it speculatively.

---

## Measurement

Re-run the M4.3 characterization workflow after each workstream. Reuse the five signals from `../agent-context-engineering/measurement-harness.md`:

1. Tool catalog churn (distinct hashes per session)
2. Narration-only step rate
3. True repeat tool-call rate
4. Context token budget envelope
5. Compaction frequency and trigger mix

Add two new signals:

6. **Scope drift event count** — count of `subtask_scope_drift` runtime traces per session. Gated on workstream B. Expected: >0 before B.3, 0 after soft enforcement is respected, and hard-rejected if B.4 ships.
7. **Prompt-section hash attribution** — `harnessSectionHash` and `domainSectionHash` emitted per `llm_request_prepared`. Gated on workstream H.5. Used to verify that behavior changes attribute cleanly to either the harness or the domain half, never both at once.

Record each measurement back in `../agent-context-engineering/measurement-harness.md` under a new "Agent fidelity" section so the comparison is visible alongside the M4.3 baseline.

---

## Exit

This plan is **Done** when:

- All eight problems (P1–P8) have a shipped fix with a regression test.
- The M4.3 workflow re-run shows 0 scope-drift events, 0 stale-blocker leaks across subtask transitions, no placeholder-fabrication after an unanswered clarification, reload-preserved task titles, and the prompt-section attribution separates harness churn from domain churn.
- The agent's static prompt contains no task-type-specific language; every task has a declared framing visible to the user before execution.
- New characterization entries are recorded in `../agent-context-engineering/measurement-harness.md`.

Until then, this file is the source of truth.
