# Agent Communication Improvements

## Objective

Make the agent feel like an active collaborator during execution instead of a silent worker that only answers at the end.

The goal is not to remove tool visibility. Tool calls, terminal output, subtasks, and artifacts should remain available. The goal is to add a first-class natural-language communication layer that keeps the user informed while the work is happening.

Success means that a user can follow the run from the main chat alone and still understand:

- what the agent is doing
- why it is doing it
- what it discovered
- what changed in the plan
- what failed
- what it will do next

If the agent panel is hidden, the conversation should still make sense.

## Current State

The current product already has strong execution telemetry but weak execution communication.

### What exists today

The backend already emits execution events through `AgentEvent` in `src-tauri/src/infrastructure/agent/events.rs`, including:

- `execution_started`
- `execution_step`
- `tool_call`
- `tool_result`
- `command_output`
- `command_finished`
- `task_created`
- `subtask_created`
- `subtask_updated`
- `status_phrase`
- `streaming_content`
- `context_compacted`
- `execution_complete`
- `execution_error`

The runtime already encourages communication in the code-agent system prompt in `src-tauri/src/infrastructure/agent/registry.rs`.

The backend already exposes `agent_emit_status` in `src-tauri/src/infrastructure/tools/agent/emit_status.rs`, and the runtime intercepts it in `src-tauri/src/infrastructure/agent/runtime/permissions.rs` to emit `StatusPhrase`.

The frontend already listens to agent events through:

- `src-ui/src/lib/stores/agent/store.ts`
- `src-ui/src/lib/stores/agent/events/handler.ts`
- `src-ui/src/lib/stores/agent/events/*.ts`

The frontend already injects some agent activity into chat in:

- `src-ui/src/routes/+page.svelte`
- `src-ui/src/lib/stores/agent/callbacks.ts`
- `src-ui/src/lib/components/chat/ChatMessagesList.svelte`

### Why the experience still feels silent

The current architecture is built around operational state, not around ongoing assistant narration.

The system knows what the agent is doing, but it mostly treats those signals as:

- telemetry for the agent panel
- transient thinking blocks
- tool cards
- terminal output
- final response persistence

That means the system has observability but not a dedicated conversational layer.

The result is:

- status updates are too short and too technical
- tool cards are visible but not user-friendly communication
- major milestones are not explained in natural language
- errors and approvals are not narrated as part of the conversation
- activity messages are treated as temporary UI artifacts instead of first-class assistant speech

## Root Causes

This is not a fundamental model limitation. It is mostly a product and architecture issue, with a few real defects that make the problem worse.

### 1. Telemetry is implemented, narration is not

The current pipeline is:

`agent runtime -> technical event -> store update -> panel/transient chat artifact`

What is missing is:

`agent runtime -> narration decision -> user-facing assistant update -> durable chat behavior`

There is no dedicated "narrator" layer that converts execution state into human communication.

### 2. The prompt asks for communication, but the tool contract suppresses it

The code-agent prompt already says the agent should explain what it is doing and use `agent_emit_status`.

However, `agent_emit_status` is explicitly designed for brief 2-6 word phrases. That pushes the model toward spinner text instead of real conversation.

Examples encouraged today:

- "Scanning codebase..."
- "Reading files..."
- "Running tests..."

Those phrases are useful as status chips, but they do not create the feeling of interacting with a real assistant.

### 3. The frontend renders progress as secondary activity, not primary conversation

The current rendering path treats agent execution updates as special-case message kinds like:

- `thinking`
- `tool_call`
- `terminal`

Those message types are useful for visibility, but they are not the same as assistant narration.

This makes the main chat feel like:

- a final answer plus some internal machine traces

instead of:

- a running conversation with an assistant

### 4. The callback/event design is fragile

The current communication path depends on single-slot callbacks in `src-ui/src/lib/stores/agent/callbacks.ts`.

This is fragile because:

- one feature can overwrite another
- execution completion behavior is split across multiple callers
- communication is wired as side effects instead of as a typed flow

There is already a concrete defect in `src-ui/src/routes/+page.svelte`: `setExecutionCompleteCallback` is registered twice, which means the later registration overwrites the previous one.

That bug is separate from narration quality, but it confirms this area is not robust enough to support richer communication until it is cleaned up.

### 5. Cleanup behavior assumes execution activity should disappear

The current completion path removes transient messages after execution in `src-ui/src/lib/stores/agent/events/sessionEvents.ts`.

This reinforces the wrong product assumption:

- execution activity is internal noise to clean up

Instead, the product needs to treat part of execution communication as the real conversation history.

## Target Product Behavior

For any non-trivial request, the agent should communicate as a collaborator throughout the run.

The target behavior is:

1. Acknowledge the request and frame the plan in user language.
2. Explain why the first action is being taken.
3. Narrate major discoveries as they affect the work.
4. Explain why tools are being used before or around the call.
5. Explain blockers and errors immediately.
6. Explain approvals in natural language.
7. Explain task and subtask creation in natural language.
8. Summarize what was completed before the final answer.

The primary channel for this is the main chat thread.

The secondary channel remains:

- tool cards
- terminal output
- subtasks
- diffs
- artifacts
- agent panel history

## Required Backend Improvements

### 1. Introduce a dedicated narration event

Add a new event type to `AgentEvent` in `src-tauri/src/infrastructure/agent/events.rs` for user-facing communication.

Recommended name:

- `Narration`

Recommended fields:

- `session_id`
- `conversation_id`
- `thread_id`
- `message`
- `kind`
- `importance`
- `is_transient`
- `replace_key`
- `related_tool`
- `timestamp`

Recommended narration kinds:

- `progress`
- `plan`
- `discovery`
- `approval`
- `warning`
- `error`
- `completion`

This event should be separate from `ToolCall`, `ToolResult`, and `StatusPhrase`.

### 2. Replace or supersede `agent_emit_status`

The current `agent_emit_status` tool is too constrained for the product goal.

Replace it with a richer tool such as:

- `agent_send_update`

Recommended input schema:

- `message: string`
- `kind: enum`
- `replace_key?: string`
- `related_tool?: string`
- `transient?: boolean`

The tool contract should require sentence-level natural language, not terse fragments.

Examples of acceptable messages:

- "I’m mapping the repository structure first so I can review the code in a sensible order."
- "I found the runtime and chat flow entrypoints, and now I’m tracing how execution events reach the thread view."
- "I need approval before running that command because it can write outside the safe workspace."

Examples that should no longer be the default:

- "Reading files..."
- "Planning..."
- "Running tests..."

### 3. Add runtime-authored narration for reliability-critical moments

The runtime should not depend entirely on prompt compliance.

The runtime in `src-tauri/src/infrastructure/agent/runtime/execution.rs` and `src-tauri/src/infrastructure/agent/runtime/permissions.rs` should emit narration automatically for:

- execution start
- task creation
- subtask planning
- subtask transitions
- approval required
- approval granted/denied
- tool failure
- retry
- cancellation
- context compaction
- execution completion
- execution error

These runtime-authored updates should be human-readable and safe even if the model emits nothing.

### 4. Add a narration mapper for tools and runtime actions

Introduce a backend mapping layer that turns technical actions into user-facing explanations.

Examples:

- `fs_read_file`
  - "I’m reading the relevant files first to confirm where this behavior is implemented."
- `system_execute_command`
  - "I’m running a command to verify the current state before I change anything."
- `agent_plan_subtasks`
  - "I’m breaking this work into tracked subtasks so I can move through it in a clear order."
- `agent_advance_to_next_subtask`
  - "I finished the current phase and I’m moving to the next part of the work."

This mapper should avoid exposing raw JSON arguments in the primary chat narration.

### 5. Add pacing, coalescing, and dedupe on the backend

The backend should prevent narration spam.

Add logic to:

- coalesce repetitive file reads into one update
- suppress repeated low-value status messages
- update an existing narration message via `replace_key` when appropriate
- force narration at minimum intervals during long runs

Suggested rules:

- at least one narration update near execution start
- at least one update every meaningful phase transition
- at least one update every 20-30 seconds during long-running work
- immediate narration on blockers, approvals, retries, or failures

### 6. Preserve safe separation between reasoning and narration

Do not expose chain-of-thought.

The backend should support:

- private internal reasoning
- public summarized narration

Narration should explain intent and progress without revealing raw hidden reasoning.

## Agent Loop Improvements

The agent loop currently optimizes for successful execution. It needs to optimize for transparent execution.

### 1. Change the loop shape

Current behavior is effectively:

- think
- act
- repeat
- answer

Target behavior:

- explain next step
- act
- explain result
- repeat
- summarize completion

### 2. Add communication checkpoints

The loop in `src-tauri/src/infrastructure/agent/runtime/execution.rs` should emit narration at these checkpoints:

- request accepted
- plan established
- subtask created
- subtask switched
- before important tool calls
- after important tool results
- after discoveries that change the plan
- on approval wait
- on failure
- on recovery path
- before final answer

### 3. Add model fallback if the run goes quiet

If the model has not sent a user-facing update for too long, the runtime should synthesize one from observed activity.

This prevents long silent runs even when prompt compliance is weak.

### 4. Separate communication-critical events from optional commentary

Communication-critical events must always be narrated:

- approvals
- failures
- completion
- task/subtask transitions

Optional commentary can remain model-driven:

- nuanced discoveries
- explanation of approach
- soft progress language

## Prompt Improvements

Prompt changes are necessary, but they are not sufficient on their own.

### 1. Reframe communication as a hard behavioral requirement

Update the code-agent instructions in `src-tauri/src/infrastructure/agent/registry.rs`.

The prompt should explicitly say:

- maintain an ongoing conversation with the user while you work
- explain what you are about to do before meaningful actions
- explain why you are taking that action
- share discoveries when they change your approach
- explain errors immediately and state your recovery plan
- explain why subtasks are being created
- explain what approval is needed and why
- keep the user informed during long-running work

### 2. Replace "brief status" guidance with conversational examples

Current examples are too terse.

Replace with examples like:

- "I’m scanning the repository structure first so I can group the review sensibly."
- "I found the main backend runtime path, and now I’m tracing how events are emitted to the frontend."
- "This request is broad, so I’m splitting it into phases before I start summarizing files."
- "That command failed, so I’m checking whether the issue is the sandbox or the command itself."

### 3. Distinguish primary narration from secondary technical evidence

The prompt should explicitly say:

- tool calls are not your primary communication channel
- use natural-language updates as the main way to keep the user informed
- technical details can remain in supporting UI

### 4. Set tone expectations

Target tone:

- human
- collaborative
- calm
- clear
- highly chatty for long tasks

The prompt should discourage:

- overly terse fragments
- silent execution
- raw technical jargon without explanation

## Required Frontend Improvements

### 1. Add a dedicated execution narration message kind

Extend `MessageKind` in `src-ui/src/lib/types/message.ts`.

Recommended additions:

- `execution_update`
- `execution_error`
- `execution_milestone`
- `approval`

Do not keep overloading:

- `thinking`
- `tool_call`
- `terminal`

Those types should remain available for secondary telemetry rendering.

### 2. Render narration inline as assistant conversation

Update:

- `src-ui/src/lib/components/chat/ChatMessagesList.svelte`
- `src-ui/src/lib/components/ChatMessage.svelte`
- message-specific components under `src-ui/src/lib/components/chat/messages/`

Narration should render:

- inline in the main thread
- visually lighter than final answers
- clearly written as assistant speech

The user should be able to read the thread and feel guided through the run.

### 3. Keep technical traces secondary

Tool calls and terminal output should still exist, but they should not replace narration.

Recommended hierarchy:

- main chat: natural-language updates
- agent panel: exact technical history
- inline expandable details: optional bridge between the two

### 4. Replace the callback fanout with a typed event-to-chat pipeline

The current callback design in `src-ui/src/lib/stores/agent/callbacks.ts` is too fragile for richer communication.

Replace it with one of:

- reducer-driven event handling
- event bus with multiple subscribers
- typed message dispatcher

Requirements:

- multiple features can observe the same execution event
- communication does not depend on single mutable callback slots
- final answer persistence and artifact attachment cannot overwrite each other
- transient and durable messages are managed explicitly

### 5. Fix persistence rules for execution communication

The frontend should distinguish:

- durable narration
- transient narration
- technical transient messages

Completion cleanup should only remove messages that are explicitly transient and superseded.

It should not assume all execution chatter is disposable.

### 6. Add coalescing behavior on the frontend

If several narration updates are part of one continuous phase, the UI should update a stable message rather than append dozens of near-duplicate messages.

Examples:

- replace "I’m scanning the repository..." with "I found the main source directories and I’m now drilling into the backend runtime."
- keep failures and approvals as separate durable messages

## Existing Problems That Will Block This Solution

### 1. `setExecutionCompleteCallback` is overwritten

In `src-ui/src/routes/+page.svelte`, `setExecutionCompleteCallback` is called twice.

The later registration overwrites the earlier one.

Impact:

- execution completion handling is not reliable
- artifact attachment and final message persistence are split incorrectly
- richer communication features will be brittle if added on top of this

Fix:

- replace single callback registration with composable subscribers or unify completion handling in one reducer path

### 2. Status communication is designed for phrases, not utterances

`agent_emit_status` in `src-tauri/src/infrastructure/tools/agent/emit_status.rs` is explicitly optimized for very short phrases.

Impact:

- communication quality is constrained at the tool level
- prompt compliance still produces low-quality user experience

Fix:

- replace the tool contract with sentence-level narration support

### 3. Session event handling cleans up the wrong things

`handleExecutionComplete` in `src-ui/src/lib/stores/agent/events/sessionEvents.ts` removes transient messages after execution.

Impact:

- the current architecture assumes execution communication should disappear
- durable conversational history is hard to achieve without rewriting cleanup behavior

Fix:

- explicitly classify messages as durable vs transient

### 4. Tool and terminal event handlers only enrich panel state

Current handlers such as:

- `src-ui/src/lib/stores/agent/events/toolEvents.ts`
- `src-ui/src/lib/stores/agent/events/terminalEvents.ts`
- `src-ui/src/lib/stores/agent/events/taskEvents.ts`

mostly update session state for the agent panel.

Impact:

- the information exists but is not promoted into human communication

Fix:

- add a narration synthesis step from these events

### 5. The chat activity interface is too weakly typed

`setAgentActivityCallback` currently accepts `Partial<Message>`.

Impact:

- difficult to reason about update semantics
- message replacement and dedupe become fragile
- execution communication is coupled to ad hoc UI composition

Fix:

- introduce typed execution chat payloads with stable IDs and explicit semantics

## Implementation Plan

### Phase 1: Stabilize the current pipeline

1. Fix the duplicated execution completion callback in `src-ui/src/routes/+page.svelte`.
2. Replace single-slot callback wiring in `src-ui/src/lib/stores/agent/callbacks.ts`.
3. Add explicit durable/transient message classification.
4. Stop blanket cleanup of all transient execution messages.

### Phase 2: Add backend narration support

1. Extend `AgentEvent` with a dedicated narration event.
2. Replace or supersede `agent_emit_status` with sentence-level narration support.
3. Add runtime-authored narration for critical milestones.
4. Add narration pacing and dedupe logic.

### Phase 3: Upgrade the agent loop

1. Add communication checkpoints inside the loop.
2. Ensure the loop narrates before and after meaningful actions.
3. Add fallback runtime narration when the model goes quiet.
4. Add tool-to-language summarization logic.

### Phase 4: Upgrade frontend rendering

1. Add dedicated execution narration message kinds.
2. Render narration in the main chat as assistant conversation.
3. Keep telemetry components secondary and optionally expandable.
4. Support stable message replacement for in-progress updates.

### Phase 5: Prompt and behavior tuning

1. Rewrite communication instructions in the code-agent prompt.
2. Replace terse examples with conversational examples.
3. Add tone and pacing expectations.
4. Validate behavior on long-running tasks and error-heavy runs.

## Validation Scenarios

Use these scenarios to validate the solution.

### Scenario 1: Large repository review

Prompt:

`Please review the codebase, analyse each file, summarize each one, and generate a PowerPoint presentation.`

Expected behavior:

- agent frames the work at the start
- agent explains that it is splitting the review into phases
- agent narrates the repository scan
- agent narrates the transition between backend, frontend, docs, and presentation generation
- agent explains when the presentation is being assembled
- final answer and artifact arrive after an understandable narrated run

### Scenario 2: Approval-gated command

Expected behavior:

- agent explains what it wants to do
- agent explains why approval is needed
- user sees a human explanation before or alongside the approval UI

### Scenario 3: Tool failure and recovery

Expected behavior:

- failure is explained immediately in user language
- next recovery step is stated
- user does not need the tool panel to understand the state of the run

### Scenario 4: Long silent task prevention

Expected behavior:

- no long silent gaps without user-facing updates
- the runtime synthesizes updates if needed

## Acceptance Criteria

The work is complete when all of the following are true:

- the agent communicates in natural language throughout non-trivial runs
- tool cards remain visible but are not the main communication mechanism
- major milestones, failures, approvals, and completions are always narrated
- the main chat thread makes sense even if the agent panel is closed
- execution communication is resilient to prompt variability
- current callback and cleanup bugs no longer undermine the interaction model

## Recommended Files To Touch

Backend:

- `src-tauri/src/infrastructure/agent/events.rs`
- `src-tauri/src/infrastructure/agent/runtime/execution.rs`
- `src-tauri/src/infrastructure/agent/runtime/permissions.rs`
- `src-tauri/src/infrastructure/tools/agent/emit_status.rs`
- `src-tauri/src/infrastructure/agent/registry.rs`

Frontend:

- `src-ui/src/lib/types/message.ts`
- `src-ui/src/lib/stores/agent/callbacks.ts`
- `src-ui/src/lib/stores/agent/events/handler.ts`
- `src-ui/src/lib/stores/agent/events/sessionEvents.ts`
- `src-ui/src/lib/stores/agent/events/streamingEvents.ts`
- `src-ui/src/routes/+page.svelte`
- `src-ui/src/lib/components/chat/ChatMessagesList.svelte`
- `src-ui/src/lib/components/ChatMessage.svelte`

