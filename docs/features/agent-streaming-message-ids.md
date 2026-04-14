# Agent Streaming Message IDs

**Status**: ACTIVE
**Last reviewed**: 2026-04-14
**Purpose**: Specification for the stable, collision-free id scheme used on every `streaming_content` event emitted by the agent runtime.

---

## Why this exists

When an agent plans a root task, it breaks it into subtasks and runs them one after another. Each time a subtask finishes or the loop transitions, the runtime emits a `StreamingContent` event carrying the assistant-visible text for that phase. The frontend chat store merges successive events by `messageId`: any event that arrives with an existing id **updates the existing chat message in place** instead of appending a new one.

Before this scheme was introduced, three of the runtime's emission points passed `message_id: None`. The frontend fell back to a single, session-wide id of the form `streaming-assistant-{sessionId}`. The result was that every subtask result, every "continuing with next subtask..." divider, and the final summary all collapsed onto the same chat message — earlier messages visibly disappeared as each new one landed.

This document specifies the id scheme that prevents the regression and the defensive frontend behaviour that guarantees it.

---

## The id scheme

Every streaming emission from the backend carries a `messageId` with the following format:

```
stream-{phase}-{sessionId}-{taskId}-{subtaskId}-s{stepNumber}
```

- `phase` — a slug identifying what kind of emission this is (see below).
- `sessionId` — the agent session id.
- `taskId` — the root task id.
- `subtaskId` — the current subtask id, or the literal string `no-subtask` when the emission belongs to the root task rather than a specific subtask.
- `stepNumber` — the execution step number, prefixed with `s`.

The id is deterministic in its inputs. Reusing the same `(phase, session, task, subtask, step)` tuple intentionally updates the same chat message (e.g. when the same emission is chunked across multiple events). Any difference in any of the five fields produces a distinct id.

### Phases

The runtime distinguishes four phases, each with its own slug:

| Phase | Slug | Emitted from | Meaning |
|---|---|---|---|
| Tool preamble | `tool-preamble` | `response_phase.rs` | Intermediate narration the LLM returns alongside tool calls. |
| Subtask continuation | `subtask-continuation` | `response_phase.rs` | The short "Continuing with next subtask..." divider shown when the loop restarts for the next subtask. |
| Subtask result | `subtask-result` | `termination.rs::complete_execution` | Result content emitted when a subtask finishes but more work remains (another InProgress subtask, or a newly advanced Pending subtask). |
| Final result | `final-result` | `termination.rs::complete_execution` | The final summary emitted when all subtasks terminate. |

### Source of truth

The id format is built by a single pure function — `streaming_message_id` in `src-tauri/src/application/agent/execution/message_ids.rs`. Callers pass a `StreamingMessageIdInput`; the function handles the `subtask_id: None → "no-subtask"` fallback and the formatting.

New emission points **must** use this helper. Ad-hoc `format!` calls that construct streaming ids are not allowed: they defeat the invariant that every emission has a unique id matching the documented scheme.

---

## Backend contract

Both `emit_streaming_content` and `emit_streaming_message` in `src-tauri/src/application/agent/execution/termination.rs` take a **required** `message_id: String` argument. The previous signature accepted `Option<String>` and silently collapsed `None` to a session-wide fallback on the frontend; that is no longer possible.

The compiler now enforces the contract: any new caller of these helpers must pass an explicit id built from `streaming_message_id`.

---

## Frontend contract

`src-ui/src/lib/stores/agent/events/streamingEvents.ts` receives the `streaming_content` event and routes each chunk into the chat store under the event's `messageId`.

If the backend ever emits a `streaming_content` event without a `messageId` (which should never happen after the backend contract above), the frontend now:

1. Generates a **unique, per-event** fallback id of the form `streaming-orphan-{sessionId}-{uuid}`. This guarantees the missing-id event does not silently overwrite any existing chat message.
2. Emits a warning via `createLogger('streamingEvents')` so a regression is visible in the logs.

This "fail loud, not silent" fallback is a regression safety net. It is not a supported path — the backend contract requires `messageId` on every emission.

---

## Tests

The invariants above are covered by unit tests at two layers.

**Backend** — `src-tauri/src/application/agent/execution/message_ids.rs` (in-module tests):

- `different_phases_produce_different_ids_for_the_same_step` — four phases with otherwise identical inputs yield pairwise-distinct ids.
- `same_phase_with_different_subtasks_produces_different_ids` — two subtasks in the same step produce distinct ids.
- `same_phase_with_different_steps_produces_different_ids` — step number participates in the id.
- `same_inputs_produce_the_same_id` — the function is deterministic; same tuple → same id.
- `id_contains_all_input_components` — every input field appears in the rendered id.
- `missing_subtask_is_handled_without_collision_with_real_subtask_ids` — documents the `no-subtask` placeholder.

**Backend integration** — `src-tauri/src/application/agent/execution/termination.rs` (in `mod tests`):

- `subtask_result_ids_differ_between_two_subtasks_in_the_same_session` — the "messages replaced on the frontend" regression test.
- `final_result_id_differs_from_subtask_result_id` — the final summary cannot overwrite the last subtask's result.
- `tool_preamble_ids_differ_across_subtasks_when_step_resets` — tool-preamble ids disambiguate by subtask, not only by step.

**Frontend** — `src-ui/src/lib/stores/agent/events/streamingEvents.test.ts`:

- `keeps two concurrent streams in the same session under distinct keys when messageIds are distinct` — two explicit ids in the same session are preserved as two independent streaming messages.
- `does not collapse two streams with missing messageId into the same session-wide key` — the defensive fallback produces distinct orphan ids per event.

---

## Historical context

The bug that motivated this scheme was reported as "agent messages are rendered but then replaced by other messages coming from the agent" when the agent ran a plan composed of several subtasks. Root cause: three `emit_streaming_content` call sites in `complete_execution` passed `message_id: None`, and the frontend fell back to a single per-session id. Each subtask result, the "continuing" divider, and the final summary all landed on the same chat message and overwrote each other. The fix replaces the session-wide fallback with a phase-aware, subtask-aware id.
