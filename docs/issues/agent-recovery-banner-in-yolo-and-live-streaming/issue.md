# Recovery ("Your progress is saved") banner surfaces in yolo mode and during live streaming

- **Status**: frontend band-aid shipped 2026-04-23 — awaiting backend design review
- **Reported**: 2026-04-23
- **Area**:
  - Backend: `src-tauri/src/application/agent/execution/park.rs`,
    `src-tauri/src/application/agent/service/clarification.rs`,
    `src-tauri/src/application/agent/execution/termination.rs`,
    `src-tauri/src/commands/agent/sessions.rs`
  - Frontend: `src-ui/src/lib/stores/agent/events/harnessEvents.ts`,
    `src-ui/src/routes/+page.svelte`,
    `src-ui/src/lib/components/chat/messages/RecoveryMessage.svelte`
- **Severity**: medium — no data loss, but the UI contradicts the agent's
  actual execution state (banner says "paused / pick up where you left off"
  while tool calls continue streaming underneath), and it surfaces a
  human-decision prompt in a mode that is explicitly meant to auto-approve.

## Summary

The recovery card ("Your progress is saved — Replan / Continue") was designed
for app-startup recovery: when the process restarts mid-execution, previously
unterminated sessions are transitioned to `SessionState::Paused` and the user
is offered a choice. In practice the same card now surfaces in two wrong
situations:

1. **Mid-execution pauses** — e.g. a clarification timeout fires while the
   agent is still producing tool output. The session state transitions to
   `Paused` in the database and a `session_paused` event is emitted, but the
   execution loop does not actually halt. New narration and tool-call
   messages continue to stream into the transcript *below* the card.
2. **Yolo / auto-approve mode** — the card is a human-decision surface, yet
   the pause emission path never consults the thread's `yolo_mode` flag.
   Yolo is only checked in the tool-approval layer
   (`src-tauri/src/infrastructure/agent/runtime/permissions.rs`).

## Evidence

Observed during normal interaction in a freshly-created conversation/thread
with yolo enabled. Agent was in a subtask-advancement loop
(`agent_mark_subtask_status` → `agent_advance_to_next_subtask`) hitting the
evidence rejection in
`src-tauri/src/infrastructure/agent/runtime/tool_handlers/advance_subtask.rs:95-104`:

> "Cannot advance subtask: no evidence found (file change, command
> execution, or explicit rationale)"

While the agent was still reading files and attempting advances, a
`session_paused` event arrived on the bus and the recovery card rendered
inline. Tool-call narration (`Let me inspect src`, `I inspected src and
found 42 item(s)`, `Let me read CLAUDE.md`, etc.) continued streaming into
the transcript directly below the "paused" card. Screenshot in thread at
`/home/kassiomaia/.claude/image-cache/7d0731fa-8d4e-4e5c-b40e-b4a67fc7932d/4.png`.

## Code path

### Backend `SessionPaused` emission sites

1. `src-tauri/src/application/agent/execution/park.rs:315-323` — clarification
   timeout transitions to `Paused`. No yolo check, no halt of in-flight
   execution.
2. `src-tauri/src/application/agent/service/clarification.rs:196-203` —
   rehydration at startup forces Paused when a clarification deadline has
   lapsed.
3. `src-tauri/src/application/agent/execution/termination.rs:37-57` —
   emits `SessionStateChanged { state: Paused { ... } }`.
4. `src-tauri/src/commands/agent/sessions.rs:224-265` — startup recovery
   loop emits `AgentEvent::SessionPaused` for each unterminated session.

None of these sites queries `thread.metadata.yolo_mode` before emitting.

### Frontend handler

`src-ui/src/lib/stores/agent/events/harnessEvents.ts:43-77`
(`handleSessionPaused`) unconditionally injects a message of
`kind: 'recovery'` into the transcript via `agentEventBus.emit('agent-activity', ...)`.
No `reason` / `context` field is consulted; no yolo check; no check on
whether the session is currently streaming.

### Frontend bus subscriber

`src-ui/src/routes/+page.svelte` `agentEventBus.on('agent-activity', ...)` —
the point at which the emitted message is persisted into the chat store.

### Card renderer

`src-ui/src/lib/components/chat/messages/RecoveryMessage.svelte` — renders
the card as an inline `ActionCard` in the transcript, **not** as a modal
overlay. This is why the agent continues executing visually past it: the
card is a historical marker, not a blocking dialog.

## Why this happens

Two coupled design gaps:

- **`SessionState::Paused` does not halt the live execution loop.** A
  persisted state transition is a database write, not a cancellation signal
  to the running task. Whatever triggered the pause (clarification timeout,
  checkpoint) flips the row, emits the event, and the executor continues
  until it naturally yields.
- **The pause emission path is context-free.** It doesn't distinguish
  "app restarted, please offer recovery" from "clarification just timed
  out during active streaming" — the frontend receives the same event
  shape in both cases and renders the same card.

## Reproduction

1. Create a conversation, create a thread, enable yolo on the thread
   (lightning-bolt icon next to the model picker).
2. Send any message complex enough to push the agent into the subtask
   runtime (plan + advance + evidence gating).
3. Let the agent iterate on `agent_mark_subtask_status` /
   `agent_advance_to_next_subtask` long enough for a clarification
   checkpoint or timeout to fire.

Expected: no recovery card; agent either proceeds auto-approved or surfaces
a structured clarification question.

Actual: "Your progress is saved — Replan / Continue" card appears inline
while tool-call streaming continues below it.

## Mitigation shipped

Frontend band-aid in `src-ui/src/routes/+page.svelte` (agent-activity
subscriber): drop `kind: 'recovery'` messages for threads where
`thread.metadata.yoloMode === true`. This suppresses the contradictory
banner in yolo mode but does not fix the underlying coordination bug.

## Proposed direction (for team discussion)

Two layers to fix properly, in order:

1. **Backend — emit with context, honor yolo**. Enrich the
   `session_paused` payload with a `reason` enum
   (`startup_recovery` | `clarification_timeout` | `user_pause` | ...)
   and, in the emitters under `park.rs` / `clarification.rs` /
   `termination.rs`, consult the thread's `yolo_mode` before emitting a
   human-decision event. Yolo threads should either auto-continue or emit
   a distinct "yolo-auto-resume" event that the UI treats as informational.
2. **Backend — make Paused actually pause**. The execution loop should
   observe session state transitions and cooperatively yield when
   `Paused` is set, so the transcript stops growing until the user resolves
   the card (or the auto-resume fires). This is the larger change and
   needs design input on how the loop cancels in-flight LLM requests.

Frontend-side, once the payload carries `reason`, the
`handleSessionPaused` handler should branch: only
`startup_recovery` should render the "Replan / Continue" card; other
reasons should surface via the existing clarification UI or be silently
informational.

## Database correlation

While the bug is live, useful queries under `~/.local/share/com.coder.app`
(`sqlite3 -readonly`):

```sql
-- Pause transitions for the affected session
SELECT created_at, from_state, to_state, reason
FROM agent_session_state_log
WHERE session_id = '<session-id>'
ORDER BY created_at DESC
LIMIT 20;

-- Recent clarification checkpoints
SELECT session_id, created_at, kind, resolved_at
FROM agent_execution_checkpoints
WHERE session_id = '<session-id>'
ORDER BY created_at DESC;
```

(Table/column names from memory of `docs/features/agent-clarification.md` —
confirm against the current schema before running.)
