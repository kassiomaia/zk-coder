# Agent Context Engineering Improvements — Execution Plan

**Status**: PLAN
**Last reviewed**: 2026-04-17
**Purpose**: Progressive implementation plan for improving agent context engineering, long-running harness behavior, and the frontend workflow required to communicate those runtime states clearly

---

## Summary

This plan upgrades the backend agent runtime to follow stronger context-engineering practices without destabilizing the current agent architecture. The work is intentionally progressive: each milestone delivers a concrete runtime improvement, preserves observability, and leaves the system in a deployable state.

This plan is grounded in three clarification sources that must be used throughout execution:

1. **External source**: Manus blog post, "Context Engineering for AI Agents: Lessons from Building Manus" (2025-07-18), used as a design input for cache stability, tool-state control, external memory, recitation, and error retention.
2. **External source**: Anthropic engineering post, "Effective harnesses for long-running agents" (2025-11-26), used as a design input for initializer-vs-worker harness design, session handoff artifacts, startup verification, incremental progress discipline, and clean-state exits.
3. **Internal source**: runtime feedback from this repository during implementation, including traces, token usage, tool-call telemetry, regression tests, and targeted manual agent runs. When blogpost advice conflicts with measured runtime behavior here, measured runtime behavior wins and the deviation must be recorded in the touched doc or PR notes.

This is a backend-first plan, but not a backend-only plan. Frontend workflow work is in scope where it is required to expose long-running harness behavior cleanly and avoid collapsing new backend concepts back into generic chat messages.

Supporting execution docs:

- [Session harness contract](/home/kassiomaia/devel/zk/coder/docs/planning/agent-context-engineering/session-harness-contract.md)

---

## Clarification Sources During Execution

Use these sources in this order when implementation details are unclear:

1. Current backend behavior in:
   - `src-tauri/src/application/agent/execution/`
   - `src-tauri/src/infrastructure/agent/runtime/`
   - `src-tauri/src/infrastructure/llm/`
   - `src-tauri/src/infrastructure/tools/`
2. This plan
3. The Manus article for context-engineering design intent and tradeoff framing
4. The Anthropic article for long-running harness and session-handoff design intent
5. Runtime feedback loop generated during execution:
   - request/response traces
   - prompt/tool hashes
   - token usage deltas
   - loop-detection and failure traces
   - characterization and regression tests

If implementation reveals a conflict between current code and this plan, update the plan before continuing with the next milestone. Do not leave the plan stale.

---

## Core Rules

1. **One stable source of truth**. This file is the execution source of truth. If a task is not listed here, it is not in scope.
2. **Each milestone must be shippable**. No milestone may require an incomplete future milestone to be safe.
3. **Test-first for behavior changes**. Characterization or regression tests are written before restructuring runtime behavior.
4. **Determinism over cleverness**. Do not introduce random prompt or serialization variance as a substitute for state control.
5. **Runtime feedback is mandatory**. Each milestone adds or uses instrumentation to validate whether the change actually improves behavior.
6. **Blogposts are advisory, not authoritative**. The Manus and Anthropic articles are design inputs, not repository contracts. Measured results in this codebase are the tie-breaker.

---

## Milestone 1 — Stable Prompt Prefix And Tool Catalog

**Ship this**: the runtime sends a deterministic tool catalog and reduces avoidable prompt-prefix churn between steps.

### M1.1 — Canonicalize tool definition ordering

- **Deliverable**: tool definitions are returned in stable order across runs and processes.
- **Build**:
  - replace implicit `HashMap::values()` iteration ordering with canonical sorting by tool name
  - preserve the same sorted order in every filtered or copied tool-definition list
  - add a helper for producing stable tool-definition vectors so call sites do not reimplement ordering
- **Tests (first)**:
  - registry definitions are sorted deterministically
  - repeated registry construction yields identical tool-definition ordering
  - filtered subsets preserve canonical order
- **Done when**: tool-order snapshots are stable across repeated test runs.

### M1.2 — Add request-shape stability telemetry

- **Deliverable**: each LLM step records enough metadata to explain prompt-prefix churn.
- **Build**:
  - add request telemetry fields for:
    - system prompt hash
    - tool catalog hash
    - message prefix hash
    - context token estimate before call
    - tool count
  - emit step-to-step deltas in runtime traces where practical
- **Tests (first)**:
  - trace payload includes the new hashes
  - identical prepared requests produce identical hashes
  - tool-catalog changes produce a different catalog hash
- **Done when**: a trace consumer can answer why two adjacent requests differ without reconstructing the full prompt manually.

### M1.3 — Stop ad hoc prefix churn in prompt assembly

- **Deliverable**: system prompt and tool serialization are stable except when a real state change occurs.
- **Build**:
  - audit prompt assembly for avoidable dynamic fields
  - keep derived prompt text deterministic
  - explicitly document which runtime transitions are allowed to change request shape
- **Tests (first)**:
  - identical session state produces identical prepared request hashes
  - subtask transition changes only the expected request components
- **Done when**: request-shape changes are intentional and traceable.

### M1 exit gate

M1 is Done when all of the following are true:

- tool-definition ordering is deterministic
- trace payloads expose prompt-shape hashes
- a repeated no-op execution path produces identical request-shape telemetry across adjacent steps

---

## Milestone 2 — Tool State Control Without Prompt Churn

**Ship this**: the runtime stops treating tool availability as “remove tools from the prompt every step” and introduces explicit tool-choice state.

### M2.1 — Introduce runtime tool-choice modes

- **Deliverable**: the backend can express `none`, `auto`, and `required` tool-choice intent at request time.
- **Build**:
  - extend the internal request model to carry tool-choice mode
  - update provider adapters for OpenAI and Anthropic to map the new mode
  - keep compatibility-safe fallback behavior where provider support is incomplete
- **Tests (first)**:
  - provider request conversion covers each tool-choice mode
  - final-summary path sends no tools
  - tool-required paths emit provider payloads that reflect required mode
- **Done when**: tool-choice behavior is a runtime decision, not hard-coded to `auto`.

### M2.2 — Decouple prompt catalog from subtask permission filtering

- **Deliverable**: session-level tool catalog remains stable while execution-time permissions still enforce subtask boundaries.
- **Build**:
  - stop deriving per-step tool catalogs directly from `SubTaskMode::allowed_tools()`
  - keep a stable session-visible tool catalog
  - enforce disallowed-tool calls at runtime with explicit tool-result errors and traces
  - if needed, retain a narrow filtered path only for final-summary or no-tool phases
- **Tests (first)**:
  - a subtask transition no longer changes the tool catalog hash when the session catalog is unchanged
  - disallowed tool call under current subtask mode is rejected cleanly at execution time
  - loop detection and approval flow still behave correctly after the change
- **Done when**: subtask changes do not churn the tool catalog unless the session-level catalog itself changes.

### M2.3 — Add state-driven action forcing for known runtime states

- **Deliverable**: the runtime can force “respond only” or “tool required” behavior where the agent currently drifts.
- **Build**:
  - final-summary path uses no-tool mode
  - narration-loop recovery uses required-tool mode where provider support allows it
  - leave explicit traces showing when the runtime forced a choice mode
- **Tests (first)**:
  - final-summary step cannot request tools
  - narration-loop recovery flips request mode before failing execution
- **Done when**: runtime state, not only prompt wording, controls whether the model can or must call tools.

### M2 exit gate

M2 is Done when all of the following are true:

- provider adapters support internal tool-choice modes
- subtask changes do not mutate the visible tool catalog unnecessarily
- forced no-tool and required-tool paths are covered by tests and traces

---

## Milestone 3 — Externalized Working Memory

**Ship this**: long-running tasks can offload durable working state into files or artifacts instead of forcing the prompt to carry everything.

### M3.1 — Define session working-memory artifact

- **Deliverable**: a first-class runtime-owned working-memory artifact exists for agent sessions.
- **Build**:
  - choose one runtime-owned memory representation:
    - workspace file under session-scoped path, or
    - app-data artifact linked to the session
  - define what belongs there:
    - task goal
    - active plan
    - completed findings
    - open questions
    - high-value extracted facts
  - keep ownership in backend orchestration, not ad hoc agent behavior
- **Tests (first)**:
  - memory artifact is created when a task crosses the chosen complexity threshold
  - repeated updates are deterministic and idempotent
  - cleanup or retention behavior is explicit
- **Done when**: external memory exists as a deliberate backend concept rather than an optional agent habit.

### M3.2 — Teach context policy to reference memory instead of carrying full state

- **Deliverable**: compacted context references the working-memory artifact and only inlines short summaries.
- **Build**:
  - add memory reference block to context policy
  - keep summaries reversible where possible
  - prefer path or artifact identifiers over copying large text into prompt context
- **Tests (first)**:
  - context packing includes memory reference block when memory exists
  - packed context stays within expected token budget while preserving recoverability
  - compaction does not silently drop the only reference to durable memory
- **Done when**: long-running state survives compaction through durable references rather than only LLM summaries.

### M3.3 — Add bounded automatic refresh rules for external memory

- **Deliverable**: memory updates happen at predictable points instead of uncontrolled every-step rewriting.
- **Build**:
  - define refresh triggers, for example:
    - subtask completion
    - compaction boundary
    - repeated retrieval bursts
    - high-confidence extracted findings
  - emit traces for memory refresh decisions
- **Tests (first)**:
  - refresh triggers produce expected writes
  - non-trigger steps do not rewrite memory unnecessarily
- **Done when**: memory updates are bounded, deterministic, and observable.

### M3 exit gate

M3 is Done when all of the following are true:

- a session working-memory artifact exists and is runtime-managed
- context packing can reference durable memory instead of re-inlining long state
- refresh behavior is deterministic and traceable

---

## Milestone 4 — Stronger Recitation And Failure Feedback

**Ship this**: each step carries a compact, recent execution brief, and failure memory becomes specific enough to reduce repeated wrong turns.

### M4.1 — Add execution brief block near the context tail

- **Deliverable**: each step can inject a compact “what matters now” brief into recent context.
- **Build**:
  - add a deterministic execution brief containing:
    - task goal
    - active subtask
    - recently completed work
    - next intended action
    - current blocker if any
  - inject it near the recent-context tail rather than only in the system prompt
- **Tests (first)**:
  - execution brief appears in packed context with stable formatting
  - brief updates on subtask transition and completion
  - brief does not duplicate itself indefinitely across steps
- **Done when**: the current objective is reintroduced near the attention tail in a controlled way.

### M4.2 — Enrich failure memory beyond “last error + tool names”

- **Deliverable**: failure memory captures actionable anti-repetition state.
- **Build**:
  - extend failure memory to include:
    - normalized error type
    - failing tool name
    - arguments fingerprint
    - retryability
    - concise “do not retry unchanged” guidance when applicable
  - preserve bounded size
- **Tests (first)**:
  - repeated identical failing tool call produces anti-repeat guidance
  - retryable and non-retryable failures are represented differently
  - failure-memory block remains within token budget
- **Done when**: failure memory tells the model what failed and what not to do next, not just that something failed.

### M4.3 — Close the feedback loop with measured evaluation

- **Deliverable**: backend changes can be evaluated against concrete runtime signals instead of intuition.
- **Build**:
  - define a small evaluation harness or repeatable manual run checklist using:
    - tool catalog hash churn
    - narration-loop incidence
    - repeated identical tool-call rate
    - context token budget before call
    - compaction frequency
  - record before/after measurements for at least one representative agent workflow
- **Tests / validation**:
  - characterization fixtures for representative workflows
  - one documented before/after measurement report in the touched documentation or implementation notes
- **Done when**: the implementation can justify itself with measured feedback, not only design arguments.

### M4 exit gate

M4 is Done when all of the following are true:

- execution brief is part of context assembly
- failure memory is materially more specific than the current implementation
- at least one representative workflow shows measured feedback from the new telemetry

---

## Public Interfaces And Contract Changes

These are the expected interface-level changes that must be documented and tested when introduced:

- internal LLM request model gains explicit tool-choice semantics
- runtime trace payloads gain request-stability hashes and related metadata
- context policy gains working-memory and execution-brief blocks
- provider adapters must preserve compatibility when tool-choice forcing is unavailable

If any of these changes affect frontend-visible runtime details, update the corresponding feature doc before closing the milestone.

Expected frontend-facing contract additions for long-running harness work:

- session-level harness mode or equivalent session-run metadata
- startup verification status and summary
- handoff summary / clean-exit summary
- durable work-ledger summary or current next-action summary
- harness-specific runtime events where backend state changes are not already expressible through current session-state events

These contract changes should be implemented as typed frontend session data, not only chat message text.

---

## Frontend Workflow Alignment

The frontend already has a strong execution surface in the agent panel and a functional event-driven store. The missing work is not a broad redesign. The missing work is making long-running harness concepts first-class in the UI contract.

### FWA.1 — Add typed harness session data to frontend contracts

- **Deliverable**: frontend agent types can represent long-running harness state without relying on free-form chat content.
- **Build**:
  - extend frontend session-facing types with fields for:
    - harness mode
    - startup verification summary
    - handoff summary
    - next recommended action
    - clean-exit or environment-health marker
  - keep these fields separate from low-level execution-step telemetry
- **Validation**:
  - type-level coverage in the touched frontend tests
  - fixture/session mapping tests for any backend-to-frontend adapters
- **Done when**: the frontend can render handoff and startup state from typed data instead of parsing generic text.

### FWA.2 — Move recovery from chat-only UI to panel-backed workflow

- **Deliverable**: recovery remains visible in chat, but the durable recovery context lives in the agent panel.
- **Build**:
  - keep the inline recovery card as a lightweight action surface
  - add a persistent session summary section near the top of the agent panel that shows:
    - last completed increment
    - current objective
    - startup verification result
    - recommended next action
  - ensure paused and resumed sessions can be understood without scrolling the chat transcript
- **Validation**:
  - component tests for paused-session and resumed-session states
  - manual validation for thread switching and app restart recovery flow
- **Done when**: recovery no longer depends on a single transient chat card to explain session state.

### FWA.3 — Expand status model for harness phases

- **Deliverable**: the UI distinguishes active execution from resume, verification, blocked, and clean-handoff states.
- **Build**:
  - expand derived status logic and status copy for:
    - resuming from handoff
    - verifying environment
    - resume blocked
    - clean handoff saved
  - keep status generation state-driven and deterministic
- **Validation**:
  - unit tests for status derivation
  - panel/header/chat indicator checks for each new state
- **Done when**: long-running workflow transitions are visible without relying on ambiguous phrases like `Thinking...` or `Progress saved`.

### FWA.4 — Add harness event handling only where state transitions require it

- **Deliverable**: the frontend store can react to startup verification and handoff updates without pushing more workflow logic into chat stores.
- **Build**:
  - add explicit event handling only for transitions that cannot be represented cleanly by current session state updates
  - keep the session store authoritative and avoid duplicating durable state in the chat transcript
- **Validation**:
  - store-event tests covering recovery, startup verification, and handoff updates
- **Done when**: new harness behavior is represented by store state first and chat rendering second.

### Frontend implementation guardrails

- keep [chat.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/chat.ts) focused on transcript loading and thread selection; do not make it the owner of durable harness state
- keep [sessionEvents.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/stores/agent/events/sessionEvents.ts) as the event-to-session-state mapping layer; extract harness-specific event mapping into dedicated helpers if the file starts accumulating unrelated transition logic
- keep [agentStatus.ts](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/utils/agentStatus.ts) pure and deterministic; do not turn it into an event history interpreter
- split [AgentPanel.svelte](/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/agent/AgentPanel.svelte) before adding more top-level conditional UI if the handoff/session-summary section grows beyond a compact focused block
- keep recovery chat cards lightweight; they should trigger and summarize, not become the primary rendering layer for durable session state

### Proposed frontend contract sketch

These names are directional and may be refined during implementation, but the shape should stay close to this:

- `HarnessMode = 'initializer' | 'worker' | 'resume_verifier'`
- `StartupVerificationState = 'idle' | 'running' | 'passed' | 'failed'`
- `EnvironmentHealth = 'unknown' | 'healthy' | 'degraded' | 'blocked'`
- `SessionHandoffSummary`
  - `lastCompletedIncrement: string`
  - `currentObjective: string`
  - `nextRecommendedAction: string`
  - `openIssues: string[]`
  - `environmentHealth: EnvironmentHealth`
- `StartupVerificationSummary`
  - `state: StartupVerificationState`
  - `headline: string`
  - `details?: string[]`
  - `checkedAt?: string`
- `AgentSessionHarnessState`
  - `mode: HarnessMode`
  - `startupVerification?: StartupVerificationSummary`
  - `handoff?: SessionHandoffSummary`
  - `cleanExitRecorded: boolean`

Keep these fields separate from:

- execution steps
- tool-call traces
- raw approval state
- transient chat messages

---

## PR-Sized Execution Slices

Implement the work in small reviewable slices. Do not combine unrelated refactors and behavior changes in one PR.

### Slice 1 — Deterministic tool catalog and request-shape telemetry

- Scope:
  - M1.1
  - M1.2
  - characterization tests for stable request preparation
- Keep out:
  - tool-choice forcing
  - external memory
  - frontend workflow changes
- Delivered:
  - M1.1: `ToolRegistry::definitions()` now sorts by tool name; three characterization tests pin the canonical order and its preservation under filtering
  - M1.2: new `infrastructure::agent::runtime::request_shape` module with `system_prompt_hash`, `tool_catalog_hash`, `message_prefix_hash`; these plus `tool_count` are now fields on `LlmTracePayload` and populated on every `LlmRequestPrepared` / `LlmResponseReceived` trace
  - 9 request_shape unit tests cover determinism, tool-add / description-change / order drift, and message-prefix stability when only the last message changes
  - measurement-harness.md signal 1 is now marked as unblocked (telemetry present; representative-workflow measurement still outstanding)

### Slice 2 — Prompt assembly stabilization

- Scope:
  - M1.3
  - trace updates needed to explain intentional request-shape changes
- Keep out:
  - session handoff artifacts
  - frontend contract changes
- Delivered:
  - audit of prompt assembly for dynamic fields: no timestamps / UUIDs / env values leak into `derive_system_prompt` or ContextPolicyV2 blocks; the base agent system prompt is a `const fn` returning `&'static str`; the only legitimate dynamic input is the retrieval pass, gated by threshold breach or subtask change
  - measurement-harness.md now documents per-hash which runtime transitions are allowed to move `system_prompt_hash`, `tool_catalog_hash`, and `message_prefix_hash`, making drift outside those transitions an explicit regression signal
  - new `src-tauri/src/application/agent/execution/prompt_stability_tests.rs` locks in the pipeline invariants with 9 characterization tests: identical session state produces identical hashes; subtask transitions move only the expected components (system prompt hash moves, tool catalog hash stays when mode unchanged, message prefix hash stays when only the tail message is appended); subtask leaving InProgress collapses back to the base-prompt hash

### Slice 3 — Tool-choice runtime state

- Scope:
  - M2.1
  - adapter tests
  - minimal trace support for forced tool-choice state
- Keep out:
  - stable session-wide catalog refactor if it makes the PR too large

### Slice 4 — Stable session-visible tool catalog

- Scope:
  - M2.2
  - M2.3
  - runtime enforcement for disallowed tools under current subtask mode
- Keep out:
  - external memory
  - startup verification

### Slice 5 — Runtime-owned working-memory artifact

- Scope:
  - M3.1
  - M3.2
  - initial memory reference block
- Keep out:
  - automatic refresh heuristics if they are not yet stable

### Slice 6 — Memory refresh policy and observability

- Scope:
  - M3.3
  - trace support for refresh decisions
  - regression tests for deterministic refresh triggers

### Slice 7 — Execution brief and enriched failure memory

- Scope:
  - M4.1
  - M4.2
  - related packing/failure-memory tests

### Slice 8 — Harness session contract and frontend type alignment

- Scope:
  - frontend-facing contract additions from `Public Interfaces And Contract Changes`
  - FWA.1
  - any backend DTO / event changes needed to expose typed harness state
  - follow the contract in [session-harness-contract.md](/home/kassiomaia/devel/zk/coder/docs/planning/agent-context-engineering/session-harness-contract.md)
- Keep out:
  - major UI rework
  - recovery panel redesign

### Slice 9 — Recovery and session-summary workflow

- Scope:
  - FWA.2
  - FWA.3
  - minimal component extraction needed in the agent panel
- Keep out:
  - broad chat redesign

### Slice 10 — Harness event handling and measured workflow validation

- Scope:
  - FWA.4
  - M4.3
  - before/after workflow measurements
  - implementation notes tying measured behavior back to the Manus and Anthropic design inputs
- Delivered:
  - harness event mapping extracted into `src-ui/src/lib/stores/agent/events/harnessEvents.ts` — keeps sessionEvents.ts as the pure session-state mapping layer (FWA.4 guardrail)
  - dedicated store-event tests in `src-ui/src/lib/stores/agent/events/harnessEvents.test.ts` covering recovery (session_paused), startup verification (running/failed/cleanExit), and handoff updates
  - measurement contract in [measurement-harness.md](./measurement-harness.md) — defines the five M4.3 signals, sources today vs. blocked-on-upstream, manual-run checklist, append-only recorded-measurement log, and the tie back to Manus / Anthropic design inputs
  - Slice 10 itself does not change backend runtime behavior; the first non-"not applicable" measurement will come with the next slice that moves runtime signals (expected M1.2 or M4.1)

### Slice sequencing rules

- Slices 1 through 4 must land before frontend harness workflow work begins
- Slice 8 should land before Slice 9 to avoid UI work based on unstable ad hoc payloads
- if a refactor of an oversized module is required to keep a slice reviewable, do the characterization test and extraction first, then land the behavior change in a follow-up slice

---

## Test Plan

Every milestone must include:

- unit tests for pure helpers and hash generation
- provider adapter tests for request conversion
- runtime integration tests covering context assembly and tool execution
- characterization tests for at least one long-running task flow

Validation commands during implementation:

- `cargo test`
- targeted `cargo test <module>` for touched runtime areas
- `cargo clippy`

If a milestone changes frontend-visible runtime behavior or traces consumed by the UI, also run the corresponding frontend checks in the same implementation change.

Frontend validation commands when the touched milestone changes session contracts or agent UI behavior:

- `cd src-ui && npm run check`
- targeted frontend tests for touched agent store / status / component modules

---

## Risks And Guardrails

- **Risk: overfitting to Manus**. Guardrail: blogpost is advisory; measured runtime behavior in this repo decides.
- **Risk: breaking provider compatibility**. Guardrail: keep fallback behavior and explicit adapter tests.
- **Risk: adding prompt complexity without improving behavior**. Guardrail: require telemetry and before/after measurements.
- **Risk: unstable external memory semantics**. Guardrail: choose one runtime-owned memory artifact model and keep it deterministic.
- **Risk: backend-only implementation leaves the workflow opaque**. Guardrail: add typed frontend contract work for handoff, startup verification, and session summary before treating the harness work as complete.

---

## Assumptions

- The current agent architecture remains: execution coordinator, context phase, LLM phase, response phase, tool phase.
- This work is backend-first and does not attempt a larger frontend redesign, but it does include frontend workflow alignment required for long-running harness clarity.
- Existing approvals, loop detection, rollback, and telemetry remain in place and are adapted rather than replaced.
- The Manus article remains a useful design reference but not a repository contract.
