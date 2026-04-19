# `agent_plan_subtasks` malformed-JSON error aborts agent session

- **Status**: fixed — recovery path shipped 2026-04-19
- **Reported**: 2026-04-19
- **Area**: `src-tauri/src/infrastructure/llm/openai/client.rs`,
  `src-tauri/src/infrastructure/llm/error.rs`,
  `src-tauri/src/infrastructure/tools/agent/plan_subtasks.rs`
- **Severity**: high — a single malformed tool-argument payload from the LLM
  provider terminated the whole agent session with no recovery path.

## Summary

When the LLM provider returns a tool call for `agent_plan_subtasks` whose
`function.arguments` string is not valid JSON, the OpenAI client converts the
`serde_json` parse failure into a non-retryable `LlmError::Parse` and the entire
agent session aborts. The UI shows `Execution interrupted` and the user has no
way to recover the in-flight task.

## Evidence (backend log, 2026-04-19 17:47:29)

```
[ERROR] coder_lib::infrastructure::llm::openai::client
  Failed to parse tool arguments for 'agent_plan_subtasks':
  key must be a string at line 1 column 718.
  Arguments: {"framing":{"domainPriors": {"hints": [
    "B2B SaaS sales presentations typically focus on ROI, ease of adoption, and scalability",
    "Sales teams need clear differentiation from competitors and use case examples",
    "Technical buyers appreciate architecture diagrams and security/compliance information",
    "Include customer testimonials or use case scenarios when possible"
  ]}, "forbiddenPatterns": ["^placeholder\\s*$", "^\\[.*\\]\\s*$", "TODO|FIXME|XXX"],
      "role": "senior sales communication professional",...

[ERROR] coder_lib::application::agent::execution::llm_phase
  [LLM Phase] LLM call failed for session '3ba6d535-d67a-4357-a0fb-654639fb9323',
  task '8543ccb6-e515-48e2-ad84-d895890d6a45', step 5:
  LLM error: Failed to parse response:
  Provider returned malformed tool arguments for 'agent_plan_subtasks':
  key must be a string at line 1 column 718.
  Arguments JSON could not be parsed.

[ERROR] coder_lib::application::agent::service::runtime_queries
  [AgentRuntime] Execution failed for session 3ba6d535-d67a-4357-a0fb-654639fb9323:
  LLM error: Failed to parse response: ...
```

Session / task IDs from the log that should be correlated in
`coder_agents.db`:

- `session_id = 3ba6d535-d67a-4357-a0fb-654639fb9323`
- `task_id    = 8543ccb6-e515-48e2-ad84-d895890d6a45`
- `step       = 5`

Suggested DB queries (run `sqlite3 -readonly`):

```sql
-- The LLM request/response trace for the failing step
SELECT created_at, trace_type, substr(payload, 1, 400)
FROM agent_runtime_traces
WHERE session_id = '3ba6d535-d67a-4357-a0fb-654639fb9323'
  AND task_id    = '8543ccb6-e515-48e2-ad84-d895890d6a45'
  AND step_number = 5
ORDER BY created_at;

-- Any tool call the runtime managed to persist before the abort
SELECT id, tool_name, status, substr(arguments, 1, 400), substr(result, 1, 400)
FROM agent_tool_calls
WHERE session_id = '3ba6d535-d67a-4357-a0fb-654639fb9323'
ORDER BY created_at;
```

The `LlmResponseReceived` trace (llm_phase.rs:230) is the authoritative record
of the raw arguments string — it is not truncated to 500 characters like the
stderr log, so it should contain the exact character at column 718.

## Root cause (current understanding)

1. **LLM emitted invalid JSON.** `key must be a string at line 1 column 718`
   is a `serde_json` error that fires when the parser expects a property name
   (a quoted string) inside an object. Common upstream causes: an unquoted
   key, a trailing comma followed by `}`, a stray character breaking out of a
   string (over-escaped backslashes in the `forbiddenPatterns` regexes are a
   likely culprit), or a duplicate `{`.
   *We cannot confirm which one from the current log* — the logged snippet is
   truncated at 500 chars while the error is at column 718.

2. **Parse errors are non-recoverable.**
   `src-tauri/src/infrastructure/llm/error.rs:122-131` classifies
   `LlmError::Parse` as not retryable, so `chat_with_retry`
   (`src-tauri/src/infrastructure/agent/runtime/client.rs:143`) returns
   immediately. The `call_llm` phase propagates the error and the runtime
   tears the session down (see
   `src-tauri/src/application/agent/service/runtime_queries.rs:123`).

3. **The `agent_plan_subtasks` schema is the most complex one we ship.** Its
   `framing` object nests `domainPriors.hints`, `voiceConstraints`, and
   `forbiddenPatterns` (a list of regex strings with escaped backslashes).
   Heavily-escaped strings are a well-known failure mode for LLM
   structured-output generation. The tool is also called early (step 5 here),
   so a single serialization slip aborts the whole task rather than a minor
   sub-step.

4. **Debuggability gap.** `src-tauri/src/infrastructure/llm/openai/client.rs:508`
   truncates the logged arguments to 500 characters. The reported failure was
   at column 718, so the log cannot show the offending substring. A window
   around the parser's reported column would make triage trivial.

## Reproduction

No deterministic repro — the failure depends on LLM output. Likely triggers:

1. A communication-style task (the log suggests a B2B SaaS sales presentation)
   that causes the planner to populate `framing` with a `forbiddenPatterns`
   array of regexes containing backslash escapes.
2. A model/provider combination prone to over- or under-escaping backslashes
   when emitting tool-call JSON. Inspect `agent_runtime_traces.payload.model`
   for the failing step to identify the exact model.

To reproduce in-app:

1. Start an agent task in the "communications / sales copy" domain so the
   planner is pushed toward the `framing` branch of the `agent_plan_subtasks`
   schema.
2. Run repeatedly against the same model; a minority of runs will emit
   malformed JSON around the `forbiddenPatterns` regex strings.
3. Observe `Execution interrupted` in the panel header and the three-error
   stack above in the backend log.

A deterministic unit-level repro is straightforward: feed a hand-crafted
malformed `function.arguments` string into the OpenAI client parse path and
assert on the resulting `LlmError`.

## Impact

- Entire agent session is killed after a single upstream serialization glitch,
  even though the model would almost certainly succeed on a retry.
- No automatic reprompt / self-correction signal is sent back to the LLM.
- Users see `Execution interrupted` with no actionable message and must
  restart the task from scratch.
- The `agent_plan_subtasks` call tends to happen early in execution, so the
  blast radius is the whole task — not a later subtask.

## Proposed direction (needs team review before implementation)

Three complementary changes, in priority order:

1. **Recover instead of aborting.** Introduce a new
   `LlmError::MalformedToolArguments { tool, parser_error, raw_args }` (or
   reuse `Parse` with a marker) and treat it as a *tool-level* failure
   rather than an *LLM-level* failure. The runtime should feed a synthetic
   tool-result back to the model ("your JSON was invalid at column N: …;
   emit a corrected call") and let the normal step loop take another swing.
   The existing no-progress/loop-detection guardrails
   (`src-tauri/src/infrastructure/agent/runtime/loop_detection.rs`) will cap
   retries if the model cannot recover.

2. **Improve diagnostic logging.** Log a ±80-character window around the
   parser's reported column instead of only the first 500 characters, and
   include the column number explicitly. Also ensure the full raw arguments
   string is persisted in the `LlmResponseReceived` runtime trace (it already
   should be — verify and tighten if not) so post-mortem SQL queries can
   reconstruct the failure.

3. **Schema hardening for `agent_plan_subtasks`.** Evaluate whether
   `forbiddenPatterns` should stay on the planner surface at all, or whether
   it should move behind a simpler interface (e.g. a fixed enum of preset
   guardrails) that avoids asking the model to emit escaped-regex strings. If
   it stays, add normalization/repair in
   `src-tauri/src/infrastructure/agent/runtime/helpers.rs` for the common
   "almost-JSON" failure modes before we declare the arguments unusable.

Fix #1 is the minimum bar: a single malformed tool call must not destroy a
task. Fixes #2 and #3 reduce the rate at which #1 fires.

## Resolution (2026-04-19)

Fixes #1 and #2 from the proposed direction shipped together. Fix #3
(schema hardening for `forbiddenPatterns`) is deferred — it reduces the
firing rate but is not load-bearing now that recovery works.

### What changed

- **New module**
  `src-tauri/src/infrastructure/llm/malformed_args.rs` — single source of
  truth for the sentinel contract shared between the LLM client (producer)
  and the agent runtime (consumer). Exposes `MALFORMED_SENTINEL_KEY`,
  `make_sentinel_value`, `extract_marker`, `build_error_result`, and
  `format_window`.

- **Producer**
  `src-tauri/src/infrastructure/llm/openai/client.rs` — on a non-truncation
  JSON parse failure the client now writes a sentinel object into
  `ToolCall.arguments` instead of returning `LlmError::Parse`. The log line
  was upgraded to show the parser's line/column and a `before»CHAR«after`
  window instead of the leading 500 characters, so the debugging blind spot
  at column 718 in the reported incident is gone.

- **Consumer**
  `src-tauri/src/infrastructure/agent/runtime/permissions.rs` — a fast path
  at the top of `process_tool_calls_with_permissions` detects the sentinel,
  persists an `is_error=true` tool result via the normal telemetry/event
  flow, and continues. No permission check, no dispatch. The model sees a
  tool-result message explaining the failure (with the offending column and
  window) on the next step and can self-correct.

- **Bounded retries**
  `src-tauri/src/application/agent/execution/state.rs`,
  `.../execution/tool_phase.rs`, and `.../execution/utils.rs` — a new
  `consecutive_malformed_tool_args` counter on `ExecutionContext` is
  incremented on any step where *every* tool call carries the sentinel,
  reset as soon as one real tool call lands, and — when it crosses
  `MALFORMED_ARGS_ABORT_THRESHOLD` (3) — surfaces a new
  `AgentRuntimeError::MalformedToolArgumentsExhausted { attempts, last_tool }`
  so a model that cannot self-correct still terminates cleanly instead of
  looping forever.

### Why not reuse `tool_feedback.rs`

`tool_feedback` only builds the *narration copy* (`"Let me read X."`) shown
in the UI — it does not own the tool-result objects fed back to the LLM.
Those originate in `permissions.rs::process_tool_calls_with_permissions`,
which is exactly where the recovery needed to live.

### Verification

- `cargo test --lib malformed_args` — 7 unit tests cover window extraction
  (ASCII, multi-byte, past-EOF, and far-past-EOF), sentinel round-trip,
  negative extraction on regular arguments, and `build_error_result`
  formatting.
- `cargo test --lib checkpoint_tests runtime_enforcement_tests plan_subtasks`
  — existing suites pass with the new `ExecutionContext` field.
- `cargo clippy --lib` — no new warnings on the touched files.

### Deferred follow-ups

- Schema hardening for `agent_plan_subtasks.framing.forbiddenPatterns`
  (proposed direction #3). Lower the rate at which the recovery path has to
  fire, ideally by removing escaped-regex strings from the planner surface.
- Cross-check model attribution for this failure once fresh telemetry is
  available: pull model names from `agent_runtime_traces` where
  `payload -> '$.toolNames' LIKE '%malformed%'` and see whether the
  remaining fires are concentrated on one provider.
