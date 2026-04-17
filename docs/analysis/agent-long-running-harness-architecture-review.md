# Agent Long-Running Harness Architecture Review

**Status**: DRAFT
**Last reviewed**: 2026-04-17
**Purpose**: Evaluate whether the current Rust architecture can support Anthropic-style long-running agent harness concepts, identify code-quality risks, and define the refactor direction required to avoid runtime-module sprawl

---

## Summary

The current Rust architecture is capable of supporting long-running harness concepts such as initializer sessions, durable handoff artifacts, startup verification, incremental progress, and clean-state exits. The core reason is that the backend already has the right high-level seams:

- persistent `AgentSession` lifecycle state
- persistent tasks, subtasks, traces, snapshots, tool calls, and terminal output
- a layered split between core contracts, application orchestration, commands, and infrastructure
- a coordinator-driven execution loop rather than prompt-only control

However, the architecture is only **structurally ready**, not **harness-ready**. The missing pieces are not low-level primitives; they are first-class domain concepts and cleaner application/runtime module boundaries.

The biggest risk is not feasibility. The biggest risk is that implementing long-running harness behavior directly inside the existing oversized runtime files will create hard-to-maintain orchestration code and push the backend further into prompt-driven spaghetti.

The frontend is in a similar position. It is already capable of rendering rich execution state, but it is still optimized for transient execution updates rather than durable session handoff, startup verification, and clean-exit communication. If the backend adds long-running harness concepts without a matching frontend contract, the UX will collapse those concepts back into generic chat cards and status text.

Primary external clarification sources:

- Manus: "Context Engineering for AI Agents: Lessons from Building Manus" (2025-07-18)
- Anthropic: "Effective harnesses for long-running agents" (2025-11-26)

Primary internal clarification source:

- the current agent runtime and persistence code in `src-tauri/src/application/agent/` and `src-tauri/src/infrastructure/agent/`

---

## Architecture Fit Assessment

### Supported Cleanly

These concepts fit the existing architecture without a structural rewrite:

- **Initializer vs worker harness roles**
  - `AgentRegistry` already owns agent definitions and system prompts.
  - `AgentSession` already persists `agent_id`.
  - Adding a second built-in agent or a typed harness mode is structurally straightforward.

- **Fresh-context resume across sessions**
  - Sessions already persist independently of a live Tokio task.
  - Recovery already demotes stale runs to `Paused`.
  - Continue / retry / replan commands already exist.

- **Startup verification phase**
  - The coordinator is phase-driven.
  - A startup or reorientation phase can be inserted before normal execution without violating the current layering.

- **Durable handoff artifacts**
  - Persistence, tool access, runtime traces, and workspace/app-data boundaries already exist.
  - The system can store and retrieve handoff artifacts; it just does not model them explicitly yet.

### Supported With Extension

These concepts fit, but require new domain/application abstractions:

- **Session handoff artifact set**
  - progress log
  - feature/work ledger
  - startup/bootstrap instructions
  - exit summary / health marker

- **Long-horizon work ledger**
  - current `Task` / `SubTask` types are execution-oriented, not project-ledger-oriented
  - a new durable work-item model should exist instead of overloading subtasks

- **Clean-state session exits**
  - current states can represent `Paused`, `Finished`, and `Error`
  - but there is no explicit “handoff is clean” contract or persisted exit record

- **Harness policy layer**
  - current prompt text carries too much behavioral policy
  - Anthropic-style harnesses need runtime-owned policy, not only instructions in the system prompt

### Weak / Needs Refactor First

These concepts are feasible, but current module shape makes them risky to implement directly:

- **Context policy growth**
  - `infrastructure/agent/runtime/context.rs` is already 1321 lines
  - adding working-memory references, startup context, execution briefs, and richer failure memory directly into this file would worsen a file that already mixes token estimation, compaction policy, extraction heuristics, and tests

- **Permission and internal tool orchestration growth**
  - `infrastructure/agent/runtime/permissions.rs` is already 2162 lines
  - it currently mixes permission routing, approval handling, internal agent tool behavior, subtask mutation, rationale tracking, decomposition heuristics, and event emission
  - adding harness-only control behavior here would further collapse boundaries

- **Termination and completion growth**
  - `application/agent/execution/termination.rs` is already 1167 lines
  - it already owns streaming events, completion branching, subtask auto-completion, and finalization decisions
  - clean-state exits and handoff summaries should not be piled directly into this module

- **Planner growth**
  - `application/agent/service/planner.rs` is already 1089 lines
  - mixing JSON cleanup, prompt building, task planning, replanning, and conversion logic in one file leaves little room to add initializer planning or feature-ledger generation cleanly

---

## Frontend Workflow Fit Assessment

The current frontend is structurally compatible with long-running harness concepts, but only if those concepts become typed session data rather than more chat-only messaging.

### Supported Cleanly

- **Session-oriented execution UI already exists**
  - [AgentPanel.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/agent/AgentPanel.svelte) already acts as a dedicated execution surface rather than relying only on the chat transcript.
  - [sessionEvents.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/agent/events/sessionEvents.ts) already centralizes event-driven session updates.

- **Recovery flow already exists**
  - [chat.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/chat.ts) reloads a paused session when selecting a thread.
  - [RecoveryMessage.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/chat/messages/RecoveryMessage.svelte) already gives the user a continue vs replan decision.

- **User-facing execution status pipeline already exists**
  - [agentStatus.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/utils/agentStatus.ts) derives a single current status from session state, subtask, and step activity.
  - [AgentPanelHeader.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/agent/AgentPanelHeader.svelte) and the chat answering indicator already consume that status.

### Supported With Extension

- **Handoff summary UI**
  - The panel can host a persistent handoff section, but current types do not expose handoff artifact summaries or exit-health metadata.

- **Startup verification UI**
  - The status and panel layers can surface startup checks, but current session states do not distinguish “resuming,” “verifying environment,” or “resume blocked.”

- **Harness mode awareness**
  - The UI can reflect initializer vs worker behavior, but [types/agent.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/types/agent.ts) has no typed place for harness mode or related artifacts.

### Weak / Needs Refactor First

- **Recovery is too chat-card-centric**
  - [RecoveryMessage.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/chat/messages/RecoveryMessage.svelte) only offers `Continue` and `Replan`.
  - It does not show what will be resumed, what was last completed, whether startup verification passed, or why replanning might be safer.

- **Agent panel is execution-detail oriented, not handoff oriented**
  - [AgentPanel.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/agent/AgentPanel.svelte) shows tasks, requests, files, and tool calls.
  - It does not currently reserve a durable place for:
    - current objective
    - startup checklist result
    - handoff summary
    - next recommended action

- **Status model is too coarse for long-running sessions**
  - [agentStatus.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/utils/agentStatus.ts) collapses many states into generic strings such as `Thinking...`, `Paused`, or `Task completed`.
  - Long-running harnesses need explicit distinctions among:
    - actively executing
    - awaiting approval
    - resuming from handoff
    - verifying environment
    - blocked on failed verification
    - handed off in clean state

- **Session event model is missing harness-specific events**
  - [sessionEvents.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/agent/events/sessionEvents.ts) handles execution, approval, recovery, and completion well.
  - It has no typed events for startup verification, handoff update, or clean-exit state.

---

## Frontend Findings

These are the highest-value frontend workflow findings to carry into the implementation plan.

### F7 — Recovery UX is underpowered for long-running sessions

**Severity**: High

**Evidence**:

- [RecoveryMessage.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/chat/messages/RecoveryMessage.svelte)
- [chat.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/chat.ts)

The current recovery card is sufficient for “the app restarted; do you want to continue?” It is not sufficient for Anthropic-style long-running sessions where the user may need to understand:

- what the agent last finished
- what remains
- whether the environment was re-verified
- what continuing will actually do

**Refactor direction**:

- keep the chat card as an entry point
- move durable recovery context into session state and the agent panel
- let the card summarize, not carry, the entire handoff experience

### F8 — Frontend contracts do not model harness concepts yet

**Severity**: High

**Evidence**:

- [types/agent.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/types/agent.ts)

The frontend type model has no first-class representation for:

- harness mode
- startup verification status
- handoff summary
- clean-exit health
- durable work ledger summary

Without these contracts, any backend harness work will leak into generic message content and brittle UI conditionals.

**Refactor direction**:

- add explicit session-facing types for harness concepts
- keep them separate from low-level execution-step telemetry

### F9 — The main execution panel is missing a durable “session story” section

**Severity**: Medium

**Evidence**:

- [AgentPanel.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/agent/AgentPanel.svelte)
- [AgentPanelHeader.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/agent/AgentPanelHeader.svelte)

The panel is currently good at explaining what happened in this run. It is weak at explaining:

- where the session stands across windows
- whether the workspace is healthy
- what the next increment should be

This is the correct place to surface handoff and startup-verification summaries because it is already the authoritative execution surface.

**Refactor direction**:

- add a compact session-summary / handoff section near the top of the panel
- keep requests, files, and tool calls below it as supporting detail

### F10 — Status copy and visual states are too generic for harness transitions

**Severity**: Medium

**Evidence**:

- [agentStatus.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/utils/agentStatus.ts)
- [AgentPanelHeader.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/agent/AgentPanelHeader.svelte)

The existing status model is good for short execution bursts. It is too generic for long-running workflows, where the user must distinguish “still working” from “re-establishing context” and “safe to resume” from “needs attention.”

**Refactor direction**:

- expand derived status to include harness-specific phases
- keep copy deterministic and state-driven rather than inferred from loose message text

---

## Frontend Refactor Direction

The frontend can support the harness concepts cleanly, but only if the new UI work is organized around current ownership boundaries instead of spreading through chat rendering, panel rendering, and store side effects at once.

### Recommended module boundaries

#### 1. Keep transcript ownership and harness ownership separate

- [chat.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/chat.ts) should stay responsible for:
  - loading thread messages
  - selecting the active thread
  - injecting lightweight transcript-level recovery affordances
- It should not become the owner of:
  - startup verification state
  - durable handoff summaries
  - long-running session health

If those concepts live in `chat.ts`, the communication flow will become harder to reason about and more brittle under thread switching.

#### 2. Keep agent event mapping explicit and typed

- [sessionEvents.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/agent/events/sessionEvents.ts) is the right place to translate backend events into frontend session state.
- If harness transitions are added, prefer extracting helpers such as:
  - `harnessEvents.ts`
  - `sessionStateMappers.ts`
- Do not add free-form payload interpretation to components.

#### 3. Keep status derivation pure

- [agentStatus.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/utils/agentStatus.ts) should remain a pure derivation layer.
- It should read typed session state and return deterministic display state.
- It should not inspect raw chat messages or reconstruct state from event history.

#### 4. Split the agent panel before it becomes the next monolith

- [AgentPanel.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/agent/AgentPanel.svelte) is currently manageable, but it is the most likely frontend file to become overloaded by this work.
- If handoff and startup-verification UI is added, extract a focused child component such as:
  - `AgentSessionSummarySection.svelte`
  - `AgentStartupVerificationSection.svelte`
- Do this before adding another large conditional block to the top-level panel.

#### 5. Keep recovery cards lightweight

- [RecoveryMessage.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/chat/messages/RecoveryMessage.svelte) should remain a lightweight action surface.
- The durable explanation of the session should live in session state and panel UI, not inside a richer and richer chat card.

### Suggested implementation order for frontend work

1. Extend typed contracts in [types/agent.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/types/agent.ts)
2. Update store mapping in [sessionEvents.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/agent/events/sessionEvents.ts)
3. Expand derived status in [agentStatus.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/utils/agentStatus.ts)
4. Add session-summary UI in the agent panel with extraction if needed
5. Keep [RecoveryMessage.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/chat/messages/RecoveryMessage.svelte) as a trigger/supplement, not the primary session view

This sequence keeps the frontend aligned with the backend contract and reduces the chance of view-driven state modeling.

---

## Code Quality Findings

These are the highest-value findings from the architecture and code-quality review, ordered by severity.

### F1 — `permissions.rs` is already a multi-responsibility orchestration hub

**Severity**: High

**Evidence**: [permissions.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/infrastructure/agent/runtime/permissions.rs)

The file currently combines:

- permission routing
- auto-run vs approval decisions
- internal agent tool behavior
- subtask mutation
- status parsing and validation
- subtask rationale tracking
- auto-decomposition of failed subtasks
- file-event emission

This is the wrong place to add harness-specific startup, handoff, or session-discipline logic. Doing so would turn one large file into the de facto hidden application layer.

**Refactor direction**:

- split into submodules by ownership:
  - `approval_flow`
  - `tool_execution`
  - `internal_agent_tools`
  - `subtask_control`
  - `file_events`
  - `failure_recovery`

### F2 — `context.rs` mixes policy, extraction, budgeting, and summarization heuristics

**Severity**: High

**Evidence**: [context.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/infrastructure/agent/runtime/context.rs)

This file already owns:

- token estimation
- provider-window heuristics
- compaction policy
- task/subtask extraction from context
- file relevance packing
- failure-memory construction
- recent-tail collection

Anthropic-style startup context, handoff artifacts, and execution briefs belong in a higher-level context assembly boundary, not as additional ad hoc blocks in this file.

**Refactor direction**:

- split into:
  - `budget.rs`
  - `packing.rs`
  - `extractors.rs`
  - `failure_memory.rs`
  - `memory_blocks.rs`

### F3 — `termination.rs` owns too many unrelated completion concerns

**Severity**: High

**Evidence**: [termination.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/application/agent/execution/termination.rs)

The module currently mixes:

- session-state transitions
- cancellation/failure paths
- streaming chunk generation
- completion branching
- subtask auto-completion
- final emission behavior

Long-running harness concepts need explicit session exit summaries and handoff finalization. That should be a separate concern, not another section added here.

**Refactor direction**:

- split into:
  - `session_finish.rs`
  - `streaming_emit.rs`
  - `subtask_completion.rs`
  - `failure_paths.rs`

### F4 — `planner.rs` is too large to absorb initializer and ledger planning safely

**Severity**: Medium

**Evidence**: [planner.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/application/agent/service/planner.rs)

Initializer-session behavior from Anthropic’s harness pattern would naturally require:

- feature-ledger generation
- harness bootstrap instructions
- progress artifact initialization
- replanning rules

The current file is already oversized and functionally broad.

**Refactor direction**:

- split into:
  - `prompt_builders.rs`
  - `json_normalization.rs`
  - `task_planning.rs`
  - `replanning.rs`
  - `conversions.rs`

### F5 — Resume semantics are request-driven, not harness-artifact-driven

**Severity**: Medium

**Evidence**: [retry.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/application/agent/service/retry.rs)

`continue_execution()` and `replan_and_continue()` currently reconstruct the next run from the last request content. That is sufficient for current retry semantics, but weak for Anthropic-style fresh-window continuation, which depends on durable handoff state and startup verification rather than only “rerun the last request.”

**Refactor direction**:

- add a session-handoff service that prepares resume context from persisted artifacts
- keep retry logic thin and delegate resume preparation to that service

### F6 — `ExecutionContext` is execution-loop state, not harness state

**Severity**: Medium

**Evidence**: [state.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/application/agent/execution/state.rs)

`ExecutionContext` is currently appropriate for one runtime loop, but it has no place for:

- startup protocol status
- handoff artifact references
- harness mode
- clean-exit assessment

That is good news architecturally: the type is still focused. But it means long-running harness support should not be shoved into miscellaneous optional fields.

**Refactor direction**:

- keep `ExecutionContext` focused on in-loop state
- introduce a separate `HarnessContext` or `SessionRunContext` feeding bootstrap/coordinator

---

## Recommended Architecture Additions

These additions match Anthropic’s harness concepts while respecting current layers.

### 1. Add a typed harness mode

Add a typed concept such as:

- `HarnessMode::Initializer`
- `HarnessMode::Worker`
- `HarnessMode::ResumeVerifier`

Likely ownership:

- core: enum/value object
- session persistence: stored on `AgentSession` or runtime-owned session metadata
- application: bootstrap/coordinator branching

Do not implement this as hidden prompt text only.

### 2. Add session handoff artifacts as first-class domain objects

Add a small durable model for:

- progress log
- feature/work ledger
- startup instructions
- exit summary

Likely ownership:

- core: typed artifact metadata / contracts
- application: handoff orchestration service
- infrastructure: SQLite or artifact-store persistence

### 3. Add a startup verification phase

Introduce a coordinator-visible phase before normal execution for:

- reading handoff artifacts
- checking workspace / app state
- running a lightweight baseline validation
- deciding whether to continue, repair, or replan

This is cleaner than treating “get bearings” as free-form prompt behavior.

### 4. Add a clean-exit handoff writer

At the end of a session, write:

- what was changed
- what remains
- current health status
- recommended next action

This should be generated by a dedicated application service, not buried in completion streaming.

### 5. Add a separate durable work ledger

Do not overload `SubTask` to mean both:

- execution unit within one run
- long-running project feature state across many runs

Introduce a distinct concept for durable feature/work tracking.

---

## Rust Refactor Plan

This is the refactor sequence I would use before or alongside harness work.

### Phase R1 — Extract runtime context policy package

**Targets**:

- [context.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/infrastructure/agent/runtime/context.rs)

**Split into**:

- `budget.rs`
- `packing.rs`
- `extractors.rs`
- `failure_memory.rs`
- `tail.rs`

**Goal**:

Make room for handoff-memory and execution-brief blocks without growing one monolithic context-policy file.

### Phase R2 — Extract internal agent tool orchestration from permissions

**Targets**:

- [permissions.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/infrastructure/agent/runtime/permissions.rs)

**Split into**:

- `approval.rs`
- `execution.rs`
- `internal_tools/mod.rs`
- `internal_tools/subtasks.rs`
- `internal_tools/narration.rs`
- `internal_tools/planning.rs`
- `recovery.rs`
- `events.rs`

**Goal**:

Keep permission policy separate from agent-control behavior.

### Phase R3 — Extract completion and streaming concerns

**Targets**:

- [termination.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/application/agent/execution/termination.rs)

**Split into**:

- `session_finish.rs`
- `streaming_emit.rs`
- `subtask_completion.rs`
- `failure_paths.rs`

**Goal**:

Reserve clean ownership boundaries for future handoff finalization and clean-state exits.

### Phase R4 — Split planner into prompt, parsing, and orchestration

**Targets**:

- [planner.rs](/home/kassiomaia/devel/zk/coder/src-tauri/src/application/agent/service/planner.rs)

**Split into**:

- `prompt_builders.rs`
- `json_normalization.rs`
- `task_planning.rs`
- `replanning.rs`
- `conversions.rs`

**Goal**:

Avoid turning planning into the dumping ground for initializer-session logic and feature-ledger generation.

### Phase R5 — Add a harness package in the application layer

**New module family**:

- `src-tauri/src/application/agent/harness/`

**Suggested contents**:

- `mode.rs`
- `startup.rs`
- `handoff.rs`
- `feature_ledger.rs`
- `exit_gate.rs`

**Goal**:

Put Anthropic-style long-running harness logic in an application-owned package instead of smearing it across runtime prompt builders, retry logic, and completion code.

---

## Recommended Implementation Order

To avoid building long-running harness behavior on top of unstable module boundaries:

1. R2 — split `permissions.rs`
2. R1 — split `context.rs`
3. R3 — split `termination.rs`
4. R4 — split `planner.rs`
5. Add `application/agent/harness/`
6. Implement initializer/worker modes
7. Implement handoff artifacts and startup verification

This order reduces the chance that harness features get buried inside already-large files.

---

## Conclusion

The Rust architecture is fundamentally compatible with Anthropic-style long-running harnesses.

The backend already has the critical primitives:

- persistent sessions
- durable runtime evidence
- resumable execution semantics
- layered orchestration
- rollback and recovery

What it lacks is a first-class harness model and the module shape needed to add one cleanly.

The correct move is not to keep extending prompts and giant runtime files. The correct move is:

- add a dedicated harness layer
- add explicit handoff artifacts
- keep execution-loop state separate from long-running session state
- refactor the largest runtime files before piling more behavior into them
