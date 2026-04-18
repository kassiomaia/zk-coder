# Agent Context Engineering — Measurement Harness

**Status**: Living document — updated alongside each representative workflow run
**Purpose**: Close the feedback loop (M4.3) so runtime improvements can be evaluated against measurable signals, not intuition
**Scope**: Frontend/backend signals that are observable today, plus the signals that each upstream milestone should add when it lands

---

## Why this exists

The execution plan (`plan.md`) is grounded in two design inputs and one feedback source:

1. **Manus — "Context Engineering for AI Agents: Lessons from Building Manus"**
   Design intent: cache stability, stable tool catalog, external memory, recitation, failure retention.
2. **Anthropic engineering — "Effective harnesses for long-running agents"**
   Design intent: initializer/worker split, session handoff, startup verification, incremental progress discipline, clean-state exits.
3. **Measured runtime behavior in this repo**
   Tie-breaker when the blogposts suggest one thing and observed signals suggest another.

M4.3 requires that every milestone be justifiable with at least one measurable signal, not only design-level arguments. This document is the shared checklist for those measurements.

---

## Signals

The plan lists five core signals. Each one is tracked here with:

- **what it measures**
- **where it comes from today**
- **how to collect it during a run**
- **what "better" looks like**

### 1. Tool catalog hash churn

- **Measures**: how often the model's visible tool catalog changes between adjacent LLM requests.
- **Why it matters**: cache stability and Manus's "don't churn the tool catalog" rule. Every change invalidates the prompt prefix cache.
- **Source today**: `llm_request_prepared` runtime trace → `payloadJson.toolCatalogHash` per step. Landed in M1.2. The companion `systemPromptHash` and `messagePrefixHash` fields cover the adjacent cache-sensitive surfaces.
- **Collection recipe**: select step-to-step `toolCatalogHash` transitions from `agent_runtime_traces` for a single task; count transitions where the hash changes and the session catalog was not actually edited.
- **Target**: 0 churn events between adjacent steps in the same subtask mode. Any non-zero churn needs a recorded reason (subtask transition, session catalog edit).

### 2. Narration-loop incidence

- **Measures**: how many times the loop detector flips the runtime into narration-loop recovery.
- **Why it matters**: narration loops are the clearest sign that the model is stalling on non-progress text.
- **Source today**: `execution_failed` runtime trace with termination reason, and `should_skip_loop_detection` / repeat-count logic in `src-tauri/src/infrastructure/agent/runtime/loop_detection.rs`.
- **Collection recipe**: count `execution_failed` traces with a narration-related termination reason, or grep runtime logs for the loop-detection path.
- **Target**: 0 incidents for a representative short workflow (<10 steps). Low double-digits for a long-running workflow (>50 steps) would be a regression, not a tradeoff.

### 3. Repeated identical tool-call rate

- **Measures**: fraction of tool calls that are identical (same name + arguments fingerprint) to a recent prior call within the same task.
- **Why it matters**: direct indicator of whether failure memory (M4.2) and the execution brief (M4.1) are helping the model avoid retrying the same wrong call.
- **Source today**: `tool_call` events and `execution_step` events. Arguments fingerprinting can be done by normalizing JSON keys.
- **Collection recipe**: gather all `tool_call` events for one session, normalize argument JSON, count duplicates within a sliding window (last 5 calls).
- **Target**: < 5% of tool calls are identical retries without an observable state change between them.

### 4. Context token budget before call

- **Measures**: size of the prepared context at each LLM request boundary.
- **Why it matters**: if the budget drifts up without proportional task growth, the working-memory or compaction policy is failing silently.
- **Source today**: `ExecutionSnapshot.totalInputTokens` and `ExecutionSnapshot.contextUsagePercent` per step, plus the current session's `state.contextUsage` during `running`. The `llm_request_prepared` trace also carries `payloadJson.estimatedContextTokens` (pre-existing) and `payloadJson.toolCount` (added in M1.2).
- **Collection recipe**: for a representative workflow, plot per-step context tokens as a series; compare to baseline (same workflow before the change).
- **Target**: monotonic non-decreasing within a single long-horizon task is acceptable; a step-to-step *spike* without a new subtask or retrieval is a regression signal.

### 5. Compaction frequency

- **Measures**: how often the runtime fires `context_compacted`.
- **Why it matters**: over-eager compaction is a cost signal; under-eager compaction trips the context limit.
- **Source today**: `context_compacted` agent events (see `src-tauri/src/infrastructure/agent/events.rs`).
- **Collection recipe**: count `context_compacted` events per run; group by reason (`token_limit` vs `subtask_transition`).
- **Target**: compaction should happen at predictable moments (subtask boundaries, explicit memory-refresh triggers). Unexplained mid-subtask compactions indicate the policy needs tightening.

---

## Manual run checklist

Use this checklist for one representative agent workflow. Run it before a change and after the change.

1. **Pick a representative workflow** that exercises multi-step execution, at least one subtask transition, and at least one tool failure. The `docs/tools/pdf/goldens/journey-a.pdf` generation is a good candidate: it is deterministic, multi-phase, and has known tool dependencies.
2. **Reset local state** so the run is reproducible: fresh conversation, fresh session, workspace known-clean.
3. **Start a fresh terminal tail** on the backend logs (or trace database) so runtime traces are captured end-to-end.
4. **Run the workflow to completion** without manual intervention; record any approvals issued.
5. **Collect the five signals** for the run:
   - count `tool_call` events per unique name+fingerprint → **signal 3**
   - count `context_compacted` events (split by reason) → **signal 5**
   - count `execution_failed` traces with narration-related reasons → **signal 2**
   - extract `ExecutionSnapshot.totalInputTokens` per step → **signal 4**
   - note tool catalog changes between steps (manual until M1.2) → **signal 1**
6. **Record** the numbers into the "Recorded measurements" section below.
7. **Compare** against the previous recorded run for the same workflow. Flag any regressions; open a follow-up item if the delta is unexplained.

---

## Recorded measurements

Each entry pins the workflow, the commit, and the five signals. Add a new entry per measured change. Do **not** rewrite prior entries — measurements are append-only so trendlines stay honest.

### Entry template

```
- **Workflow**: <short name>
- **Commit**: <short sha>
- **Date**: YYYY-MM-DD
- **Steps executed**: <n>
- **Signal 1 (tool catalog churn)**: <n churn events> — <note>
- **Signal 2 (narration loops)**: <n incidents>
- **Signal 3 (repeated tool calls)**: <n/total = n%>
- **Signal 4 (context token budget)**: <first step / max step / last step>
- **Signal 5 (compaction)**: <n events — breakdown by reason>
- **Notes**: <free-form — regressions, surprises, follow-ups>
```

### Baseline — before Slice 10

- **Workflow**: placeholder — not yet executed against a measured baseline
- **Commit**: `f81346e` (post-Slice 9)
- **Date**: 2026-04-17
- **Steps executed**: pending
- **Signal 1 (tool catalog churn)**: pending — telemetry landed in M1.2, first representative-workflow measurement still to run
- **Signal 2 (narration loops)**: pending
- **Signal 3 (repeated tool calls)**: pending
- **Signal 4 (context token budget)**: pending
- **Signal 5 (compaction)**: pending
- **Notes**: Captured before a representative workflow has been rerun under the Slice 10 event-handling changes. Slice 10 itself does not change backend execution behavior — it only re-homes harness event mapping and adds this measurement contract — so the frontend side of the workflow should show identical runtime signals pre/post Slice 10. M1.2 (commit after `0d9a654`) wires `toolCatalogHash`, `systemPromptHash`, `messagePrefixHash`, and `toolCount` into the `llm_request_prepared` trace, unblocking signal 1's automatic collection path.

### Post-Slice 10 smoke measurement

- **Workflow**: panel-backed recovery smoke (Svelte tests standing in for a full agent run; no backend runtime was invoked)
- **Commit**: pending — to be filled in on the Slice 10 commit
- **Date**: 2026-04-17
- **Steps executed**: 108 frontend unit tests across `agentStore`, `harnessEvents`, `sessionEvents`, `threadAgentState`, `AgentPanel`
- **Signal 1 (tool catalog churn)**: not applicable (no backend run)
- **Signal 2 (narration loops)**: not applicable
- **Signal 3 (repeated tool calls)**: not applicable
- **Signal 4 (context token budget)**: not applicable
- **Signal 5 (compaction)**: not applicable
- **Notes**: Slice 10 deliberately scopes to (a) event-handler refactor and (b) this measurement contract. The signal columns are "not applicable" on purpose — a measurable-change slice should record meaningful numbers, and this slice does not change the backend runtime. The next slice that does change runtime behavior (M1.2, M4.1, M4.2) must populate the template with real data.

---

## Runtime transitions allowed to change request shape

M1.3 audited prompt assembly for avoidable dynamic fields (timestamps, UUIDs,
env-derived strings leaking into the system prompt or context packing). The
audit found none — `derive_system_prompt` is a pure function over
`(agent_system_prompt, Option<(SubtaskSummary, total)>)`, the base agent system
prompt is a `const fn` returning `&'static str`, and the ContextPolicyV2
blocks are deterministic given the same `ContextPolicyInput`.

Given that baseline, the following — and only the following — runtime
transitions are allowed to change each hash. Any other drift is a regression
signal and should be recorded under **Notes** in the entry that surfaces it.

### `system_prompt_hash`

Allowed causes of drift:

- **Subtask transition** — `derive_system_prompt` inlines the current subtask
  title, mode instructions, and order. Any change to `SubtaskSummary` produced
  by `context_phase::resolve_context` changes this hash by design.
- **Subtask status change** — moving from `InProgress` to a terminal status
  reverts the prompt to the bare `agent_system_prompt`, which is a different
  hash from the in-progress variant.

Not allowed:

- Same subtask state producing different hashes across adjacent steps. Every
  input to `derive_system_prompt` is either static (`agent_system_prompt`
  from `registry.rs`) or derived from the current subtask record.

### `tool_catalog_hash`

Allowed causes of drift:

- **Session-visible catalog change** — if a tool is added to or removed from
  `ToolRegistry` between steps, the catalog hash changes. This is rare in
  practice since registry construction happens once per runtime.

Not allowed (as of M2.2):

- **Subtask mode change** — after M2.2, `context_phase::resolve_context`
  always sends the full session catalog. Permission filtering moved to
  runtime enforcement in `tool_phase` via `ctx.current_allowed_tool_names`.
  A subtask mode transition therefore must *not* change the visible catalog
  hash. Any drift on a mode transition is a regression signal.
- Same subtask mode producing different filtered catalogs across steps. M1.1
  guarantees canonical ordering, so identical allowed-tool sets yield
  identical hashes.
- Adding a tool to or removing a tool from the visible catalog without a
  corresponding session-level catalog change.

Runtime enforcement observability:

- When the LLM calls a tool that is outside the current subtask mode's
  allowed set, `tool_phase::run_tools` emits a `ToolBatchStarted` trace whose
  payload includes `disallowedByMode` (the rejected tool names) and
  `allowedToolNames`. The synthesized tool result carries
  `reason: "subtask_mode_disallowed"` and the allowed-tools list so the model
  can recover by choosing a permitted tool or advancing the subtask.

### `message_prefix_hash`

Allowed causes of drift:

- **Context compaction** — `apply_context_policy_v2` and `maybe_compact_context`
  explicitly rewrite the prefix (task summary, active subtask, file relevance,
  failure memory, pre-computed summary, recent dialogue tail). Compaction is
  the single most common legitimate prefix mutation. Each compaction emits a
  `ContextPolicyEvaluated` trace so the hash change is traceable.
- **Retrieval pass** — when `usage_ratio >= threshold` or the subtask changed,
  `run_retrieval_pass` adds fresh snippets to the packed context via the
  file-relevance block. This is gated behavior, not every-step drift.
- **Subtask transition packing** — `force_pack = subtask_changed` rewrites the
  packed blocks on subtask transitions, which changes the prefix.
- **Session memory update** (M3.2 / M3.3) — when `SessionMemory.updated_at`
  moves or any knowledge-item count (findings / questions / facts)
  changes, the `ContextPolicyV2 session memory reference:` block in the
  packed context updates, which changes the prefix hash. Every mutation
  is emitted as a `MemoryRefreshed` runtime trace (see the M3.3 block
  below), so a prefix-hash shift here should always correlate with a
  trace. The block format is a pure function of `MemoryReference`, so
  an unchanged `SessionMemory` yields identical bytes across packing
  passes.
- **Execution brief update** (M4.1) — the `ContextPolicyV2 execution
  brief:` block is a pure function of
  `(task_goal, active_subtask, recently_completed, next_intended_action,
  current_blocker)`. Any field movement — a subtask transition, a new
  completed subtask, a new pending next action, a new or cleared
  blocker — rewrites the block and therefore moves the prefix hash.
  Identical inputs produce byte-identical blocks across packing
  passes. Unrelated dialogue appended at the tail must not move the
  hash.
- **Failure-memory mutation** (M4.2) — the
  `ContextPolicyV2 failure memory:` block changes whenever the most
  recent error's `(tool_name, args_fingerprint, class, message)`
  tuple changes, or when a new identical repeat is detected and the
  "do not retry unchanged" guidance appears. No error present → no
  block → identical bytes to the previous step. Each call to
  `extract_failure_record` is deterministic over the same context, so
  re-packing the same state yields the same block.

Not allowed:

- Identical session state producing different prefix hashes across adjacent
  steps outside of the triggers above. Appending a single new message should
  leave the prefix hash unchanged — that is the cache-warmth invariant the
  hash is built to observe.

Working-memory observability (M3.1 / M3.2):

- When a task is non-trivial (planner flagged `is_non_trivial && subtasks
  non-empty`), `start_execution` creates a row in `agent_session_memory`
  keyed by `session_id` with the task goal and a joined subtask-title
  active plan. Trivial tasks never create a memory row, so
  `deps.memory_repo.get_memory(...) == None` is the expected state there.
- Every `resolve_context` call fetches the memory and — when present —
  inlines a `ContextPolicyV2 session memory reference:` system block
  between the pre-computed summary and the recent dialogue tail. The
  block is capped at `BUDGET_MEMORY_REFERENCE_TOKENS` (180) and carries
  only the session id, timestamp, knowledge-item counts, and truncated
  goal/plan text — never the raw finding/question/fact lists.
- Compaction cannot silently drop the reference: the tiny-context
  forced-pack short-circuit in `apply_context_policy_v2` explicitly
  bypasses when `input.memory_reference.is_some()`, so the block is
  always rebuilt during compaction when memory exists.

Memory refresh triggers (M3.3):

- `application::agent::execution::memory_refresh::apply_memory_refresh`
  is the **only** site outside of `start_execution` that writes
  `SessionMemory`. It is invoked from `context_phase::resolve_context`
  by three bounded, deterministic triggers:
  - **SubtaskCompleted** — fires when `resolve_context` observes a
    subtask id transition (`previous_subtask_id != current_subtask_id`)
    and the previous row is now `Completed` on disk. Appends the
    subtask title to `completed_findings` (idempotent via dedup) and
    refreshes `active_plan` to the new in-progress subtask title.
    Guarded by `ctx.last_completed_subtask_for_memory`, so the same
    completion never fires the trigger twice.
  - **CompactionBoundary** — fires only when a compaction pass returned
    `ContextPackOutcome::Compacted`. Mutation is limited to
    `active_plan` refresh; no findings/facts are added. The trigger's
    purpose is to audit that compaction happened and that the memory
    reference block is the primary state-carrier at that boundary.
  - **RetrievalBurst** — fires when a per-subtask counter
    (`ctx.retrieval_count_in_current_subtask`) crosses
    `RETRIEVAL_BURST_THRESHOLD = 3`. The counter resets on every
    subtask change, so bursts are scoped to a single subtask. Mutation
    appends deduplicated file paths distilled from retrieval snippets
    into `extracted_facts`; counter resets after firing.
- Retention: `MAX_COMPLETED_FINDINGS = 50` and
  `MAX_EXTRACTED_FACTS = 50` cap each list; oldest entries evict from
  the front. A `RetrievalBurst` firing that would exceed the cap is
  still recorded in the trace with the real `factsAdded` count; the
  memory body just sheds older entries.
- Every firing — including no-op cases — emits a `MemoryRefreshed`
  runtime trace with payload shape:
  ```json
  {
    "trigger": "subtask_completed" | "compaction_boundary" | "retrieval_burst",
    "wrote": true | false,
    "findingsAdded": <uint>,
    "factsAdded": <uint>,
    "activePlanChanged": true | false,
    "skippedReason": null | "no_memory" | "no_change" | "load_failed" | "write_failed",
    "triggerDetails": { ... trigger-specific }
  }
  ```
  Trace consumers can count firings, pivot by `trigger`, and detect
  "trigger fired but no state change" via `wrote: false` +
  `skippedReason: "no_change"`. A trigger whose `skippedReason` is
  `"no_memory"` indicates the task was trivial (never got a memory row
  at bootstrap) and is the expected state for single-step requests.
- Non-trigger steps do not write memory and do not emit
  `MemoryRefreshed`. This is the key M3.3 invariant: memory updates
  are bounded, deterministic, and observable, not every-step
  rewrites.

Execution brief block (M4.1):

- `apply_context_policy_v2` inlines a single
  `ContextPolicyV2 execution brief:` system block as the last
  synthetic block — between the session memory reference and the
  recent-dialogue tail — so the "what matters now" snapshot sits as
  close to the attention tail as possible without interleaving with
  live dialogue.
- Block body shape (missing fields omitted; fields individually
  truncated to `BUDGET_EXECUTION_BRIEF_TOKENS / 4`):
  ```
  ContextPolicyV2 execution brief:
  goal: <task goal>
  active: <current subtask title>
  recent: <most-recent-completed titles; "; " separated>
  next: <next pending subtask title>
  blocker: <class>: <failing tool>
  ```
- `context_phase::build_execution_brief` derives the brief from
  `(request.content, current_subtask_summary, loaded_subtasks,
  llm_context)`. It is a pure function of durable inputs, so two
  packing passes on identical state produce byte-identical blocks.
  Unrelated dialogue appended at the tail must not move the brief.
- `blocker` is only present when the context contains an unrecovered
  tool error — otherwise the line is absent, not an empty string.
  `current_blocker` delegates classification to
  `FailureClass::classify` (see M4.2 below) so the brief and the
  failure-memory block agree on the normalized class.
- Tiny-forced-pack exception: when an execution brief is supplied,
  the tiny-context short-circuit is bypassed — mirrors the memory
  reference behavior — so subtask transitions never suppress the
  brief on small contexts.

Enriched failure memory (M4.2):

- `build_failure_memory` emits a typed, multiline block instead of
  the free-form "Last actionable error: ..." string. Body shape,
  capped at `BUDGET_FAILURE_MEMORY_TOKENS = 140`:
  ```
  ContextPolicyV2 failure memory:
  Last error:
  tool: <failing tool name>
  args_fp: <8-hex-char canonical JSON fingerprint>
  type: <file_not_found | permission_denied | timeout | validation_failed | not_allowed | other>
  message: <truncated error text, <= 160 chars>
  retry: safe to retry (likely transient)  # for timeout | other
  retry: change args before retry          # for the rest
  # Optional last line — only when a prior (tool, args_fp) pair also failed:
  note: identical call has failed before — do not retry unchanged
  ```
- Classifier (`FailureClass::classify`) uses bounded
  case-insensitive substring matching on the error message. Unknown
  shapes degrade to `other` and are treated as safe-to-retry by
  default. Add new classes deliberately — the label set is the
  public contract the signal-3 (repeated identical tool-call rate)
  analyzer groups on.
- Fingerprint (`arguments_fingerprint`) is an 8-hex-char FNV-1a
  hash of the canonicalized JSON arguments. It is stable across
  processes and equivalent-object permutations, so signal-3
  dedup-by-fingerprint works across a whole trace history.
- Repeat detection walks the full error history in the visible
  context, not a sliding window. A previously-error'd call with the
  same `(tool_name, args_fingerprint)` anywhere in context flips
  `repeat_detected = true`, which emits the anti-repeat `note:` line.
- Consumers interested in the structured breakdown without the
  packed-context detour can call `extract_failure_record(&context)`
  directly — it returns `Option<FailureRecord>` over the same
  inputs and is what `build_execution_brief` uses to populate
  `ExecutionBrief.current_blocker`.

## Measurement rules

- **Append, do not rewrite**. An entry is a timestamped measurement. Keep the history intact so a later reader can see drift.
- **Prefer automation**. Where a signal can be pulled from the trace database by a small script, write the script and check it in. Manual-only signals are acceptable until a better source exists, but they should carry a note.
- **Record "no-op" changes explicitly**. A slice that is expected to not move signals (like this one) must still be logged with "not applicable" reasoning — so future reviewers can tell whether a signal was skipped or simply unchanged.
- **Call out blockers on upstream telemetry**. Signal 1 and part of signal 4 depend on M1.2 landing. Do not silently drop them; mark them as blocked.

---

## Tie to design inputs

- Signals 1 and 4 track **cache stability** (Manus) and **incremental progress discipline** (Anthropic): the request shape stays close to stable across steps except when a real state change happens.
- Signals 2 and 3 track **error retention / failure memory** (Manus) and the Anthropic notion of **not regressing on the same wrong move**.
- Signal 5 tracks **external memory boundary behavior** (Manus) — compaction is only acceptable at predictable points, not as a silent belt-and-suspenders fix.
- The manual-run checklist mirrors the Anthropic long-running harness idea of **treat a real run as the source of truth** rather than synthetic microbenchmarks.

When a measurement disagrees with the blogpost advice, the measurement wins — and the disagreement is recorded under **Notes** in that entry.
