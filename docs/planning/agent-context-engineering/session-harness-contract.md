# Session Harness Contract

**Status**: DRAFT
**Last reviewed**: 2026-04-17
**Purpose**: Define the concrete backend/frontend contract changes needed to expose long-running harness state without overloading existing execution and chat session models

---

## Summary

The current agent contract is execution-centric:

- Rust session state is represented by [session.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/core/agent/session.rs)
- frontend event payloads are driven by [events.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/infrastructure/agent/events.rs)
- full session IPC reads are returned by [history.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/commands/agent/history.rs)
- frontend session typing is concentrated in [agent.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/types/agent.ts)

That contract is good for execution steps, approvals, tools, files, and basic recovery. It is not yet sufficient for Anthropic-style long-running harness behavior because there is no typed place for:

- harness mode
- startup verification state
- handoff summary
- clean-exit summary
- environment health

The goal of this contract is to add those concepts as **session-facing metadata**, not to mutate execution-step data or overload chat messages.

---

## Current Contract Surface

### Backend session model

Current persisted session shape in [session.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/core/agent/session.rs):

- `id`
- `agent_id`
- `agent_name`
- `conversation_id`
- `thread_id`
- `state`
- `workspace_root`
- `parent_session_id`
- `model_id`
- `created_at`
- `updated_at`

### Backend event model

Current live-update surface in [events.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/infrastructure/agent/events.rs):

- `session_created`
- `session_state_changed`
- `session_paused`
- `recovery_continue`
- `recovery_replan`
- `execution_started`
- `execution_step`
- `execution_complete`
- `execution_error`
- approval / tool / task / subtask / file / streaming events

### Backend session details IPC

Current `get_session_details` response in [history.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/commands/agent/history.rs):

- flattened `AgentSession`
- `filesTouched`
- `terminalOutput`
- `executionSteps`
- `statusPhrases`
- `tasks`
- `toolCalls`

### Frontend session model

Current frontend `AgentSession` in [agent.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/types/agent.ts):

- flattened backend session data
- plus runtime detail arrays for files, terminal, tool calls, execution steps, artifacts, status phrases

This model currently mixes:

- persistent session identity
- execution-loop telemetry
- ad hoc UI support fields

That is acceptable today, but it will get messy if harness metadata is added without structure.

---

## Contract Design Principles

1. **Do not overload `SessionState` with harness-only detail**
   - `SessionState` should remain the coarse execution lifecycle.
   - Do not add every startup/handoff nuance as a new lifecycle enum variant unless it truly changes execution semantics.

2. **Add harness state beside the session, not inside execution steps**
   - harness metadata should be session-facing summary state
   - execution steps remain low-level telemetry

3. **Prefer additive compatibility**
   - existing event handlers and panel views should keep working while new UI surfaces are added
   - use additive fields and events first

4. **Keep typed session metadata authoritative**
   - chat messages may summarize recovery or handoff
   - they must not be the only source of truth

---

## Proposed Additions

### A. New session-facing harness payload

Add a new optional session-facing block to the detailed session response and in-memory frontend session model:

```ts
type HarnessMode = 'initializer' | 'worker' | 'resume_verifier';

type StartupVerificationState = 'idle' | 'running' | 'passed' | 'failed';

type EnvironmentHealth = 'unknown' | 'healthy' | 'degraded' | 'blocked';

interface StartupVerificationSummary {
  state: StartupVerificationState;
  headline: string;
  details?: string[];
  checkedAt?: string;
}

interface SessionHandoffSummary {
  lastCompletedIncrement: string;
  currentObjective: string;
  nextRecommendedAction: string;
  openIssues: string[];
  environmentHealth: EnvironmentHealth;
  recordedAt?: string;
}

interface AgentSessionHarnessState {
  mode: HarnessMode;
  startupVerification?: StartupVerificationSummary;
  handoff?: SessionHandoffSummary;
  cleanExitRecorded: boolean;
}
```

### B. Additive IPC response shape

Recommended additive response field in [history.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/commands/agent/history.rs):

- `harnessState?: AgentSessionHarnessState`

This should appear in:

- `get_session_details`
- optionally `get_agent_session` if the panel/store benefits from lighter access to current harness summary

### C. Additive frontend session type

Recommended additive field in [agent.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/types/agent.ts):

- `harnessState?: AgentSessionHarnessState`

This keeps harness metadata grouped and prevents new top-level session fields from leaking across unrelated UI code.

---

## Proposed Event Additions

Do not replace `session_state_changed`. Keep it as the coarse lifecycle event.

Add narrowly-scoped harness events only where live updates are required beyond normal session reloads.

### Recommended new events

1. `harness_state_updated`

Purpose:

- session-level additive update for mode, handoff, clean-exit marker, or startup verification summary

Shape:

```json
{
  "type": "harness_state_updated",
  "sessionId": "session-123",
  "harnessState": {
    "mode": "worker",
    "startupVerification": {
      "state": "passed",
      "headline": "Workspace check passed"
    },
    "handoff": {
      "lastCompletedIncrement": "Stabilized tool catalog ordering",
      "currentObjective": "Expose startup verification in session state",
      "nextRecommendedAction": "Resume worker flow and validate panel state",
      "openIssues": [],
      "environmentHealth": "healthy"
    },
    "cleanExitRecorded": false
  }
}
```

2. `startup_verification_started`

Purpose:

- lets the UI show a distinct “verifying environment” phase without waiting for a full refresh

Shape:

- `sessionId`

This event is optional if `session_state_changed` plus `harness_state_updated` can express the same transition clearly enough.

3. `startup_verification_finished`

Purpose:

- communicate pass/fail plus summary details for resumed sessions

Shape:

- `sessionId`
- `summary: StartupVerificationSummary`

This may be redundant if every completion immediately emits `harness_state_updated`. Prefer fewer events if one additive event is sufficient.

### Event recommendation

The minimal clean version is:

- add only `harness_state_updated`
- use it for startup verification updates and handoff summary updates

That keeps event fan-out smaller and preserves the current event model.

---

## Rust Ownership Recommendation

### Core layer

Add typed harness summary objects in `src-tauri/src/core/agent/` only if they are persisted domain state, for example:

- `HarnessMode`
- `StartupVerificationSummary`
- `SessionHandoffSummary`
- `AgentSessionHarnessState`

If the first implementation stores them only as application/runtime summaries, they may temporarily live in an application read-model layer instead.

### Application layer

Own:

- preparing `AgentSessionHarnessState`
- merging startup verification and handoff artifacts into a session-facing read model
- deciding when `harness_state_updated` should be emitted

Recommended home:

- `src-tauri/src/application/agent/harness/`

### Commands/read-model layer

Own:

- exposing `harnessState` in IPC DTOs
- preserving backward compatibility for current frontend consumers

---

## Frontend Ownership Recommendation

### `$lib/types`

Own canonical harness types in:

- [agent.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/types/agent.ts)

Do not recreate the same shape in stores or components.

### `$lib/stores/agent/events`

Own:

- mapping `harness_state_updated` into `session.harnessState`

Recommended extraction if needed:

- `harnessEvents.ts`

### `$lib/utils`

Own:

- pure status derivation from `session.state` plus `session.harnessState`

Recommended touched file:

- [agentStatus.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/utils/agentStatus.ts)

### `$lib/components/agent`

Own:

- persistent session-summary / handoff rendering

Recommended extracted component:

- `AgentSessionSummarySection.svelte`

### `$lib/components/chat/messages`

Own:

- lightweight recovery card only

Do not move full handoff rendering into the chat layer.

---

## Compatibility Strategy

### Phase 1

- add `harnessState` as an optional field in detailed session reads
- no existing UI should break if the field is absent

### Phase 2

- add `harness_state_updated`
- update session event mapping in the agent store

### Phase 3

- update panel and status derivation to consume typed harness state
- keep existing recovery card functioning during transition

### Phase 4

- once session-summary UI is stable, reduce any duplicate explanatory text in recovery chat cards if it becomes noisy

---

## First Implementation Slice Recommendation

The first contract-focused implementation slice should do only this:

1. define frontend harness types
2. add optional `harnessState` to session IPC DTOs
3. add store mapping for `harness_state_updated`
4. add tests for absent vs present `harnessState`

Do not mix this with:

- prompt refactors
- context packing changes
- panel redesign
- recovery card redesign

That keeps Slice 8 reviewable and gives later UI work a stable target.
