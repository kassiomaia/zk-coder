## 1. Goal
Establish a single, explicit pause/recovery contract so only genuine startup recovery produces recovery UX, while other paused states remain accurate, non-contradictory runtime states.

## 2. Approach
The current code uses `SessionState::Paused` as a coarse bucket for multiple situations, then derives recovery UX from that coarse state. The fix should introduce explicit pause intent/cause metadata in the backend contract, propagate it through Tauri events and frontend types, and make the UI render from that typed intent instead of heuristics and band-aids. This is the smallest change that addresses the visible inconsistency without inventing a second state machine.

## 3. File Changes
- **Modify** `src-tauri/src/core/agent/session.rs:159-220`
  Add typed pause metadata to `SessionState::Paused` so the persisted session state can distinguish startup recovery from clarification timeout and any future user-initiated pause.
- **Modify** `src-tauri/src/infrastructure/agent/events.rs:68-136`
  Extend `AgentEvent::SessionPaused` with the same pause reason/intention fields so startup recovery events and runtime pause events share one explicit contract.
- **Modify** `src-tauri/src/application/agent/service/recovery.rs:65-98`
  Mark crash/startup recovery pauses with the new recovery-specific reason and keep the recovery event emission scoped to this path.
- **Modify** `src-tauri/src/commands/agent/sessions.rs:224-264`
  Emit `SessionPaused` with recovery-specific metadata only for `initialize_agent_recovery`, preserving this command as the sole source of recovery-card events.
- **Modify** `src-tauri/src/application/agent/execution/park.rs:312-348`
  Tag clarification-timeout pauses with a non-recovery reason and keep the UI-facing signal on `ClarificationTimedOut`, not a generic recovery affordance.
- **Modify** `src-tauri/src/application/agent/service/clarification.rs:190-218`
  Apply the same non-recovery pause metadata on rehydrate-timeout repair so restarted clarification timeouts do not masquerade as crash recovery.
- **Modify** `src-tauri/src/application/agent/read_models.rs:186-228`
  Derive `HarnessMode::ResumeVerifier` and the startup-verification summary only for recovery pauses, not for every paused session.
- **Modify** `src-ui/src/lib/types/agent.ts:234,492-503`
  Mirror the new paused-state and `session_paused` event fields in the frontend type contract.
- **Modify** `src-ui/src/lib/stores/agent/events/harnessEvents.ts:38-77`
  Render chat-side recovery cards only when `session_paused` carries a recovery reason; otherwise update panel state without injecting transcript UI.
- **Modify** `src-ui/src/lib/stores/chat.ts:250-290`
  Stop synthesizing a recovery message for every paused session on thread load; gate it on recovery-specific pause metadata.
- **Modify** `src-ui/src/routes/+page.svelte:598-612`
  Remove the yolo-mode suppression band-aid once recovery-card rendering is reason-based instead of heuristic.
- **Modify** `src-ui/src/lib/stores/agent/events/harnessEvents.test.ts:181-303`
  Update/add tests for recovery-card emission only on startup recovery pauses.
- **Modify** `src-ui/src/lib/stores/chat.test.ts`
  Add regression coverage for loading a paused clarification-timeout session without creating a recovery card.
- **Modify** `src-tauri/src/application/agent/read_models.rs:407+` and `src-tauri/src/application/agent/service/recovery.rs:134+`
  Extend existing Rust tests to verify harness derivation and recovery bookkeeping for the new pause reasons.
- **Modify** `docs/features/agent-clarification.md`
  Update the user-visible/runtime contract so timeout pauses are documented separately from startup recovery.
- **Modify** `docs/issues/agent-recovery-banner-in-yolo-and-live-streaming/issue.md`
  Record the actual root contract and mark the issue state once the backend/frontend contract is aligned.
- **Modify** `CHANGELOG.md`
  Note the recovery/paused-state UX fix as part of the shipped agent runtime behavior.

## 4. Implementation Steps
### Task 1: Introduce explicit pause semantics in the backend
1. Update `src-tauri/src/core/agent/session.rs:201-211` to add a pause-reason enum or equivalent typed field on `SessionState::Paused`.
2. Update `src-tauri/src/infrastructure/agent/events.rs:77-90` so `AgentEvent::SessionPaused` carries the same pause reason/intention.
3. Update the startup recovery path in `src-tauri/src/application/agent/service/recovery.rs:65-98` and `src-tauri/src/commands/agent/sessions.rs:240-250` to use the recovery-specific reason.
4. Update clarification-timeout pause writes in `src-tauri/src/application/agent/execution/park.rs:315-348` and `src-tauri/src/application/agent/service/clarification.rs:190-218` to use a non-recovery reason.

### Task 2: Make harness derivation follow the new contract
1. Update `src-tauri/src/application/agent/read_models.rs:186-228` so `HarnessMode::ResumeVerifier`, startup verification, and handoff summaries only derive from startup-recovery pauses.
2. Review any other `SessionState::Paused { .. }` matches surfaced by `rg` in `src-tauri/src/application/agent` and narrow them where behavior should differ by pause reason.
3. Extend the existing tests in `src-tauri/src/application/agent/read_models.rs:407+` and `src-tauri/src/application/agent/service/recovery.rs:134+` to cover both startup recovery and clarification-timeout pauses.

### Task 3: Remove frontend heuristics and render from pause reason
1. Update `src-ui/src/lib/types/agent.ts:234,495-501` to include the new paused-state metadata.
2. Change `src-ui/src/lib/stores/agent/events/harnessEvents.ts:43-77` so only recovery-tagged `session_paused` events create `kind: 'recovery'` transcript messages.
3. Change `src-ui/src/lib/stores/chat.ts:257-290` so thread selection only synthesizes a recovery card when the loaded session state is explicitly a recovery pause.
4. Remove the workaround in `src-ui/src/routes/+page.svelte:598-612`; the route should no longer need to suppress incorrectly-emitted recovery messages.
5. Update `src-ui/src/lib/stores/agent/events/harnessEvents.test.ts:181-303` and `src-ui/src/lib/stores/chat.test.ts` with regressions for clarification timeout, yolo mode, and startup recovery.

### Task 4: Update docs and close the loop
1. Update `docs/features/agent-clarification.md` so timeout behavior and startup recovery are documented as separate flows.
2. Update `docs/issues/agent-recovery-banner-in-yolo-and-live-streaming/issue.md` with the contract-level fix and final status.
3. Add a concise entry to `CHANGELOG.md` describing the corrected recovery-card behavior.

## 5. Acceptance Criteria
- A startup-recovered session emitted by `initialize_agent_recovery` still produces a recovery card and resume-verifier harness state.
- A clarification-timeout session persists as `SessionState::Paused`, but does **not** produce a recovery card on event handling or thread reload.
- `HarnessMode::ResumeVerifier` is derived only for startup recovery pauses; clarification-timeout pauses do not show startup verification copy.
- The workaround in `src-ui/src/routes/+page.svelte:598-612` is removable without reintroducing recovery cards in yolo mode.
- The frontend `AgentEvent`/`SessionState` types match the backend serialized payloads for paused sessions.
- Existing startup recovery behavior remains functional: selecting a thread with a startup-recovered paused session still surfaces a single unresolved recovery card.
- Documentation for clarification timeout and recovery no longer describes them as the same UI flow.

## 6. Verification Steps
- Run Rust-targeted tests that cover the touched logic:
  - `cargo test read_models`
  - `cargo test recovery`
  - `cargo test clarification`
- Run frontend tests for the touched stores/event mappers:
  - `cd src-ui && npm test -- harnessEvents`
  - `cd src-ui && npm test -- chat`
- Run frontend type checking:
  - `cd src-ui && npm run check`
- Manual verification in the app:
  1. Simulate startup recovery and confirm the thread shows one recovery card.
  2. Simulate a clarification timeout and confirm the session is paused but no recovery card is injected.
  3. Repeat with yolo enabled and confirm no special route-level suppression logic is needed.

## 7. Risks & Mitigations
- **Risk:** Adding fields to `SessionState::Paused` can break old deserialization paths.
  **Mitigation:** Preserve backward-compatible serde defaults in `src-tauri/src/core/agent/session.rs` and add coverage for legacy rows/events.
- **Risk:** Some existing `SessionState::Paused { .. }` matches outside the inspected files may still assume every pause is resumable via recovery.
  **Mitigation:** Run a repo-wide search for paused-state matches and audit each call site during implementation.
- **Risk:** Frontend tests may currently encode the old assumption that every `session_paused` means a recovery card.
  **Mitigation:** Update those tests first so the new contract is explicit before touching route/store logic.
- **Risk:** The issue document may be partly stale because the event source changed over time.
  **Mitigation:** Base the implementation on the live code paths above, then rewrite the issue doc to reflect the final verified behavior instead of preserving outdated diagnosis.