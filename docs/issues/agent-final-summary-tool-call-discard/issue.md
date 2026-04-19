# Final-summary path silently discards LLM responses when tool calls slip through `ToolChoiceMode::None`

- **Status**: investigated — awaiting team review
- **Reported**: 2026-04-19
- **Area**:
  - `src-tauri/src/application/agent/execution/coordinator.rs`
  - `src-tauri/src/application/agent/execution/response_phase.rs`
  - `src-tauri/src/application/agent/execution/tool_phase.rs`
  - `src-tauri/src/application/agent/execution/context_phase.rs`
- **Severity**: high — the "all subtasks completed" finalization branch can
  produce a malformed `llm_context` (two consecutive `user` messages with no
  intervening `assistant` turn), which either aborts the next LLM call on
  providers that require role alternation (Anthropic) or drives a duplicate-
  prompt loop that only terminates when the step budget is exhausted.

## Summary

When every subtask on the active task is in a terminal state
(`Completed` / `Failed` / `Cancelled`), the coordinator enters a dedicated
"final-summary" branch that (a) appends a `"All subtasks have been
completed..."` user message to `llm_context`, (b) sets
`current_allowed_tool_names = Some(Vec::new())`, and (c) overrides the next
LLM call's `tool_choice` to `ToolChoiceMode::None`. The intent, as stated in
the inline comment at `coordinator.rs:122-127`, is a three-layer defense:
the prompt, the provider hint, and a runtime rejection for "any call that
slips through".

The third layer **does not run** in this branch. The `match` arm at
`coordinator.rs:150-153` treats any non-`Finished` response as `_ => continue`,
which jumps back to the top of the loop **before** `run_tools` can execute.
Because the push of `Message::assistant_with_tool_calls` into `llm_context`
lives inside `tool_phase::run_tools` (`tool_phase.rs:67-70`), an LLM response
that contains tool calls in this branch is:

1. **Not stored** in `llm_context` — the assistant turn is lost.
2. **Never rejected** with an error result — the "empty allow-list" guardrail
   in `permissions.rs:203-219` is unreachable here.
3. **Never surfaced to the user** — the clarification park
   (`maybe_park_for_clarification`) is also skipped by the `continue`.

The next iteration re-enters the same branch (because
`all_subtasks_completed` is recomputed from the DB each step and the subtask
statuses have not changed) and appends *another* copy of the "All subtasks
have been completed..." user message. The resulting message sequence in
`ctx.llm_context` is:

```
... prior history ...
User   : "All subtasks have been completed..."        ← iteration N
[assistant response from iteration N was DISCARDED]
User   : "All subtasks have been completed..."        ← iteration N+1
```

Two consecutive `user` messages with no `assistant` turn between them.

## Evidence

### 1. The branch that discards the response

`src-tauri/src/application/agent/execution/coordinator.rs:128-154`

```rust
if resolved.all_subtasks_completed {
    log::info!(
        "[AgentExecution] All subtasks completed for task {} – requesting finish summary",
        ctx.task_id
    );
    ctx.llm_context.push(Message::user(
        "All subtasks have been completed. Provide a concise summary of what was accomplished, key findings, and any important results.",
    ));
    ctx.current_allowed_tool_names = Some(Vec::new());
    ctx.pending_tool_choice_override = Some(ToolChoiceMode::None);
    let _ = deps.tracer().append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::LlmResponseClassified,
        serde_json::json!({
            "forcedToolChoiceMode": "none",
            "reason": "all_subtasks_completed_final_summary",
        }),
    );
    let llm_result = call_llm(&deps, &mut ctx, &resolved, client.as_ref()).await?;
    match handle_response(&deps, &mut ctx, session, llm_result).await? {
        ResponseDecision::Finished(result) => return Ok(result),
        _ => continue,                                     // ← discards response
    }
}
```

The `_ => continue` arm matches both `ResponseDecision::ExecuteTools(...)`
and `ResponseDecision::ContinueWithNextSubtask`. In either case, the LLM's
response is dropped without further processing.

### 2. The push of the assistant message lives in `tool_phase`, not `handle_response`

`src-tauri/src/application/agent/execution/response_phase.rs:148-229` —
`handle_response` only pushes to `llm_context` on the empty-tool-calls path
(via `complete_execution`). When `response.tool_calls` is non-empty it
returns `ResponseDecision::ExecuteTools(response)` **without mutating
`llm_context`**.

`src-tauri/src/application/agent/execution/tool_phase.rs:66-70`

```rust
add_to_recent_calls(&response.tool_calls, &mut ctx.recent_tool_calls, 10);
ctx.llm_context.push(Message::assistant_with_tool_calls(
    response.content,
    response.tool_calls.clone(),
));
```

This is the **only** place the assistant-with-tool-calls message is
appended. Skipping `tool_phase::run_tools` means skipping this push.

### 3. `all_subtasks_completed` is recomputed each iteration

`src-tauri/src/application/agent/execution/context_phase.rs:86-121`

```rust
} else {
    let all_done = loaded_subtasks.iter().all(|st| {
        matches!(
            st.status,
            crate::core::agent::task::TaskStatus::Completed
                | crate::core::agent::task::TaskStatus::Failed
                | crate::core::agent::task::TaskStatus::Cancelled
        )
    });
    (None, None, None, all_done)
}
```

The flag is derived from the *current* DB state of the subtasks. Nothing in
the final-summary branch modifies subtask status, so once `all_done` becomes
`true` it stays `true` on every subsequent iteration, and the coordinator
re-enters the same branch each time.

### 4. The "defense in depth" comment documents intent that does not hold

`src-tauri/src/application/agent/execution/coordinator.rs:122-127`

```
// All subtasks are in a terminal state — ask the LLM for a finish
// summary. Keep the session-visible tool catalog stable (M2.2) but
// force `ToolChoiceMode::None` for this step (M2.3) so the model is
// explicitly told not to call any tool. Also set the runtime-level
// allow-list to empty so any call that slips through is rejected as
// disallowed by mode.
```

The third clause ("any call that slips through is rejected as disallowed by
mode") refers to the filter at `tool_phase.rs:79-90` and the error-result
builder at `permissions.rs:203-219`. Both live inside `run_tools`, which the
branch never calls.

### 5. The malformed-sequence outcome

Because `ctx.llm_context` is passed to `call_llm` by `.clone()`
(`llm_phase.rs:69`), and because `call_llm` attaches the full cloned history
to every `ChatRequest`, iteration N+1 sends a request whose `messages` array
ends with two back-to-back `role: "user"` entries. Anthropic's Messages API
rejects this with `400 Bad Request` ("messages: roles must alternate between
user and assistant"). OpenAI's Chat Completions API accepts it but then
almost always re-emits a similar narration tool call, perpetuating the
cycle until `begin_step` trips `max_execution_steps` and returns
`AgentRuntimeError::StepBudgetExhausted`.

### 6. Correlating a suspected incident in telemetry

If this bug fires in production, the runtime traces will carry a
signature that is easy to grep for. Suggested queries (run
`sqlite3 -readonly` against `coder_agents.db`):

```sql
-- Sessions where the final-summary branch fired more than once for the same task.
SELECT session_id, task_id, COUNT(*) AS firings
FROM agent_runtime_traces
WHERE trace_type = 'llm_response_classified'
  AND payload LIKE '%all_subtasks_completed_final_summary%'
GROUP BY session_id, task_id
HAVING firings > 1
ORDER BY firings DESC;

-- For one offending session, show the step-by-step trace around the repeats.
SELECT created_at, step_number, trace_type, substr(payload, 1, 300)
FROM agent_runtime_traces
WHERE session_id = :offending_session_id
ORDER BY created_at;

-- Did the session end with a step-budget exhaustion?
SELECT state, updated_at
FROM agent_sessions
WHERE id = :offending_session_id;
```

A session with more than one `all_subtasks_completed_final_summary` trace is
proof of at least one discarded response, because in a well-behaved run the
branch fires exactly once before returning `ResponseDecision::Finished`.

## Root cause

The `_ => continue` on `coordinator.rs:152` is the precise defect. It was
almost certainly added as a conservative fallback in case `handle_response`
ever returned a non-`Finished` variant from the final-summary path, but in
practice `handle_response` **does** return `ExecuteTools` whenever the LLM
emits any tool call, including the agent-metadata tools (`agent_narrate`,
`agent_emit_status`, `agent_mark_subtask_status`,
`agent_advance_to_next_subtask`, `agent_plan_subtasks`) that are explicitly
exempt from repetitive-loop detection (`loop_detection.rs:16-22`).

The branch therefore has three overlapping safety layers on paper:

| Layer | Mechanism | Status |
|-------|-----------|--------|
| 1. Prompt | "Provide a concise summary..." user message | fires |
| 2. Provider hint | `ToolChoiceMode::None` forwarded to the client | fires |
| 3. Runtime rejection | Empty `current_allowed_tool_names` filter in `tool_phase` | **unreachable** in this branch |

Layer 3 is the only one that can recover from a non-compliant model. Because
the branch bypasses it, a single non-compliant LLM response is enough to
leave the session in a degraded state that **no existing guardrail** detects:

- `handle_narration_loop_escalation` (`response_phase.rs:240-314`) requires
  `tool_calls.is_empty()` and never fires here.
- `detect_repetitive_loop` (`loop_detection.rs:48-69`) skips the agent
  metadata tools most likely to be emitted by a summary request.
- `consecutive_malformed_tool_args`
  (`tool_phase.rs:130-168`) only increments inside `run_tools`.
- The step-budget check (`step_gate.rs:96-112`) eventually terminates the
  session but after burning N LLM round-trips and producing a user-facing
  "Step budget exhausted" error rather than a graceful summary.

## Reproduction

No field repro log is attached to this report (the bug was found by reading
`coordinator.rs`), but a deterministic synthetic repro is straightforward:

1. Stub or mock the LLM client so that when it sees a request whose last
   user message starts with `"All subtasks have been completed"`, it replies
   with `content = "I'll summarize."` plus a single
   `agent_narrate` tool call (any valid arguments).
2. Seed a task with one or more subtasks, all in `TaskStatus::Completed`.
3. Spawn the agent via the normal application entrypoint (e.g.
   `AgentServiceImpl::send_request`).
4. Observe:
   - `agent_runtime_traces` rows with
     `payload LIKE '%all_subtasks_completed_final_summary%'` appear once per
     iteration.
   - `ctx.llm_context` (visible via the checkpoint snapshot) accumulates
     one duplicate `"All subtasks have been completed..."` user message
     per iteration with no `assistant` messages between them.
   - On Anthropic: the next `call_llm` fails with a role-alternation error
     after the first duplication.
   - On OpenAI or a permissive mock: the loop continues until
     `AgentRuntimeError::StepBudgetExhausted` fires at `step_gate.rs:111`.

A pure unit-level repro is even simpler: construct an `ExecutionContext`
with `all_subtasks_completed = true`, call the coordinator loop body once
with a stubbed `handle_response` that returns `ResponseDecision::ExecuteTools`,
and assert that after the iteration the last two messages of `ctx.llm_context`
are both `Role::User`.

## Impact

- **Silent loss of the LLM's summary response.** The assistant turn that was
  intended to answer "summarize what was accomplished" is dropped without
  being shown to the user, persisted to telemetry, or streamed to the
  frontend.
- **Malformed conversation history.** Back-to-back `user` messages break
  Anthropic requests outright and confuse other providers. The history is
  stored in `ExecutionCheckpoint.llm_context`
  (`core/agent/execution.rs:76`), so a checkpoint captured in this state is
  also unusable for resume.
- **Unbounded duplication inside a bounded step budget.** Each iteration
  appends another copy of the "All subtasks have been completed..." user
  message. On a generous step budget the `llm_context` can grow large
  enough to trip the context-window ceiling before the step ceiling,
  producing a second-order failure with an even less actionable error.
- **Wasted LLM spend.** Every iteration is a full LLM round-trip whose
  response is thrown away.
- **User-visible failure mode is a cryptic "Step budget exhausted" or
  "Execution interrupted"** with no indication that the agent actually
  finished its subtasks successfully. The completed work is saved, but the
  finalization UI never gets a summary to render.

## Proposed direction (needs team review before implementation)

Three options, in order of increasing invasiveness. Pick one; do not stack.

### Option A — Route the response through `run_tools` (recommended)

Change the branch so the non-`Finished` arm runs the same tool phase the
main loop runs. This activates the layer-3 defense the comment already
promises:

```rust
match handle_response(&deps, &mut ctx, session, llm_result).await? {
    ResponseDecision::Finished(result) => return Ok(result),
    ResponseDecision::ContinueWithNextSubtask => continue,
    ResponseDecision::ExecuteTools(response) => {
        match run_tools(
            &deps,
            &mut ctx,
            session,
            response,
            &permission_resolver,
            &yolo_mode_provider,
        ).await? {
            ToolPhaseOutcome::Continue => continue,
            ToolPhaseOutcome::Cancelled => return Err(AgentRuntimeError::Cancelled),
        }
    }
}
```

With this change:
- The assistant message is pushed to `llm_context` (fixing the
  role-alternation break).
- Every tool call is rejected with the existing `subtask_mode_disallowed`
  error-result (`permissions.rs:203-219`), which the model sees on the next
  step and can self-correct against.
- The duplicate user-prompt push on the next iteration still happens, but
  it now sits between valid `assistant` / `tool_result` turns — the history
  stays well-formed.

Remaining worry: `loop_detection` still skips the agent-metadata tools, so
a model that keeps emitting `agent_narrate` forever is only caught by the
step budget. Acceptable as a first fix; see Option C for a hardening follow-
up.

### Option B — Fail fast with a dedicated error

If the team prefers to treat a tool call during the final-summary branch as
a contract violation rather than an edge case, introduce
`AgentRuntimeError::FinalSummaryToolCallViolation { tool_names }` and fail
the session immediately instead of continuing. Pros: zero risk of wasted
iterations. Cons: users whose LLM is only intermittently non-compliant lose
the summary and see a hard failure instead of a recovered soft failure.

### Option C — Guard the duplicate push, and narrow loop-detection skips

Orthogonal to A/B. Two small hardening changes:

1. In the final-summary branch, only push the `"All subtasks have been
   completed..."` message if the last message in `ctx.llm_context` is not
   already that exact string. Prevents runaway duplication regardless of
   how the response is handled.
2. Narrow `LOOP_SKIP_TOOLS` in
   `src-tauri/src/infrastructure/agent/runtime/loop_detection.rs` so that
   `agent_narrate` is *not* skipped during the final-summary branch. The
   skip list exists so that routine narration does not count as a loop in
   the middle of normal execution, but during finalization any further
   narration is by definition a loop. Gate the skip on
   `ctx.current_allowed_tool_names.as_deref() != Some(&[])`.

## Adjacent concerns surfaced during investigation

These are **not** part of this bug but fell out of the same reading pass
and should get their own issues if the team agrees they matter:

- **Checkpoint resume with a malformed `llm_context`.** If a session is
  checkpointed while in the degraded state described here, `from_checkpoint`
  (`execution/state.rs:204-234`) will restore the duplicate-user-message
  history as-is. Resume will hit the same Anthropic 400 immediately.
- **Tool filtering is silent in traces.** `tool_phase` emits
  `disallowedByMode` in the `ToolBatchStarted` trace
  (`tool_phase.rs:92-104`) but there is no matching event surfaced to the
  session panel; ops cannot see a slip-through without running SQL.
- **`context_usage_percent` is computed but never enforced.**
  `step_gate.rs:118-120` measures the ratio and persists it on every step,
  but no code path degrades gracefully when it approaches 100 %. The final-
  summary duplication can push a session over the window before anything
  else fires.
