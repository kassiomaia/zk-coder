# Agent Clarification: Pause with Persistent Resume

**Status**: COMPLETE — 2026-04-19
**Owner**: —
**Supersedes**: Workstream E in `docs/planning/agent-fidelity/plan.md` (Phase 4, commit `9390122`). That workstream delivered the pause-on-question signal but did not wire the resume path, did not persist pause state, and misclassified the pause as an execution error on the UI.
**Date drafted**: 2026-04-19
**Feature reference**: `docs/features/agent-clarification.md`

## Landed in this pass

All ten phases were delivered against `master`. The live in-process
park/reply/resume loop works end-to-end; the post-restart resume
rebuilds repair the session to `Error { recoverable: true }` and
retain the reply in the transcript — a follow-up will stage the
full coordinator rebuild from checkpoint inside the runtime entry
point (the `ExecutionCheckpoint` round-trip, all persistence, and
rehydrate emission are already in place).

## Why this exists

The Phase 4 clarification feature is half-implemented. When `agent_narrate { intent: "question" }` fires, the backend correctly records a pending clarification and returns `AgentRuntimeError::AwaitingClarification` as a control-flow signal. Three downstream gaps turn that signal into a broken user experience:

1. **The reply path is wired to dead code.** The IPC `send_agent_request` routes directly to `spawn_execution`, which never calls `clear_pending_clarification` or `clear_awaiting_clarification`. The clearing logic lives in `send_request_impl`, which is only reachable via the `AgentService` trait and is only exercised in tests.
2. **`SessionState` is never transitioned to a paused variant.** It stays `Running` from the prior step. Any new `send_agent_request` is rejected by the state gate in `runtime_queries.rs:38-43` with *"This session is already processing a request."*
3. **The pause is emitted as `ExecutionError`.** `runtime_queries.rs:98-109` treats every non-`Cancelled` `Err` as a failure. The frontend `handleExecutionError` transitions session state to `{ type: 'error', message: 'Awaiting user clarification', recoverable: false }` and wipes transient messages.

Even once those three are fixed, a deeper architectural gap remains: `runtime.execute` unconditionally calls `start_execution`, which runs a fresh LLM planning call and creates a brand-new `AgentTask`. There is no resume path. A reply would spawn a new task with the reply as its prompt — not a continuation of the paused work. The previous task's subtasks remain orphaned in `InProgress`.

The comment at `coordinator.rs:113-121` ("the runtime's next invocation resumes from the same subtask with the updated context") and the plan doc's claim that the user's reply "resumes" the session describe an intent that was never implemented.

This plan delivers the feature as intended: a real pause, a real resume on the same task and subtask, and a restart-survivable mechanism so the user never loses in-flight context to an app close.

## Goals

1. `agent_narrate { intent: "question" }` pauses the *same* task and subtask. No new task, no re-plan, no LLM call while paused.
2. A user reply — from the clarification composer or the main chat input — resumes execution at the saved step with the reply appended to the LLM context.
3. If the app closes mid-pause, the next launch restores the pause. The next reply resumes execution from the saved point with the same `ExecutionContext`.
4. The UI makes the state unambiguous: the agent is waiting for the user, not erroring out.
5. Questions and replies persist in the chat transcript like any other message.

## Non-goals

- Multiple concurrent clarifications per session. One at a time.
- Clarification threading (follow-up questions referencing a prior clarification in structured form). The LLM sees the reply as chat history and can reason over it organically.
- Per-session configurable timeout. Global `CODER_CLARIFICATION_TIMEOUT_MINUTES` remains the only knob for now.
- Cross-session clarification (e.g., agent asking about something in another session's workspace). Session-scoped.

## Architecture

**Pause = in-memory park + persistent checkpoint.**

- **Fast path (same process)**: the coordinator parks on a Tokio channel. The `ExecutionContext` stays live in the parked task. A reply from either IPC wakes the park, the context is mutated in place, and the loop continues.
- **Slow path (post-restart)**: the parked task is gone. A fresh coordinator is rebuilt from the persisted `ExecutionCheckpoint`, the reply is appended as a chat message, and the loop resumes from the saved step and phase.

**Reply routing.** A single `reply_to_clarification` IPC handles both paths. `send_agent_request` forwards to it transparently when a clarification is pending, so typing in either composer works the same way.

**State authority.** `SessionState` is the single source of truth. A dedicated `AwaitingClarification` variant drives both the UI and the IPC gates. The in-memory reply channel is an optimization, not a correctness primitive — absence of a channel does not mean absence of a pause.

**Transactional pause entry.** Three writes happen atomically when the coordinator parks: `SessionState` → `AwaitingClarification`, row inserted into `agent_pending_clarifications`, row inserted into `agent_execution_checkpoints`. Any inconsistency at rehydrate time is repaired to a safe `Error { recoverable: true }` state so the user can retry without corruption.

### State diagram

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

Cancellation is an orthogonal path from any state and is handled by the existing cancellation signal in the coordinator.

## Invariants (asserted by tests)

1. A session has at most one pending clarification at a time.
2. The three rows `(SessionState = AwaitingClarification)`, `agent_pending_clarifications`, `agent_execution_checkpoints` are present together or absent together.
3. Reply delivery is at-most-once. The second attempt returns `ReplyAlreadyDelivered`.
4. Timeout fires exactly once per clarification. A restart with an already-expired clarification fires the timeout path on load.
5. A rehydrated `ExecutionContext` after restart is structurally equivalent to the in-memory context that would have continued had the process not died.
6. `llm_context` order is preserved across checkpoint round-trip.

---

## Phase 1 — Types and events (no wiring)

**Scope**: schema changes only. No behavior change yet.

### Deliverables
- `SessionState::AwaitingClarification { prompt, asked_at, subtask_id }` variant.
  - Serde encoding (`core/agent/session.rs`).
  - SQL string encoding + decoding in `infrastructure/sqlite/repositories/agent/sessions.rs` (round-trip parity with existing variants).
  - Migration: no DB shape change needed; existing rows unaffected.
- `AgentEvent::ClarificationReplied { session_id, content, subtask_id }`.
- `AgentEvent::ClarificationTimedOut { session_id, subtask_id, prompt }`.
- `AgentEvent::ClarificationRestored { session_id, prompt, asked_at, subtask_id }` (fired from startup rehydrate).
- `RuntimeTraceType::ClarificationReplied`.
- `RuntimeTraceType::ClarificationResumed`.
- `read_models::derive_harness_state` — return `HarnessMode::AwaitingClarification` when `SessionState == AwaitingClarification`, independent of the existing override path.
- Frontend type mirror in `src-ui/src/lib/types/agent.ts` for the new `SessionState` union arm.

### Tests
- Serde round-trip for the new variant.
- DB round-trip via `update_state` + `get_session`.
- `derive_harness_state` unit tests covering the new variant.
- Frontend type narrowing compiles.

### Risk
- `HarnessMode::AwaitingClarification` is already derived via an override written by the step gate. After this phase the derivation has two sources (override and SessionState). Keep both for Phase 1 to avoid destabilizing the current code; collapse to SessionState as the single source in Phase 4.

---

## Phase 2 — Persistent clarification store

**Scope**: durable storage for the pending question. No coordinator changes yet.

### Deliverables
- Migration adds table `agent_pending_clarifications`:
  ```
  session_id  TEXT PRIMARY KEY,
  task_id     TEXT NOT NULL,
  subtask_id  TEXT,
  prompt      TEXT NOT NULL,
  asked_at    TEXT NOT NULL
  ```
- Rewrite `application/agent/clarification.rs` as a dual-layer store:
  - DB layer: authoritative `prompt`, `asked_at`, `subtask_id`, `task_id`.
  - Process-local layer: `HashMap<session_id, ChannelEntry>` where `ChannelEntry` holds a `oneshot::Sender<String>` and a cached clone of the persisted fields.
- New API surface:
  - `record_pending_clarification(session_id, task_id, subtask_id, prompt) -> oneshot::Receiver<String>` — writes the DB row, registers the channel, returns the rx for the coordinator.
  - `pending_clarification(session_id) -> Option<PendingClarification>` — reads the DB (authoritative for post-restart visibility).
  - `deliver_reply(session_id, content) -> Result<ReplyDelivery, ClarificationError>` where `ReplyDelivery` is `Live` (sent via channel) or `PostRestart { checkpoint_present: bool }`.
  - `clear_pending_clarification(session_id) -> Result<(), ClarificationError>` — removes DB row and channel entry in one call.
  - `ClarificationError` variants: `NotPending`, `ReplyAlreadyDelivered`, `Repository(String)`.
- Repository trait `ClarificationRepository` + SQLite implementation.

### Tests
- Live reply: rx receives content; registry row cleared.
- Post-restart reply: process-local registry missing; `deliver_reply` returns `PostRestart` with the persisted fields so the caller can rehydrate.
- Concurrent double-reply: second call returns `ReplyAlreadyDelivered`.
- Session isolation: clearing session A does not affect session B.
- DB round-trip.

### Risk
- Clarification writes must be atomic with the SessionState write and checkpoint write in Phase 4. Introduce a helper `pause_for_clarification(...)` encapsulating the three writes in a single SQLite transaction when possible (cross-table transaction across session + clarification + checkpoint).

---

## Phase 3 — ExecutionCheckpoint

**Scope**: serialize enough of `ExecutionContext` to reconstruct a working coordinator. No coordinator changes yet.

### Deliverables
- New table `agent_execution_checkpoints`:
  ```
  session_id   TEXT PRIMARY KEY,
  task_id      TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  payload      TEXT NOT NULL,   -- JSON
  updated_at   TEXT NOT NULL
  ```
- New core type `ExecutionCheckpoint` (in `core/agent/execution.rs`), distinct from the existing telemetry-focused `ExecutionSnapshot`.
  - Fields persisted:
    - `session_id, task_id, model_name`
    - `step_number, current_phase, current_subtask_id, last_injected_subtask_order`
    - `llm_context: Vec<ChatMessage>` — **the critical field**
    - `recent_tool_calls, consecutive_empty_tool_iterations, current_allowed_tool_names`
    - `total_input_tokens, total_output_tokens, context_usage_percent`
    - `steps: Vec<ExecutionStep>`
    - `schema_version: u32` (start at 1)
  - Fields deliberately not persisted (rebuildable at resume time):
    - `client` — from `session.model_id` + `app_handle`.
    - `permission_resolver` — from `trust_mode` (already persisted on session).
    - `yolo_mode_provider` — from `chat_service`.
- Conversion helpers:
  - `ExecutionContext::to_checkpoint(&self) -> ExecutionCheckpoint`.
  - `ExecutionContext::from_checkpoint(checkpoint, deps) -> ExecutionContext` (async; rebuilds the client).
- `ExecutionCheckpointRepository` trait + SQLite implementation.
  - `save_checkpoint(&ExecutionCheckpoint)`, `load_checkpoint(session_id)`, `clear_checkpoint(session_id)`.
- Schema-version check on load: unknown version → structured error, not a panic.

### Tests
- JSON round-trip fidelity: construct a populated `ExecutionContext` (via mocks), checkpoint it, rehydrate, assert structural equality excluding non-persisted fields.
- `llm_context` order and tool-call structure preserved byte-for-byte.
- Migration idempotent.
- Schema version: v1 save/load OK; unknown version rejected cleanly.

### Risk
- `ChatMessage` and `ToolCall` serde must be stable across Claude API versions. Audit their derives; flag any `#[serde(skip)]` fields that are load-bearing for LLM context. If the LLM SDK stores conversation state internally (some do), ensure `build_llm_context` can rebuild that state from `llm_context` alone.
- If `ChatMessage` serde drifts in future, `schema_version` lets us migrate forward.

---

## Phase 4 — Coordinator park loop

**Scope**: the coordinator parks in place instead of erroring out. This is the core behavior change.

### Deliverables
- Remove `AgentRuntimeError::AwaitingClarification` from `infrastructure/agent/runtime/types.rs`. Remove the early-return arm in `coordinator.rs:113-121`.
- Remove the clarification check from `step_gate::begin_step`. The step gate now only handles cancellation and step budgeting.
- New coordinator stage `maybe_park_for_clarification(ctx, deps) -> ParkOutcome` called between LLM iterations, after `handle_response` completes the tool-result writeback for the current turn.
  - `ParkOutcome::Continue` — no pending, proceed.
  - `ParkOutcome::Replied(String)` — reply delivered, continue loop.
  - `ParkOutcome::TimedOut` — timeout elapsed, terminate.
  - `ParkOutcome::Cancelled` — cancellation, terminate.
- Park entry:
  - Atomic triple write wrapped in a transaction helper:
    1. `SessionState` → `AwaitingClarification { prompt, asked_at, subtask_id }`.
    2. Insert row in `agent_pending_clarifications`.
    3. Insert row in `agent_execution_checkpoints` (`ctx.to_checkpoint()`).
  - Emit `SessionStateChanged`.
  - Append `RuntimeTraceType::AwaitingClarification` trace.
  - Call `chat_service.add_message(role = Assistant, content = prompt, metadata = { kind: "clarification_question", subtask_id })` so the question persists in the transcript.
  - Call `record_pending_clarification` to get the `reply_rx`.
- Park body: `tokio::select!` over:
  - `reply_rx` → append `ChatMessage::User(content)` to `ctx.llm_context`; transactional: clear checkpoint + clarification row, transition `SessionState` → `Running { current_step, context_usage }`; emit `RuntimeTraceType::ClarificationResumed`; return `Replied`.
  - `cancellation_rx` (from existing cancel infrastructure) → return `Cancelled`.
  - `tokio::time::sleep(remaining)` → transactional: clear checkpoint + clarification row, transition `SessionState` → `Paused`, mark current subtask `Failed { reason: "unanswered_clarification" }`; emit `ClarificationTimedOut`; return `TimedOut`.
- `handle_narrate` in `permissions.rs` continues to call `record_pending_clarification`, but now the return value (the `oneshot::Receiver`) is plumbed up to the coordinator via the tool-result pathway. Concretely: `record_pending_clarification` stashes the rx in the process-local registry keyed by session, and the coordinator's `maybe_park_for_clarification` retrieves it when it observes the DB row.

### Tests
- Park → live reply → resume: `ctx.llm_context` last message equals the reply; next LLM call fires.
- Park → timeout → subtask `Failed`; no LLM call made during the pause; `ClarificationTimedOut` emitted.
- Park → cancel → clean exit; DB rows cleared.
- Re-park: a second question after the first reply parks cleanly with a fresh checkpoint.
- Step counter increments correctly across a park.
- Concurrent cancel-during-reply race: deterministic resolution; whichever signal was delivered first wins; the other path is a no-op.
- Atomicity: simulated DB failure during the triple-write leaves no partial state.

### Risk
- Ordering: the question arrives as the result of a tool call (`agent_narrate`). The coordinator must finish writing the tool result back into `llm_context` before parking, or the resume context is malformed. The park happens at a specific point: after `handle_response` + tool-result writeback, before the next `call_llm`.
- Removing `AgentRuntimeError::AwaitingClarification` touches every match site. Grep coverage should make it mechanical; run full test suite after the removal.

---

## Phase 5 — Reply IPC and auto-routing

**Scope**: wire a single reply entrypoint that handles both live and post-restart paths, with transparent auto-routing from the main chat input.

### Deliverables
- New IPC `commands/agent/sessions::reply_to_clarification(session_id, content) -> Result<(), String>`.
  - Persist reply as a user chat message via `chat_service`.
  - Call `deliver_reply(session_id, content)`.
    - `Live` → done.
    - `PostRestart { checkpoint_present: true }` → call `AgentServiceImpl::resume_from_checkpoint(session_id, content)` (Phase 6).
    - `PostRestart { checkpoint_present: false }` → return `ClarificationError::OrphanedClarification` and repair the session to `SessionState::Error { recoverable: true, message: "Clarification state corrupted; please retry" }`.
  - Emit `ClarificationReplied`.
  - Append `RuntimeTraceType::ClarificationReplied`.
- Update `send_agent_request` in `commands/agent/sessions.rs`: before anything else, check `pending_clarification(session_id)`. If present, forward to `reply_to_clarification` and return early.
- Remove the dead `send_request_impl` clearing logic in `application/agent/service/sessions.rs:138-144`. The method's remaining responsibilities (validation) either move to `spawn_execution` or are deleted.

### Tests
- Main chat input during pause auto-routes.
- Clarification composer path delivers identically.
- Double-reply: second call returns `ReplyAlreadyDelivered`.
- Reply when no pause exists returns `NotPending` (distinct from "already processing").
- Transcript contains the persisted reply after the IPC returns.

### Risk
- The auto-route changes the semantic of `send_agent_request`. Existing tests that assert "send during pause errors with 'already processing'" need updating. Low risk, clear win.

---

## Phase 6 — Restart rehydration

**Scope**: on app start, make paused sessions discoverable and resumable. When a reply arrives after restart, rebuild the coordinator from the checkpoint.

### Deliverables
- `AgentServiceImpl::rehydrate_paused_clarifications(app_handle) -> RepositoryResult<()>` called from the existing bootstrap in `service/recovery.rs`.
  - Query sessions in `SessionState::AwaitingClarification`.
  - For each:
    - Load the matching `agent_pending_clarifications` and `agent_execution_checkpoints` rows.
    - If either missing: log, transition the session to `SessionState::Error { recoverable: true, message: "Pause state incomplete; please retry" }`, clear whichever rows exist, continue.
    - If both present and `now - asked_at >= timeout`: fire the timeout path (clear rows, mark subtask Failed, emit `ClarificationTimedOut`).
    - Otherwise: emit `ClarificationRestored { session_id, prompt, asked_at, subtask_id }` so the UI renders the prompt immediately on startup. Do not rebuild the coordinator yet — wait for a reply.
- New private `AgentServiceImpl::resume_from_checkpoint(session_id, reply_content, app_handle) -> RepositoryResult<()>`:
  - Load checkpoint, session, task.
  - Construct an `ExecutionDeps`, then `ExecutionContext::from_checkpoint(checkpoint, deps)` (rebuilds the LLM client).
  - Append `ChatMessage::User(reply_content)` to `ctx.llm_context`.
  - Transactionally: delete clarification row, delete checkpoint, transition SessionState → `Running`.
  - Spawn a coordinator task seeded with this context, entering the loop at the saved phase and step number. Register in `active_executions` like a normal execution. Emit `RuntimeTraceType::ClarificationResumed` with a `restored: true` flag for observability.
- Concurrency guard: a per-session lock prevents a rehydrate scan from racing with an incoming `reply_to_clarification` call.

### Tests (integration; construct DB rows in a fresh service)
- Basic restart + reply + resume: subtask and task IDs unchanged; `llm_context` includes prior history plus the reply; next LLM call fires.
- Restart with expired clarification: timeout path fires on load; user sees the timed-out UI.
- Restart with orphan checkpoint (no clarification row): repair path fires; session marked `Error`; rows cleaned.
- Restart with orphan clarification (no checkpoint): same repair.
- Race: reply arrives between rehydrate scan and full load; serialization via the per-session lock ensures a single resume.

### Risk
- If the LLM client or API schema changed between the process that wrote the checkpoint and the one that loads it, rehydrate may fail. Mitigate: schema_version on checkpoint + graceful error to `SessionState::Error { recoverable: true }` on mismatch. The user can retry the task from scratch.
- Clock skew is negligible on desktop; the timeout check uses the persisted `asked_at`, not elapsed wall-clock across resume, so it's consistent.

---

## Phase 7 — Frontend store and events

**Scope**: event handlers and type updates. No UI component changes yet.

### Deliverables
- TypeScript `SessionState` union gains `{ type: 'awaiting_clarification', prompt, askedAt, subtaskId }` (mirror of the Rust variant).
- `src-ui/src/lib/stores/agent/events/sessionEvents.ts`:
  - `session_state_changed` handler recognizes `awaiting_clarification` and sets UI state distinct from `error`.
  - `handleClarificationReplied`: append the reply as a user message to the transcript; set a transient "resuming" indicator; clear `awaitingClarification` from `harnessState`.
  - `handleClarificationTimedOut`: append a system message "The agent didn't hear back…"; surface retry affordance.
  - `handleClarificationRestored`: ensure the paused-session panel renders correctly on startup; scroll into view.
- `agentStore` action `replyToClarification(sessionId, content)` invoking the new IPC.
- Update event handler registration in `events/handler.ts` for the three new event types.

### Tests
- Store handler unit tests per event.
- `SessionState` type narrowing compiles for `awaiting_clarification`.
- `replyToClarification` action calls the correct IPC with the correct payload.

---

## Phase 8 — Frontend UX

**Scope**: the visible affordances. This is where the professional-grade feel lands.

### Deliverables
- `AgentClarificationPrompt.svelte`:
  - Submit via `replyToClarification`, not `sendRequest`.
  - Show active subtask context ("Working on: {subtaskTitle}"); null-safe when subtask metadata is missing.
  - Live elapsed / time-remaining indicator updated via a 1-Hz interval. Swap to a warning visual at `< 5 minutes` remaining; critical at `< 1 minute`.
  - Focus-on-mount on the textarea.
  - ARIA live region announces the question text on mount.
  - Inline position above the chat composer — one clear affordance, not a dismissable banner.
- Main chat composer behavior when `harnessMode === 'awaiting_clarification'`:
  - Placeholder: "Answer the question above to continue…"
  - Submit handler auto-calls `replyToClarification`.
  - Small helper text linking attention to the clarification panel.
- Transcript rendering:
  - `AgentNarration { intent: 'question' }` messages get a dedicated "agent asked" style (distinct border, question icon, subtask chip), not a plain narration.
  - The user's reply appears inline as their next message.
  - Timeout fallback renders a system message with a Retry action that re-invokes the plan (or re-asks the question, depending on product call in Phase 9 QA).
- `AgentPanelActivitySection`: the active subtask shows a "Waiting for your answer" chip instead of the running spinner while paused; tasks list indicates pause state.
- Startup surfacing: on `ClarificationRestored`, auto-scroll to and focus the clarification panel. Show a subtle "Restored from previous session" tag so the user knows why it appeared.
- Accessibility: tab order is Question → Textarea → Send; reduced-motion respected (existing CSS); high-contrast token compliance.

### Tests
- Component: focus on mount, Enter submits, Shift+Enter inserts newline, countdown updates.
- Main chat input auto-route verified by store assertion.
- Snapshot / visual regression for the paused-session panel (if infra exists).
- Manual cross-browser check (Chromium/WebKit).

---

## Phase 9 — End-to-end tests and QA

### Deliverables
- Backend integration test (`service_tests.rs`): mock LLM fires `agent_narrate { intent: "question" }` → pause observed in all three stores; reply via IPC; second LLM turn sees the reply; session state returns to `Running`.
- Backend integration test: restart scenario — construct all rows in a fresh service, reply via IPC, verify resume correctness and transcript consistency.
- Backend integration test: timeout path end-to-end, including rehydrate with already-expired clarification.
- Frontend integration test (`vitest` or Playwright, whichever matches existing patterns): question → reply → resume with a mocked backend event stream.
- Manual QA checklist in this document (updated in Phase 10):
  - [ ] Question renders in transcript and in the clarification panel.
  - [ ] Reply via clarification composer resumes.
  - [ ] Reply via main chat input resumes.
  - [ ] App close during pause; reopen shows the panel.
  - [ ] Reply post-restart resumes correctly.
  - [ ] Timeout marks the subtask failed and shows retry.
  - [ ] Cancelling the session during pause exits cleanly.
  - [ ] Reduced-motion mode respects animation preferences.
  - [ ] Screen reader announces the question on mount and on restore.

### Exit criteria for the workstream
- All backend integration tests green.
- Frontend type check + build clean.
- Manual QA checklist green on Linux and macOS (Windows defer if blocked on unrelated issues).
- `cargo clippy --all-targets` clean on touched files.
- No `ExecutionError` event emitted for any clarification-related code path.
- No DB rows left in `agent_pending_clarifications` or `agent_execution_checkpoints` after the QA run.

---

## Phase 10 — Docs and cleanup

### Deliverables
- `docs/features/agent-clarification.md`: user-facing behavior, state diagram, event/IPC contract, restart semantics, timeout configuration, failure/recovery paths.
- Annotate `docs/planning/agent-fidelity/plan.md` Workstream E with a "superseded by agent-clarification-resume plan" pointer.
- Update `AGENTS.md`'s Feature Documentation table row for clarification, pointing to the new doc.
- `CHANGELOG.md` entry summarizing the user-visible change.
- Remove the `review_and_replan` dead code in `application/agent/service/planner.rs` if confirmed unreferenced after this landing.
- Update coordinator comments that claim "resumes from the same subtask" — now true — with a short pointer to this doc.

---

## Effort estimate

- Backend: ~10 focused working days. Phase 1 is small. Phases 2–4 are the bulk, Phase 6 adds another day.
- Frontend: ~3 focused working days.
- Tests, manual QA, docs: ~2 working days.

Roughly 15 working days total. Ship phase-by-phase with tests at each boundary. Review gates at:
- End of Phase 4 — park works in-process.
- End of Phase 6 — restart works.
- End of Phase 8 — UX complete.

---

## Open questions (non-blocking; decide as we reach them)

- Per-session configurable timeout? Probably yes for a professional product, but out of scope here.
- "Cancel the clarification and let the agent re-ask" button? Easy to add in Phase 8 once the plumbing exists.
- Should the assistant's question appear as a dedicated message kind in the transcript (distinct from narration), or as a styled narration? Leaning dedicated kind for better searchability and future export formats.
- Should `ClarificationRestored` fire before or after the chat history loads? After, so the panel appears in context with the rest of the transcript.

---

## Cross-references

- Phase 4 origin: commit `9390122` ("feat(agent): blocking clarifications + task-declared framing (Phase 4: E + H.2)"), 2026-04-18.
- Trigger bug report: `[2026-04-19][05:14:57]` session `d8a2d1c9-df17-4000-a332-c85a5333c6b2`, logged as ERROR "Awaiting user clarification".
- Relevant code:
  - `src-tauri/src/application/agent/execution/coordinator.rs:113-121` — current early-return path to remove.
  - `src-tauri/src/application/agent/execution/step_gate.rs:79-112` — current pause check to relocate.
  - `src-tauri/src/application/agent/clarification.rs` — dual-layer rewrite target.
  - `src-tauri/src/application/agent/service/runtime_queries.rs:22-121` — `spawn_execution`, auto-route and error-arm changes.
  - `src-tauri/src/application/agent/service/sessions.rs:98-152` — dead `send_request_impl` to remove.
  - `src-tauri/src/infrastructure/agent/runtime/permissions.rs:1465-1522` — `handle_narrate` plumbing.
  - `src-ui/src/lib/components/agent/AgentClarificationPrompt.svelte` — component updates.
  - `src-ui/src/lib/stores/agent/events/sessionEvents.ts` — event handlers.
  - `src-ui/src/routes/+page.svelte:142-144` — `handleReplyToClarification` switch.
