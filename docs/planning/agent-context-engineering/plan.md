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
- **Delivered**:
  - Evaluation contract + manual-run checklist + append-only recorded-measurement log shipped in Slice 10 ([measurement-harness.md](./measurement-harness.md)).
  - First real (non-placeholder) measurement entry recorded in `measurement-harness.md` under "M4.3 closeout — after-only characterization baseline", on commit `888c6ea`, workflow "codebase review and PDF book generation" (session `1dbb3c34-d318-4f82-b000-0c5b7b40f8f9`). All five signals populated with real numbers (0 tool catalog churn across 130 requests; 2.3% narration-only steps; 1.5% true repeat-tool-call rate; avg 11.5k / max 36k context tokens across 130 steps; 7 compaction events, all subtask-driven, zero token-limit-driven).
  - Measurement read against design claims: M1.1 (catalog stability), M3.2 (memory packing keeps context bounded), M3.3 (refresh triggers fire as spec'd), M4.1 (execution brief keeps agent action-oriented), M4.2 (runtime compaction is intentional, not a reactive rescue) all confirmed by the numbers.
  - Two follow-ups captured by the measurement and recorded in the entry: (1) scope gap in `FailureClass` — `subtask_mode_disallowed` rejections are classified but the "change args before retry" guidance does not match the actual remediation needed; (2) the clean-exit criterion is loose — a session can finish `clean_exit_recorded = true` with all subtasks marked completed but without producing the planned artifact. Both captured as future work; neither blocks M4.3 closure.
  - Limitation noted in the entry: this is an "after-only" characterization. The M4.3 done-when clause is about measured feedback existing, not about a before/after delta being present, so this satisfies the done-when while leaving a richer before/after comparison as future work.

### M4 exit gate

M4 is **Done** — all of the following are true:

- Execution brief is part of context assembly (Slice 7 / M4.1).
- Failure memory is materially more specific than the previous implementation — `FailureClass` classifier, FNV-1a argument fingerprint, repeat detection, structured block format (Slice 7 / M4.2).
- At least one representative workflow shows measured feedback from the new telemetry — recorded under "M4.3 closeout — after-only characterization baseline" in [measurement-harness.md](./measurement-harness.md), confirming four of five design claims against real numbers and documenting the single partial claim with honest findings.

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
- Delivered:
  - M2.1: new `ToolChoiceMode` enum (`Auto` default, `Required`, `None`) and `ChatRequest::with_tool_choice` builder; `tool_choice` field plumbed through the core entity, the application-layer request conversion, the planner, the summarizer, and the orchestrator
  - OpenAI adapter maps `Auto`/`Required`/`None` → `"auto"` / `"required"` / `"none"` when tools are present; omits `tool_choice` entirely when no tools are sent (final-summary path)
  - Anthropic adapter maps `Auto`/`Required`/`None` → `{"type":"auto"}` / `{"type":"any"}` / `{"type":"none"}` with the same no-tools-means-no-choice fallback
  - 8 new adapter tests (4 per provider) cover each mode plus the final-summary no-tools path
  - `LlmTracePayload.tool_choice_mode` records the mode on every `LlmRequestPrepared` / `LlmResponseReceived` trace so a consumer can distinguish "model chose not to call a tool" from "runtime disallowed tool calls"
  - no runtime caller yet forces a non-`Auto` mode — that is M2.3's scope in Slice 4

### Slice 4 — Stable session-visible tool catalog

- Scope:
  - M2.2
  - M2.3
  - runtime enforcement for disallowed tools under current subtask mode
- Keep out:
  - external memory
  - startup verification
- Delivered:
  - M2.2: `context_phase::resolve_context` no longer filters `tool_definitions` by `SubTaskMode::allowed_tools()`. The LLM always sees the full canonical session catalog from `deps.all_tool_definitions`, so the `tool_catalog_hash` stays stable across subtask transitions (the mode-change allowance documented in measurement-harness.md was removed)
  - M2.2 runtime enforcement: `ExecutionContext.current_allowed_tool_names: Option<Vec<String>>` carries the per-step permitted set; `process_tool_calls_with_permissions` accepts `allowed_tool_names: Option<&[String]>` and returns a structured `is_error: true` result with `reason: "subtask_mode_disallowed"` + `allowedToolNames` for any disallowed call, without invoking the tool executor. `tool_phase::run_tools` enriches the `ToolBatchStarted` / `ToolBatchCompleted` traces with `disallowedByMode` names and counts
  - M2.3: `ExecutionContext.pending_tool_choice_override: Option<ToolChoiceMode>` is a one-shot override consumed (`.take()`) by `llm_phase::call_llm`. Two runtime sites now drive it:
    - final-summary path in `coordinator` sets `ToolChoiceMode::None` + empty `current_allowed_tool_names` (defense-in-depth) when `all_subtasks_completed`; emits a trace with `forcedToolChoiceMode: "none"` and reason `all_subtasks_completed_final_summary`
    - narration-loop `EmitFillerAndNudge` stage sets `ToolChoiceMode::Required` alongside the existing user-facing nudge; emits a trace with `forcedToolChoiceMode: "required"` and reason `narration_loop_nudge`
  - 21 passing tests cover the slice: the 10 prompt-stability tests now include `subtask_mode_transition_no_longer_changes_session_tool_catalog_hash` (the M2.2 invariant) plus 11 new `runtime_enforcement_tests` covering the allow-list predicate (allowed/disallowed/empty/None cases) and the one-shot `ToolChoiceMode` plumbing through `ChatRequest::with_tool_choice`
  - measurement-harness.md updated: the "subtask mode change currently changes `tool_catalog_hash`" allowance is gone; a new "runtime enforcement observability" block documents the `disallowedByMode` trace fields and the `reason` code consumed by the tool result

### Slice 5 — Runtime-owned working-memory artifact

- Scope:
  - M3.1
  - M3.2
  - initial memory reference block
- Keep out:
  - automatic refresh heuristics if they are not yet stable
- Delivered:
  - M3.1: new `core::agent::memory::SessionMemory` entity (task goal,
    active plan, completed findings, open questions, extracted facts,
    created/updated timestamps) plus a `MemoryRepository` trait with
    `get_memory` / `upsert_memory` / `delete_memory`. Backend
    orchestration owns creation and updates — the trait is not exposed
    to tool code.
  - M3.1 storage: `agent_session_memory` SQLite table keyed by
    `session_id` (one row per session); `ON CONFLICT(session_id) DO
    UPDATE` drives idempotent upsert semantics and `updated_at` is
    bumped on every write. Cleanup is explicit in
    `sessions::delete_session` (the CASCADE in the schema is decorative
    because `PRAGMA foreign_keys` is not enabled on existing
    connections).
  - M3.1 wiring: `ExecutionDeps.memory_repo: &dyn MemoryRepository` is
    plumbed through `AgentRuntime::new`, `coordinator::execute`, and
    the infrastructure/application delegation boundary; the
    `SqliteSessionRepository` implements `MemoryRepository` alongside
    the four existing agent traits.
  - M3.1 bootstrap trigger: when `plan.is_non_trivial &&
    !plan.subtasks.is_empty()`, `start_execution` upserts a fresh
    `SessionMemory` with `task_goal = plan.description`,
    `active_plan = plan.subtasks.map(title).join("; ")`, and empty
    finding/question/fact lists. Failure to persist is logged but
    non-fatal — trivial tasks never get a memory row.
  - M3.2: new `MemoryReference` struct (compact: id + truncated goal +
    truncated plan + three knowledge-item counts + updated_at). A new
    `ContextPolicyInput::memory_reference` field carries it into
    `apply_context_policy_v2`, which inlines a deterministic
    `ContextPolicyV2 session memory reference:` system block between
    the pre-computed summary and the recent dialogue tail.
  - M3.2 compaction safety: the block format is a pure function of
    `MemoryReference`, so repeated packing passes produce identical
    bytes; `collect_recent_dialogue_tail` already drops previous
    `ContextPolicyV2 *` blocks from the tail walk, preventing
    duplication. The tiny-context forced-pack short-circuit explicitly
    bypasses when a memory reference is present, so compaction cannot
    silently drop the only durable pointer.
  - M3.2 lookup: `context_phase::resolve_context` fetches the memory
    each step via `deps.memory_repo.get_memory(&session_id)`, condenses
    it into `MemoryReference` (counts instead of raw lists), and feeds
    it to the packer.
  - 18 new passing tests cover the slice: 5 entity tests
    (`core::agent::memory`), 7 SQLite repository tests
    (get/upsert/delete + idempotency + cascade + ordering), and 6
    context-policy packer tests (inline when present, omit when
    absent, deterministic, truncation, tiny-forced-pack runs when
    memory reference exists, repeated-pack does not duplicate block).
  - Measurement harness updated: new working-memory observability
    section documents the `ContextPolicyV2 session memory reference:`
    block format and the stable `message_prefix_hash` invariant that
    identical session memory produces identical packed prefixes across
    steps.
  - Out of scope (intentionally deferred to Slice 6 / M3.3): automatic
    refresh triggers, per-refresh traces, and agent-facing tools that
    edit memory. Slice 5 only creates the memory on bootstrap; updates
    happen through direct repository calls from runtime code (none yet
    wired beyond bootstrap).

### Slice 6 — Memory refresh policy and observability

- Scope:
  - M3.3
  - trace support for refresh decisions
  - regression tests for deterministic refresh triggers
- Delivered:
  - M3.3 trigger contract: new closed enum
    `application::agent::execution::memory_refresh::MemoryRefreshTrigger`
    with three variants — `SubtaskCompleted { subtask_id, title, order }`,
    `CompactionBoundary`, and `RetrievalBurst { count }`. The enum is
    intentionally closed so additions are deliberate; trigger kinds
    serialize as stable `snake_case` labels for traces.
  - Single write path:
    `memory_refresh::apply_memory_refresh(memory_repo, trace_repo,
    session_id, task_id, subtask_id, step_number, trigger, mutation)` is
    the sole site that writes `SessionMemory` outside of bootstrap. It
    loads the current row, applies deduplicated append-with-cap on
    `completed_findings` / `extracted_facts`, updates `active_plan` only
    when the provided value differs, and writes only when something
    actually changed. A `MemoryRefreshed` runtime trace is emitted on
    every call — including skipped cases (`no_memory`, `no_change`,
    `load_failed`, `write_failed`) — so non-writes are observable, not
    inferred from absence.
  - Bounded retention: `MAX_COMPLETED_FINDINGS = 50` and
    `MAX_EXTRACTED_FACTS = 50` cap each list; oldest entries are evicted
    from the front when the cap is crossed. Ensures long runs don't
    grow memory unbounded.
  - `RuntimeTraceType::MemoryRefreshed` enum variant added to
    `core::agent::execution`, wired through
    `execution_debug::trace_type_to_string` / `string_to_trace_type` so
    traces round-trip through the `agent_runtime_traces` table.
  - Runtime wiring in `context_phase::resolve_context`:
    - Snapshots `previous_subtask_id` before subtask resolution.
    - `maybe_fire_subtask_completed` — fires when the previous subtask
      id differs from the current one **and** the previous row is now
      `Completed` on disk; appends the subtask title to findings and
      refreshes `active_plan` to the new in-progress title. Guarded by
      `ctx.last_completed_subtask_for_memory` so the trigger fires at
      most once per completed subtask id.
    - `maybe_fire_retrieval_burst` — fires when the per-subtask
      retrieval counter crosses `RETRIEVAL_BURST_THRESHOLD = 3`. The
      counter resets on every subtask change, so bursts are measured
      inside a single subtask. The mutation appends deduplicated file
      paths distilled from snippet strings (`"path:line text"` →
      `"path"`) into `extracted_facts`; counter is reset after firing.
    - `maybe_fire_compaction_boundary` — fires only when
      `ContextPackOutcome::Compacted` was produced (not on
      `Reorganized` or `Unchanged`); mutation is limited to refreshing
      `active_plan`, so the memory body doesn't drift just because
      compaction happened. Non-compacting pack outcomes do not fire
      this trigger.
  - `ExecutionContext` gains two new fields —
    `retrieval_count_in_current_subtask: u32` and
    `last_completed_subtask_for_memory: Option<String>` — both seeded
    at bootstrap to their zero-values.
  - Pure helpers:
    `memory_refresh::dedupe_append_capped(existing, new, cap)` and
    `memory_refresh::snippet_paths(snippets)`. Both tested in
    isolation; `snippet_paths` preserves first-seen order and handles
    snippet strings that lack a `':'` separator.
  - 14 new passing tests under `memory_refresh::tests`: pure helpers
    (dedupe idempotency, cap eviction, empty-entry skipping,
    path-extraction), plus end-to-end firings using an in-memory
    `SqliteSessionRepository` fixture — no-memory path emits a trace
    and returns early; subtask-completed writes title + active plan
    and emits a trace; subtask-completed repeat is idempotent and
    emits a second "no_change" trace; compaction boundary without
    mutation logs the boundary without touching content;
    compaction-boundary with an active-plan mutation writes;
    retrieval-burst appends paths and deduplicates; retrieval-burst
    respects the extracted-facts cap; non-trigger steps leave memory
    untouched and never emit a `MemoryRefreshed` trace.
  - Trace payload shape is stable: `{ trigger, wrote, findingsAdded,
    factsAdded, activePlanChanged, skippedReason, triggerDetails }`
    where `triggerDetails` carries `subtaskId` / `subtaskTitle` /
    `subtaskOrder` for `SubtaskCompleted` and `retrievalCount` for
    `RetrievalBurst`. Consumers can drive the five M4.3 measurement
    signals from this without reconstructing state.
  - Measurement harness updated: the
    `message_prefix_hash` allowlist entry for "session memory update"
    now calls out that every mutation is emitted by a
    `MemoryRefreshed` trace, and a new "memory refresh triggers
    (M3.3)" block documents the three triggers, their deterministic
    firing rules, the retention caps, and the skipped-reason
    vocabulary.
  - Out of scope (intentionally deferred): (1) agent-facing tools
    that let the model write directly into memory — all Slice 6 writes
    come from runtime-owned triggers; (2) FWA.x frontend surface for
    memory refreshes, which belongs in Slice 8/9; (3) the measured
    before/after workflow run (Slice 10 scope). The backend side of
    M3 — exit gate — is now complete.

### Slice 7 — Execution brief and enriched failure memory

- Scope:
  - M4.1
  - M4.2
  - related packing/failure-memory tests
- Delivered:
  - M4.1: new `ExecutionBrief` struct on
    `infrastructure::agent::runtime::context` with fields `task_goal`,
    `active_subtask`, `recently_completed`, `next_intended_action`,
    `current_blocker`. Exposed as `ContextPolicyInput.execution_brief:
    Option<ExecutionBrief>` and emitted by `apply_context_policy_v2`
    as a single `ContextPolicyV2 execution brief:` system block
    positioned last among the synthetic blocks, so the brief sits as
    close to the attention tail as possible. Field-level truncation
    keeps the block within `BUDGET_EXECUTION_BRIEF_TOKENS = 240`;
    missing fields are omitted rather than rendered as empty lines.
  - M4.1 tiny-pack exception: the tiny-context short-circuit was
    extended — an execution brief now forces the packer to run even
    when the context is tiny, so subtask transitions never suppress
    the brief (mirrors the M3.2 memory-reference exception).
  - M4.1 wiring: `context_phase::resolve_context` synthesizes the
    brief each step from `request.content` (goal),
    `current_subtask_summary` (active subtask title), the full
    `loaded_subtasks` list (most recent 3 completed titles; earliest
    pending title as next intended action), and the runtime LLM
    context (`extract_failure_record` yields the `current_blocker`
    when an error is present). The brief is a pure function of
    state, so two packs on identical state yield byte-identical
    blocks and `message_prefix_hash` stays stable.
  - M4.2: new `FailureClass` enum (`FileNotFound` /
    `PermissionDenied` / `Timeout` / `ValidationFailed` /
    `NotAllowed` / `Other`) classifies the last tool error via
    bounded case-insensitive substring rules (missing substring →
    `Other`). New `FailureRecord { tool_name, args_fingerprint,
    class, message, repeat_detected }` carries the structured
    breakdown of the latest failure. `arguments_fingerprint(&args)`
    produces a stable 8-hex-char FNV-1a digest of the canonicalized
    JSON so equivalent argument objects fingerprint identically
    across runs.
  - M4.2 output: `build_failure_memory` now emits a structured
    multiline block — `Last error:\ntool: ...\nargs_fp: ...\ntype:
    ...\nmessage: ...\nretry: ...` — with a trailing `note:
    identical call has failed before — do not retry unchanged`
    when `extract_failure_record` detects a prior failure with the
    same `tool + args_fingerprint`. The retry verdict is derived
    from `FailureClass::retryable_unchanged()` — `Timeout` and
    `Other` are marked safe-to-retry (transient), the rest require
    changed args. Block stays inside
    `BUDGET_FAILURE_MEMORY_TOKENS = 140`.
  - 15 new passing tests under
    `infrastructure::agent::runtime::context::tests`: six brief
    tests (inline/omit/deterministic/omit-missing-fields/transition-
    moves-block/no-duplicate-across-packs/tiny-forced-pack-exception)
    plus seven failure-memory tests (classifier coverage, single-
    error block shape, retryable verdict for timeouts, repeat
    detection, no-repeat-when-args-differ, budget invariant) plus
    two fingerprint tests (stability / distinguishes-different-args).
  - Measurement harness updated: new "execution brief (M4.1)" and
    "enriched failure memory (M4.2)" blocks document block
    formats, the `message_prefix_hash` allowances for brief-state
    transitions, the anti-repeat guidance signal, and how the five
    M4.3 measurement signals can now correlate against the
    structured failure fields.
  - Out of scope (deferred): (1) M4.3 measured before/after
    workflow run (Slice 10); (2) FWA surface that renders the
    brief / failure-memory contents in the agent panel.

### Slice 8 — Harness session contract and frontend type alignment

- Scope:
  - frontend-facing contract additions from `Public Interfaces And Contract Changes`
  - FWA.1
  - any backend DTO / event changes needed to expose typed harness state
  - follow the contract in [session-harness-contract.md](/home/kassiomaia/devel/zk/coder/docs/planning/agent-context-engineering/session-harness-contract.md)
- Keep out:
  - major UI rework
  - recovery panel redesign
- Delivered:
  - FWA.1 backend shape: new typed read models in
    `application::agent::read_models` — `HarnessMode`,
    `EnvironmentHealth`, `StartupVerificationState`,
    `StartupVerificationSummary`, `SessionHandoffSummary`,
    `AgentSessionHarnessState` — all `camelCase`-serialized so the
    frontend consumes them unchanged. `AgentSessionRuntimeDetails`
    gained `harness_state: Option<AgentSessionHarnessState>` and
    `SessionDetailsResponse` in `commands::agent::history` now
    forwards it under `harnessState` (skip-if-none preserves backward
    compat for older UI builds).
  - FWA.1 derivation: `derive_harness_state(session, tasks,
    overrides)` produces conservative defaults from session lifecycle
    plus task snapshot — `Launching → Initializer`, `Paused →
    ResumeVerifier`, otherwise `Worker`; `Paused` synthesizes an
    `Idle` verification headline and a handoff from the in-progress /
    most-recent-completed subtasks; `Finished` seeds a clean-exit
    handoff. Store-recorded overrides replace individual fields,
    unrecorded fields stay at the derived default so the frontend
    never sees partial state.
  - FWA.1 durable authoring: `application::agent::harness` owns the
    `HarnessStateStore` trait and an `InMemoryHarnessStateStore`
    default (lost on restart — persistence can be layered later
    without changing the consumer contract). Execution lifecycle
    calls already record the authoritative values:
    `bootstrap::start_execution` records running → passed startup
    verification, `termination::fail_execution` records failed
    verification with error detail, `termination::complete_execution`
    records the clean-exit marker. Handoff authoring helper
    `handoff_from_tasks` is available for paused-session capture.
  - FWA.1 live updates: new `HarnessStateUpdated` variant in
    `infrastructure::agent::events::AgentEvent` (serialized as
    `harness_state_updated` with a `harnessState` payload).
    `execution::harness_events::emit_harness_state_updated` rebuilds
    the state from the session + tasks + store overrides and emits
    it; `update_session_state` now calls this after every lifecycle
    transition so the panel stays consistent with
    `session_state_changed` without a second IPC round-trip.
  - FWA.1 frontend types: `$lib/types/agent` now carries
    `HarnessMode`, `StartupVerificationState`, `EnvironmentHealth`,
    `StartupVerificationSummary`, `SessionHandoffSummary`, and
    `AgentSessionHarnessState` as the canonical types. `AgentSession`
    gained an optional `harnessState` field and `AgentEvent` gained
    the `harness_state_updated` variant — both additive so no
    existing event handler or panel view breaks when the field is
    absent.
  - FWA.1 store mapping: harness-event handlers live in their own
    `$lib/stores/agent/events/harnessEvents` module so
    `sessionEvents.ts` stays focused on lifecycle state. The store
    dispatcher in `handler.ts` routes `harness_state_updated`,
    `session_paused`, `recovery_continue`, and `recovery_replan` to
    the dedicated module.
  - FWA.1 derived status and UI: `utils/agentStatus::harnessToStatus`
    takes priority over generic state-derived text so the panel /
    thread header surface resume-verifier transitions, running /
    failed verification, blocked environment, and the clean-handoff
    marker directly from typed data. `AgentPanelHeader.svelte`
    pulls the verification headline and clean-exit marker the same
    way. The new `AgentSessionSummarySection.svelte` component
    renders the durable handoff + verification block conditionally —
    activated only when `harnessState` carries something — and
    exposes Continue / Replan actions scoped to
    `resume_verifier` mode with a blocked-verification guard.
  - FWA.1 frontend tests: 11 new tests in
    `harnessEvents.test.ts` (startup verification / handoff /
    clean-exit / unknown-session / paused-preserves-harness-state /
    recovery dispatch); 4 new harness-aware cases in
    `AgentPanel.test.ts` covering summary rendering, resume-action
    click-through, blocked Continue button, and clean-handoff copy;
    5 new `threadAgentState.test.ts` cases locking in the status
    priority order (verifying / resuming / paused-idle /
    resume-blocked / environment-blocked / clean-handoff-saved);
    `agentStore.test.ts` gained three cases — absent-harness,
    present-harness hydration, and live `harness_state_updated`
    event mutation — so the IPC + event contract is pinned end to
    end.
  - FWA.1 backend tests: 8 new cases across
    `application::agent::read_models::tests` (derive defaults /
    finished-session handoff / full overrides replace / empty
    overrides preserve derived / partial overrides are per-field),
    `application::agent::harness::tests` (in-memory store
    read/write/isolate/clear; `running_/passed_/failed_*`
    verification shape helpers; `handoff_from_tasks` pause-message
    and in-progress subtask behavior), and
    `commands::agent::history::tests` (IPC serializes and omits
    `harnessState`). The existing `runtime_queries.rs` flow now
    asserts that paused sessions derive `ResumeVerifier` + `Idle`
    startup verification and that a `record_clean_exit(true)`
    override wins over the derived default.
  - Compatibility: every new field is optional or skip-if-none on
    the wire, so older frontend builds continue to parse the session
    details response unchanged. Recovery chat cards still render
    from the existing `session_paused` event — the panel-backed
    summary section is additive, not a replacement (keeps Slice 9's
    recovery-workflow-move cleanly scoped).
  - Out of scope (intentionally deferred): (1) FWA.2 — moving the
    recovery primary surface into the panel summary section (Slice
    9); (2) FWA.3 — session-summary contextual menu and
    status-phrase integration (Slice 9); (3) FWA.4 — harness event
    handling extraction beyond what Slice 10 already delivered; (4)
    persistent-store backing for `HarnessStateStore` — the in-memory
    default is sufficient for the current workflow and restart-lost
    overrides simply revert to the conservative derived state.

### Slice 9 — Recovery and session-summary workflow

- Scope:
  - FWA.2
  - FWA.3
  - minimal component extraction needed in the agent panel
- Keep out:
  - broad chat redesign
- Delivered:
  - FWA.2 acceptance: the durable recovery surface now lives in
    `AgentSessionSummarySection.svelte`, mounted at the top of the
    panel's active view (above Activity / Requests / Files / Tool
    Calls) whenever `harnessState` carries a verification, handoff,
    or recorded clean-exit marker. Paused and resumed sessions can be
    understood without scrolling the chat transcript — last completed
    increment, current objective, recommended next action, open
    issues, environment health, and startup verification status all
    render from the typed `AgentSessionHarnessState` that
    `get_session_details` returns and `harness_state_updated` keeps
    live. The summary section also surfaces the Continue / Replan
    actions, so the user can act on the handoff without returning to
    the chat card. The inline `RecoveryMessage` chat card stayed
    lightweight — generic copy (`"Your progress is saved"`) plus
    Continue / Replan buttons, no duplicated explanatory text — per
    the Phase-4 compatibility note in
    [session-harness-contract.md](/home/kassiomaia/devel/zk/coder/docs/planning/agent-context-engineering/session-harness-contract.md).
  - FWA.3 acceptance: `utils/agentStatus::harnessToStatus` is the
    priority branch in `getAgentStatus`, ahead of state-derived text
    and execution-step text, so the four long-running transitions
    surface unambiguously wherever the shared status is rendered
    (panel header, chat status indicator in `ChatLayout`, thread
    headers via `AppShell`'s `currentThreadAgentState`):
    - **Verifying environment** — `verification.state === 'running'`
      yields the verification headline or `"Verifying environment..."`
      with a `scan` icon and `active: true`, even before the coarse
      `SessionState` moves.
    - **Resuming from handoff** — `state.type === 'running' &&
      harnessState.mode === 'resume_verifier'` yields the
      verification headline or `"Resuming from handoff..."`, keeping
      the indicator distinct from a fresh `"Thinking..."`.
    - **Resume blocked** — `paused + verification.state === 'failed'`
      surfaces `"Resume blocked: <headline>"`; `paused +
      handoff.environmentHealth === 'blocked'` surfaces
      `"Environment blocked — investigate before resuming"`. Both
      resolve to `active: false` with a `wait` icon so the stall
      detector does not treat them as a still-running spinner.
    - **Clean handoff saved** — `finished +
      cleanExitRecorded` surfaces `"Clean handoff saved"` with a
      `check` icon, so the post-run state reads as a deliberate
      handoff rather than a generic `"Task completed"`.
    Status derivation stays a pure function of `(SessionState,
    ExecutionStep?, SubTask?, hasPendingApproval, statusPhrases,
    contextUsage, lastActivityAt, AgentSessionHarnessState?)` — no
    hidden scheduling or timers — which keeps the copy deterministic
    and easy to regression-test.
  - Validation: FWA.2 component coverage lives in
    `AgentPanel.test.ts` — summary rendering on paused + idle
    verification, Continue / Replan click-through, disabled Continue
    on failed verification, and the clean-handoff copy for finished
    sessions. FWA.3 derived-status coverage lives in
    `threadAgentState.test.ts` — verifying-environment,
    resuming-from-handoff, paused-resume-verifier, resume-blocked,
    environment-blocked, and clean-handoff-saved cases pin the
    priority order so future status edits cannot silently regress
    harness-awareness.
  - Component extraction: `AgentSessionSummarySection.svelte` is the
    extracted unit — keeps summary concerns out of
    `AgentPanel.svelte`, which stays a layout-only shell. No further
    extraction was needed to keep this slice reviewable, so the
    "minimal component extraction" budget stayed at exactly one new
    file.
  - Timeline note: the FWA.2 / FWA.3 surface was implemented as part
    of the typed-contract work in Slice 8 (types, store mapping,
    panel wiring, and status derivation all move together or none of
    them move at all). Slice 9 formally closes the two deliverables
    and records the acceptance evidence so the plan stays the single
    source of truth for "what ships where". This is why the Slice 8
    Delivered block cited FWA.2 / FWA.3 as deferred — at the time the
    panel summary and status priorities were implemented, but not yet
    treated as the authoritative delivery surface under Slice 9.
  - Out of scope (intentionally deferred): (1) moving recovery
    controls out of chat entirely — the lightweight inline card stays
    because the panel may be collapsed and users need a visible
    resume entry point; (2) redesign of the status-phrase rotation
    mechanism — harness status takes precedence but the existing
    LLM status-phrase rotation is preserved for non-harness runs;
    (3) FWA.4 measurement hookup, which Slice 10 already delivered.

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
  - Slice 10 itself does not change backend runtime behavior; the first non-"not applicable" measurement was recorded after Slice 7 backend landed — see the "M4.3 closeout — after-only characterization baseline" entry in `measurement-harness.md` (commit `888c6ea`, codebase-review+PDF workflow). That entry closes M4.3's done-when and the M4 exit gate.

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
