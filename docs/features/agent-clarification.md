# Agent Clarification: Pause with Persistent Resume

**Status**: ACTIVE
**Last reviewed**: 2026-04-19
**Purpose**: User-facing behavior + architecture reference for the
clarification pause/resume flow implemented by the
`agent-clarification-resume` plan. Supersedes the Workstream E
partial implementation from `docs/planning/agent-fidelity/plan.md`.

## What it does

When the agent emits `agent_narrate { intent: "question" }`, the
runtime parks the session on the current task and subtask and waits
for the user to reply. On reply the coordinator resumes the same
loop with the reply appended to the LLM context — no new task, no
re-plan, no fresh LLM planning call. If the app closes mid-pause,
the next launch restores the paused question and accepts the reply
on the rehydrated session.

## User-visible flow

1. **Agent asks.** The chat transcript shows an "agent asked" message
   with the prompt; an inline composer appears above the main chat
   input. The session's `harnessState.mode` becomes
   `awaiting_clarification` with `prompt` and `askedAt` details.
2. **User replies** — from either the dedicated composer or the main
   chat input (auto-routed at the backend). The reply is persisted
   to the transcript as a user message.
3. **Resume.** The coordinator wakes, appends the reply to its
   `llm_context`, transitions the session state back to `Running`,
   and continues the same step on the same subtask.
4. **Timeout fallback.** If `CODER_CLARIFICATION_TIMEOUT_MINUTES`
   (default `30`) elapses before a reply, the active subtask is
   marked `Failed` with reason `unanswered_clarification` and the
   session transitions to `Paused`. A retry affordance surfaces in
   the UI.
5. **Restart.** If the app closes while parked, the next launch
   re-emits `ClarificationRestored` for the paused session. The
   reply path recognises the post-restart condition and rebuilds the
   coordinator from the persisted `ExecutionCheckpoint` (today: the
   full coordinator rebuild is staged as a follow-up; Phase 6
   currently repairs the session to
   `Error { recoverable: true }` so the user is never silently
   stuck — a retry re-spawns the task with the reply in transcript
   context).

## State diagram

```
  ┌─────────┐
  │ Running │◄──────────────────────────┐
  └────┬────┘                           │
       │ agent_narrate(question)        │ reply arrives
       ▼                                │  (live channel or rehydrate)
  ┌──────────────────────┐              │
  │ AwaitingClarification│──────────────┘
  └────┬─────────────────┘
       │ timeout elapsed
       ▼
  ┌────────────────────────────────┐
  │ Paused                         │
  │ (active subtask Failed with    │
  │  reason unanswered_clarification)│
  └────────────────────────────────┘
```

Cancellation is an orthogonal path from any state and is handled by
the cancellation signal observed by the coordinator's park stage.

## IPC contract

| IPC | Direction | Purpose |
|---|---|---|
| `send_agent_request` | UI → backend | Main chat input. Auto-routes to `reply_to_clarification` when a pending clarification exists. |
| `reply_to_clarification` | UI → backend | Clarification composer. Persists the reply to the transcript and delivers it to the parked coordinator (live path) or rebuilds from checkpoint (post-restart path). |
| `initialize_agent_recovery` | UI → backend | Called at startup. Rehydrates paused clarifications, emitting `ClarificationRestored` / `ClarificationTimedOut` as appropriate. |

## Events

| Event | Purpose |
|---|---|
| `session_state_changed` with `state.type = 'awaiting_clarification'` | Session entered the pause. Carries `prompt`, `askedAt`, optional `subtaskId`. |
| `clarification_replied` | The reply was delivered. Mirrors the transcript append. |
| `clarification_timed_out` | Timeout elapsed before a reply. Carries the original prompt for the UI fallback message. |
| `clarification_restored` | Emitted at startup when a paused session is rehydrated. Carries the same payload as the original pause event. |

## Persistence

Two tables, introduced by migrations 9 and 10:

- `agent_pending_clarifications` — authoritative pause record
  (`session_id`, `task_id`, `subtask_id`, `prompt`, `asked_at`).
- `agent_execution_checkpoints` — serialised `ExecutionCheckpoint`
  (`schema_version`, full `llm_context` + step bookkeeping).

Both cascade on session delete. Checkpoint `schema_version` guards
forward compatibility: unknown versions degrade to a recoverable
error rather than panicking.

## Timeout configuration

`CODER_CLARIFICATION_TIMEOUT_MINUTES` (env var) sets the timeout.
Defaults to `30`. Values ≤ 0 disable the timeout (park alive until
the user replies or the session is cancelled). Malformed values fall
back to the default.

## Invariants (asserted by tests)

1. A session has at most one pending clarification at a time.
2. `SessionState::AwaitingClarification` and the
   `agent_pending_clarifications` row are present together or absent
   together (enforced by the park-entry writes).
3. Reply delivery is at-most-once. The second attempt returns
   `ClarificationError::ReplyAlreadyDelivered`.
4. Timeout fires exactly once per clarification. A restart with an
   already-expired clarification fires the timeout path on load.
5. `llm_context` order is preserved across checkpoint round-trip.

## Failure and recovery paths

| Situation | Handling |
|---|---|
| Orphan clarification row (no checkpoint) | Repair to `Error { recoverable: true }`. |
| Expired on rehydrate | Timeout path (mark subtask Failed, transition Paused, emit `ClarificationTimedOut`). |
| Checkpoint schema version unsupported | Load returns a structured error; caller repairs to `Error { recoverable: true }`. |
| Sender dropped before reply | Deliver falls through to the post-restart path. |

## Code pointers

- `src-tauri/src/application/agent/clarification.rs` — dual-layer
  store (process-local channel registry + `ClarificationRepository`).
- `src-tauri/src/application/agent/execution/park.rs` — coordinator
  park stage.
- `src-tauri/src/application/agent/service/clarification.rs` —
  application-layer reply + rehydrate.
- `src-tauri/src/core/agent/execution.rs` — `ExecutionCheckpoint`.
- `src-tauri/src/infrastructure/sqlite/repositories/agent/clarifications.rs`
  and `execution_checkpoints.rs` — SQL layer.
- `src-ui/src/lib/components/agent/AgentClarificationPrompt.svelte` —
  UI affordance.
- `src-ui/src/lib/stores/agent/events/sessionEvents.ts` — event
  handlers for the three clarification events.
