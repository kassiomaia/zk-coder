# Final-Summary Tool-Call Discard — Execution Plan

**Status**: COMPLETE — 2026-04-19
**Last reviewed**: 2026-04-19
**Issue**: `docs/issues/agent-final-summary-tool-call-discard/issue.md`
**Severity**: high
**Scope**: backend only (`src-tauri/src/application/agent/execution/`)

## Outcome

All three phases shipped in one PR. Build and `cargo test --lib` are
green (1928 / 1929; the one failing test is a pre-existing
unrelated failure on `master`). Zero new clippy warnings vs. the
master baseline. See the **Resolution** section in the issue
document for a full file-by-file changelog.

## Purpose

Fix the `_ => continue` arm at
`src-tauri/src/application/agent/execution/coordinator.rs:152` that
silently discards an LLM response containing tool calls when the
final-summary branch fires, and harden the branch so it cannot
duplicate the `"All subtasks have been completed..."` user prompt
across iterations.

## Background (one paragraph)

When every subtask reaches a terminal state, `coordinator.rs` enters
a dedicated branch that pushes a `user`-role "summarize" prompt,
empties `current_allowed_tool_names`, sets
`pending_tool_choice_override = ToolChoiceMode::None`, calls the LLM,
and matches on `handle_response`. The match arm
`_ => continue` accepts both `ContinueWithNextSubtask` and
`ExecuteTools(response)` — but `ExecuteTools` is the LLM's
*assistant + tool-calls* response. Skipping `run_tools` means the
assistant turn is never appended to `ctx.llm_context`, no
`subtask_mode_disallowed` correction is sent back, and the next
iteration re-enters the same branch and pushes another
identical `user` prompt — producing two consecutive `user` messages
that Anthropic rejects with HTTP 400 and that OpenAI accepts but
loops on until `StepBudgetExhausted` fires.

## Goals

1. **No silent loss.** A non-`Finished` response on the final-summary
   path must be fully processed: the assistant message lands in
   `ctx.llm_context`, every tool call is rejected with the existing
   `subtask_mode_disallowed` error result so the model can self-
   correct, and the LLM context stays well-formed (alternating
   user/assistant/tool roles).
2. **No duplicate-prompt accumulation.** The
   `"All subtasks have been completed..."` user message is pushed at
   most once across consecutive final-summary iterations.
3. **Loop detection cannot mask repeated narration during
   finalization.** If the LLM keeps emitting `agent_narrate` on the
   final-summary branch, the runtime must observe that as a loop
   rather than skipping it (the existing `LOOP_SKIP_TOOLS` whitelist
   only makes sense during normal execution).
4. **All existing characterization tests remain green.** The fix is
   additive at the `coordinator.rs` match-arm level and at the
   prompt-push site — no public API changes, no IPC contract
   changes, no frontend changes.

## Non-goals

- Introducing a new `AgentRuntimeError::FinalSummaryToolCallViolation`
  variant (Option B in the issue). Option A (route through
  `run_tools`) plus the duplicate-push guard (Option C.1) is
  sufficient and lower-risk.
- Re-architecting how `all_subtasks_completed` is recomputed each
  iteration. The flag is correctly derived from DB state; the bug
  is in how the branch reacts when the LLM does not comply.
- Rewriting the narration-loop guard. The existing
  `handle_narration_loop_escalation` only triggers on empty-tool
  responses; the final-summary branch produces tool-bearing
  responses that fall under loop detection instead.
- Touching the agent-clarification park path
  (`maybe_park_for_clarification`). It is not relevant to the
  final-summary defect.

## Architectural problem being addressed

A defense-in-depth comment at `coordinator.rs:122-127` claims three
overlapping safety layers (prompt, provider hint, runtime allow-list
filter). In production only layers 1 and 2 fire on this branch —
layer 3 lives inside `tool_phase::run_tools`, which the branch
deliberately bypasses with `_ => continue`. The fix collapses the
asymmetry: the final-summary branch must use the *same*
`ResponseDecision` dispatch as the main loop so all three layers
take effect uniformly.

## Target design

### Coordinator dispatch (Option A)

The final-summary branch matches on `ResponseDecision` exhaustively
and routes `ExecuteTools` through the same `run_tools` call the
main loop uses. After `run_tools` returns `Continue`, the loop
restarts at `begin_step`. The next iteration re-enters the
final-summary branch (because the DB state is unchanged), sees
that the assistant + tool-result messages are now well-formed in
`ctx.llm_context`, and either gets a clean text response from the
LLM (because of the `subtask_mode_disallowed` correction) or makes
one more bounded attempt that the loop-detection guard now catches
(since `agent_narrate` is no longer skipped during finalization).

### Duplicate-prompt guard (Option C.1)

Before pushing the
`"All subtasks have been completed..."` user message, the branch
checks the tail of `ctx.llm_context`. If the *most recent* `User`-
role text message is byte-equal to the canonical prompt string,
skip the push. This makes the branch idempotent across consecutive
firings — the prompt is appended exactly once until it is
intervening assistant/tool messages move it off the tail.

### Narrowed loop-skip allowlist (Option C.2)

`detect_repetitive_loop` consults the `LOOP_SKIP_TOOLS` constant to
decide whether to count repeats. The whitelist exists so that
routine narration during normal execution does not falsely trip
loop detection. During the final-summary branch, however, *any*
tool call — including narration — is by contract not desired. We
add a single function `should_skip_loop_detection_in_state` that
takes the call name **and** the current allow-list state, and
returns `false` when the allow-list is `Some(empty)` so that the
existing repetitive-loop detector catches a model that keeps
re-emitting the same narration call across iterations.

The default `should_skip_loop_detection(name)` continues to return
the legacy answer for the rest of the codebase. Only
`response_phase::handle_response` is updated to consult the new
state-aware variant by passing `ctx.current_allowed_tool_names`.

## Modules and files expected to change

| File | Change |
|------|--------|
| `src-tauri/src/application/agent/execution/coordinator.rs` | Replace the `_ => continue` arm with an exhaustive match that delegates `ExecuteTools` through `run_tools`; gate the prompt push on a duplicate-tail check |
| `src-tauri/src/infrastructure/agent/runtime/loop_detection.rs` | Add `should_skip_loop_detection_in_state(name, allowed_tool_names)` that returns `false` when the allow-list is `Some(empty)` even for whitelisted tools |
| `src-tauri/src/application/agent/execution/response_phase.rs` | Switch the `detect_repetitive_loop` call site to consult the state-aware variant via `ctx.current_allowed_tool_names` |
| `src-tauri/src/application/agent/execution/coordinator.rs` (tests) | Add a unit test that asserts the duplicate-push guard skips the push when the tail already carries the prompt |
| `src-tauri/src/infrastructure/agent/runtime/loop_detection.rs` (tests) | Add tests for the state-aware variant |
| `src-tauri/src/application/agent/execution/runtime_enforcement_tests.rs` | Add a regression test pinning the M2.2 contract on the final-summary branch (empty allow-list rejects every tool call) — or a similarly placed integration-flavored unit test |
| `docs/issues/agent-final-summary-tool-call-discard/issue.md` | Add a "Resolution" footer noting the fix landed and pointing at this plan |

> Note: the duplicate-push guard lives **inside the if-branch in
> `coordinator.rs`**, not in `context_phase.rs`, because the prompt
> push is a coordinator-level decision. Lifting it deeper would
> require reshaping `ResolvedContext` for one read-only check.

## Sequencing

The work is one PR with three commits, each independently revertable.

### Phase 1 — Routing fix (Option A)

1. Add a unit test that constructs an `ExecutionContext` with
   `all_subtasks_completed = true`, calls a mocked
   `handle_response` returning `ResponseDecision::ExecuteTools(...)`,
   and asserts the assistant turn lands in `ctx.llm_context` after
   the iteration.
   - The test will not exercise the live coordinator (which needs a
     full Tauri `AppHandle` plus repositories). Instead, build it as
     a focused harness mirroring the dispatch arm — the same shape
     the existing `runtime_enforcement_tests` use.
2. Replace the match in `coordinator.rs:150-153` with the
   exhaustive form. Reuse the `permission_resolver` and
   `yolo_mode_provider` already constructed at the top of `execute`.
3. Run `cargo test -p coder` and confirm all existing tests pass.

### Phase 2 — Duplicate-push guard (Option C.1)

1. Add a unit test (same module) that calls the helper twice in a
   row on a context whose tail is the exact prompt and asserts the
   second call does not append.
2. Extract the prompt string to a `const FINAL_SUMMARY_PROMPT: &str`
   at the top of `coordinator.rs` so the test and the production
   call share one source of truth.
3. Wrap the `ctx.llm_context.push(...)` call in a guard that checks
   `ctx.llm_context.last()` for a `User`-role message whose
   `as_text() == Some(FINAL_SUMMARY_PROMPT)`.

### Phase 3 — Narrowed loop-skip during finalization (Option C.2)

1. Add tests in `loop_detection.rs` for the new state-aware
   variant — both the whitelist-honored case (allow-list is `None`
   or non-empty) and the whitelist-suppressed case (allow-list is
   `Some(empty)`).
2. Add `should_skip_loop_detection_in_state(name, allowed)` that
   returns `false` whenever `allowed == Some(&[])`, otherwise
   delegates to `should_skip_loop_detection(name)`.
3. Update `detect_repetitive_loop_in_state(call, recent, max,
   allowed)` (a thin wrapper) and switch
   `response_phase::handle_response` to use it.
4. Verify the existing `test_detect_repetitive_loop_skips_metadata_tools`
   continues to pass (it uses `None` for the allow-list).

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Routing the response through `run_tools` changes telemetry — extra `ToolBatchStarted`/`ToolBatchCompleted` traces appear on the final-summary path | High (intended) | Document in this plan; the new traces correctly reflect a (rejected) tool batch |
| The model emits `agent_narrate` three times in a row → narrowed loop-skip now triggers `LoopDetected` and fails the session instead of recovering | Low–medium | This is desirable: under the original bug the same scenario silently burned the entire step budget. Telemetry now distinguishes the two outcomes (`loop_detected` reason vs. `step_budget_exhausted`). The user-visible message goes through `fail_execution` which already produces a recoverable error variant. |
| Duplicate-push guard misclassifies a legitimate sequence (user typed the exact prompt themselves) | Effectively zero | The literal prompt is generated by the runtime; it is not user input. Even if a user typed it verbatim, the only consequence of skipping a duplicate push is that the LLM does not see it twice — strictly better. |
| `permission_resolver` is constructed per-`execute` call; extracting it into the final-summary branch could be misread as a refactor | Low | No extraction needed — the existing local binding is in scope where the new arm lives. |
| Tests use a mocked `handle_response` that drifts from production semantics | Low | Add an end-to-end style assertion at the unit level: after constructing the response with one tool call, after the dispatch arm runs, the last messages of `ctx.llm_context` should be `Assistant(with tool calls)` followed by `Tool(result)` — matching production behavior |

## Validation criteria

A PR is ready to merge only when:

- `cargo build` is clean.
- `cargo test -p coder` is green, including:
  - the new duplicate-push guard test
  - the new loop-skip-in-state tests
  - the existing `runtime_enforcement_tests::empty_allow_list_disallows_every_tool_call`
  - the existing narration-loop tests
- `cargo clippy` produces no new warnings.
- Manual reading of `coordinator.rs` confirms there is no
  `_ => continue` left on the final-summary branch.
- The "Resolution" section in
  `docs/issues/agent-final-summary-tool-call-discard/issue.md` is
  filled in with a one-paragraph summary and a pointer to the
  commit hash.

## Adjacent concerns explicitly NOT in scope

These were surfaced by the issue investigation pass and are
deliberately deferred:

- **Checkpoint resume with malformed `llm_context`.** Once this
  fix lands, new sessions cannot reach the malformed state. Any
  *pre-existing* checkpoint already in that state will still hit
  Anthropic 400 on resume; a separate one-shot repair migration
  would be needed for those, tracked under a new issue if/when a
  session is observed.
- **Surfacing `disallowedByMode` to the session panel.** A nice-
  to-have for ops visibility but orthogonal to correctness.
- **Enforcing `context_usage_percent` graceful degradation.** A
  pre-existing latent risk independent of this bug.

## Test/update strategy summary

| Layer | Test |
|-------|------|
| `coordinator.rs` (unit, mocked dispatch) | Asserts that on `ExecuteTools` the assistant turn is appended and the tool-result is appended; asserts the `_ => continue` regression cannot return |
| `coordinator.rs` (unit, prompt guard) | Asserts the guard skips a second push when the tail already has the prompt; asserts a fresh push happens when the tail does not |
| `loop_detection.rs` (unit) | Asserts the state-aware variant honors the allow-list-empty signal even for `agent_narrate` |
| `runtime_enforcement_tests.rs` (existing) | All assertions remain green; the empty-allow-list contract is unchanged |
| `response_phase.rs` (existing) | Narration-loop classifier tests unchanged; the new state-aware loop check fires only when an allow-list is `Some(empty)` |

## Out-of-band: telemetry expectations after the fix

A correctly behaved session triggers exactly **one**
`all_subtasks_completed_final_summary` trace before
`ResponseDecision::Finished` returns. A session that hits the new
recovery path triggers at most **two** such traces (the second
representing the model's recovery turn after seeing the
`subtask_mode_disallowed` correction). More than two indicates a
genuinely non-compliant model, and the loop-detection guard now
fires; the `step_budget_exhausted` outcome should no longer be
reachable via this code path.

## Definition of done

- All three phases implemented in one PR.
- All listed tests added and green.
- Issue document updated to `Status: resolved` with commit hash.
- Plan document updated to `Status: COMPLETE` with date.
