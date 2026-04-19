# Agent Fidelity — Followup Execution Plan

**Status**: IN PROGRESS
**Last reviewed**: 2026-04-19
**Driver session**: `8311a601-2827-4fef-ac91-535cd4603f0e` (forensic analysis uncovered four defects that rendered Phases 3–6 of the main agent-fidelity plan dead in production).

## Progress

- Workstream α — delivered: tool-planner merge (contracts/framing flow through)
- Workstream β — pending: dynamic step budget
- Workstream γ — pending: `fs_replace_in_file` robustness
- Workstream δ — pending: terminal-state disambiguation

---

## Problem inventory

| ID | Problem | Evidence | Fix workstream |
|----|---------|----------|----------------|
| F1 | Auto-planner at task creation produces contract-less, framing-less, FullAccess-only subtasks. `agent_plan_subtasks` early-returns "already planned" and discards the model's rich plan. Phases 3 (B), 5 (F.1), 6 (G) and scope-drift are dead code in production. | Session 8311a601: planner calls at 02:21:24 and 02:25:30 both return `{"message": "Subtasks already planned for this task. Continue with current subtask."}`; all 7 stored subtasks have `contract = NULL`. | α |
| F2 | Flat `MAX_EXECUTION_STEPS = 120` does not scale with plan size. 7-subtask MVP gets 17 steps/subtask; 15-subtask plan gets 8. | Session 8311a601 terminated with `"Execution step limit reached (120). Aborting to prevent runaway loop."` after 2/7 subtasks complete. | β |
| F3 | `fs_replace_in_file` returns a sparse error payload on miss; model cannot self-correct and loops on stale `old` snippets. | 5 replace failures on the same session, all `"Replacement text not found"`. | γ |
| F4 | Budget exhaustion raises `AgentRuntimeError::LoopDetected` and emits `"runaway loop"` wording — factually wrong; mis-classifies telemetry; flips the active subtask to Failed. | Terminal state on 8311a601 is `LoopDetected` even though no loop existed. | δ |

---

## Workstream α — Unblock the tool-planner

**Fixes**: F1.
**Ships alone**: yes.
**Risk**: medium — touches the hot path for every task.
**Blast radius**: `permissions.rs::handle_plan_subtasks`, `TaskRepository`, `planner.rs` (prompt only), `harness_contract_prompt()` (one sentence + golden).

### α.1 — Merge instead of early-return

Replace the `"Subtasks already planned"` early-return with `merge_model_plan_into_existing`:

- Iterate the model's subtasks in declared order.
- For each, find the existing subtask at the same `order`.
- If status is `Pending` or `InProgress`: attach `contract`, update `description`, `title`, `mode` if the model supplied non-empty / different values. Persist via new `TaskRepository::refine_subtask`.
- If the model declared *more* subtasks than exist: append at the tail (only allowed if no existing subtask is `Completed`).
- If the model declared *fewer*: ignore extras; never delete in-progress / pending work.
- If any existing subtask is `Completed` and the model's counterpart differs materially: return structured error with `requires_replan: true` so the model reconciles.

### α.2 — Auto-planner emits skeleton only

Remove any mention of `mode`, `contract`, or `expected_outputs` from the auto-planning LLM prompt (`build_planning_system_prompt` or equivalent). Keep it fast and minimal — titles + descriptions only.

### α.3 — Harness contract instruction

Append one sentence to `harness_contract_prompt()`:

> When a task begins, the runtime has produced a skeleton plan (titles + descriptions). Your first action must be `agent_plan_subtasks` to attach contracts (expected_artifacts) and, if applicable, a task-level framing. Skipping this step disables scope-drift detection and clean-exit verification for the remainder of the task.

Regenerates the H.6 golden file — intentional.

### α.4 — Tests

- Unit: merge attaches contract to existing subtasks by order index.
- Unit: merge refuses to mutate a Completed subtask when the model's counterpart differs.
- Unit: merge handles fewer-than-existing and more-than-existing correctly.
- Characterization: replay the `8311a601` first `agent_plan_subtasks` payload → all 7 subtasks end up with non-null `contract`.
- Regression: legacy path (no model plan call) continues to work (contracts stay None; no subtask is mutated).

### α.5 — Migration / compat

- No schema migration needed — `agent_subtasks.contract` column already exists from Phase 3.
- No frontend changes — types already carry `contract?`.

---

## Workstream β — Dynamic step budget

**Fixes**: F2.
**Ships alone**: yes, after α (α can grow the subtask count at merge time).
**Risk**: low — one constant → formula, one field on `ExecutionContext`.
**Blast radius**: `utils.rs`, `state.rs`, `bootstrap.rs`, `step_gate.rs`.

### β.1 — Formula

```rust
pub const EXECUTION_STEPS_BASE: u32 = 30;
pub const EXECUTION_STEPS_PER_SUBTASK: u32 = 15;
pub const EXECUTION_STEPS_FLOOR: u32 = 45;
pub const EXECUTION_STEPS_CEILING: u32 = 600;

#[must_use]
pub fn max_execution_steps(num_subtasks: u32) -> u32 {
    let computed = EXECUTION_STEPS_BASE
        .saturating_add(EXECUTION_STEPS_PER_SUBTASK.saturating_mul(num_subtasks));
    computed.clamp(EXECUTION_STEPS_FLOOR, EXECUTION_STEPS_CEILING)
}
```

### β.2 — `ExecutionContext.max_execution_steps`

Populated in `bootstrap.rs` once subtasks are persisted. Re-computed in the α merge path when the model adds subtasks beyond the existing count (cap grows monotonically; never shrinks mid-task).

### β.3 — Step gate uses the dynamic cap

`step_gate.rs:125` swaps `MAX_EXECUTION_STEPS` for `ctx.max_execution_steps`.

### β.4 — Tests

- Formula unit tests at the boundary values (0, 7, 40, saturating).
- Bootstrap integration: 7 subtasks → context.max_execution_steps == 135.
- Plan-growth integration: model adds 5 subtasks via α merge → cap grows.

---

## Workstream γ — `fs_replace_in_file` robustness

**Fixes**: F3.
**Ships alone**: yes.
**Risk**: low — purely additive on the error payload + one drift-detection rule.
**Blast radius**: `fs_replace_in_file` handler + loop-detection code path.

### γ.1 — Enrich error payload

On miss, return:

```json
{
  "error": "Replacement text not found in 'server/src/services/auth.service.ts'",
  "submitted_old": "<first 200 chars>",
  "nearest_match": {
    "similarity": 0.78,
    "context": "...5-line window around the best fuzzy hit...",
    "starting_line": 1
  },
  "hint": "File contents may have changed since you last read it. Call fs_read_file first and retry with the current content."
}
```

Similarity via `strsim` or line-ratio; suppress `nearest_match` below 0.5 to avoid misleading hints.

### γ.2 — Drift-aware retry nudge

If two `fs_replace_in_file` calls on the same file miss within a 5-step window, inject a system message for the next step:

> The last two replacements on `<path>` failed with "Replacement text not found". The file is almost certainly out of sync with your working model. Read it fresh with `fs_read_file` before your next edit.

Soft nudge — mirrors the B.3 scope-drift correction pattern. No hard block.

### γ.3 — Tests

- `nearest_match` present when similarity > 0.5.
- `nearest_match` absent when file diverged enough.
- Drift-aware: synthetic 2 failures on same file → correction system message appended to next context.

---

## Workstream δ — Terminal-state disambiguation

**Fixes**: F4.
**Ships alone**: yes.
**Risk**: low — new error variant + telemetry rename; preserves all existing behavior except the budget-exhausted code path.
**Blast radius**: `types.rs` (one variant), `step_gate.rs` (one branch), runtime trace type, optional Svelte string.

### δ.1 — New error variant

```rust
#[error("Step budget exhausted ({used}/{budget}) — task saved for review")]
StepBudgetExhausted { used: u32, budget: u32 },
```

### δ.2 — Step gate distinguishes

```rust
if ctx.step_number > ctx.max_execution_steps {
    // ...warn, mark needs_review, preserve subtask states, emit distinct trace type...
    return Err(AgentRuntimeError::StepBudgetExhausted { used, budget });
}
```

### δ.3 — Preserve subtask states on exhaustion

Current behavior cascades-fails the active subtask. Change: on `StepBudgetExhausted`, leave subtask status as-is. Session gets `needs_review = true` (Phase 5 infra).

### δ.4 — Runtime trace type

Add `RuntimeTraceType::StepBudgetExhausted` alongside `SubtaskScopeDrift` from Phase 3.

### δ.5 — Frontend copy

When session state is budget-exhausted, render: *"Task reached its step budget ({used}/{budget}). Work preserved — review and resume."* No new Svelte component; extend the existing status render.

### δ.6 — Tests

- Hitting cap emits `StepBudgetExhausted { used, budget }` not `LoopDetected`.
- Session `needs_review == true` after the event.
- Active subtask remains `InProgress` (not flipped to `Failed`).
- Completed and Pending subtasks untouched.

---

## Sequencing

```
α (planning bypass)  ───┐  ship first — everything else measures against α.
                        │
β (dynamic budget)  ────┤  depends on α's merge growing the subtask count.
                        │
γ (replace robustness)  │  independent, low risk.
                        │
δ (terminal state) ─────┘  cosmetic + correctness; easier before α's tests pin wording.
```

Recommended order: **α → β → γ → δ**, one commit per workstream, tests-first in each.

## Out of scope

- Env-var override for step budget.
- Resume-from-interrupted checkpointing — deferred until α+β show the cap is legitimately hit.
- Per-subtask sub-budget.
- Auto-invoking `fs_read_file` after replace miss — γ.2 is a prompt nudge only.

## Measurement additions

Append to `docs/planning/agent-context-engineering/measurement-harness.md`:

- **Signal 8 — budget headroom**: `step_budget_used / step_budget_total` per terminal session. Lets us see whether β solved the ceiling problem or merely raised it.
- **Signal 9 — contract coverage**: fraction of subtasks with `contract != NULL` per session. Gated on α; expected near-1.0 after α lands.

## Exit

This plan is **Done** when:

- All four problems (F1–F4) have a shipped fix with a regression test.
- A re-run of a comparable MVP-scale request shows: contracts persisted on every subtask, cap scales with plan size, replace-miss errors carry a `nearest_match` hint, and budget exhaustion surfaces as `StepBudgetExhausted` (not `LoopDetected`) with `needs_review = true` and preserved subtask states.
- Measurement-harness doc carries Signals 8 and 9.
