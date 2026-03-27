# Agent Quality Audit (Backend-First)

Date: 2026-03-27

This document captures the current agent quality assessment and the agreed backend-first improvement plan.

## 1. Findings

- Backend validation is currently green for core agent/LLM targets.
  - `cargo test agent -- --nocapture` passes.
  - `cargo test llm -- --nocapture` passes.
- Tool contract drift was present and has been corrected for backend names.
  - Allowed tool lists now use `system_execute_command` and `search_google_scrape`.
  - Registry and subtask/task mode filters now align on tool names.
- Determinism has improved, but is not complete.
  - The final answer now uses the primary execution response (no second LLM call).
  - Execution now defaults to `temperature=0.0` and can be overridden with `CODER_AGENT_TEMPERATURE`.
- Planning/subtask orchestration now has runtime side effects.
  - Planning, advance, and manual status updates mutate subtask state and emit events.
  - Subtask creation defaults to FullAccess mode and marks the first subtask in progress.
- Context management is improved but still limited.
  - Compaction now inserts a summary (task goal, active subtask, last error, recent file actions).
  - Retrieval is still out of scope.
- Mutation rollback is partial but improved.
  - Direct file writes are snapshotted.
  - Command-driven edits now take a bounded, best-effort workspace snapshot before execution.
- Search capability is improved for coding use.
  - A local workspace grep tool (`search_grep`) is now available and wired into default agent/tool-mode allowlists.
  - Dev-only web scrape search still exists as an optional tool.

## 2. Feature Inventory

### Stable
- Session lifecycle (create/list/update/cancel).
- File read/list/write tools within workspace.
- Command execution with streaming stdout/stderr.
- Tool approvals and pending action handling.
- Event emission for tool calls, command output, and execution steps.

### Partial
- Rollback for direct file writes only.
- Loop detection for repeated tool calls.
- Agent UI event traces (backend side).

### Inconsistent
- Search tool capability vs. “search code” prompts.
- Planning tool semantics vs. runtime behavior.

### Missing or Incomplete
- Local repo search/grep tool.
- Deterministic finalization path.
- Robust context retention/retrieval for long tasks.
- Comprehensive rollback for command-driven changes.

## 3. Gap Analysis

### Critical
- None at the moment for compile/test baseline; tests are green.

### High
- Determinism: second LLM finalization call and non-zero temperature.
- Tool capability mismatch: web search only, no local code search.
- Planning: no demonstrated end-to-end subtask state transitions in runtime.
- Rollback: command-driven mutations are not captured.

### Medium
- Context management: shallow compaction without summarization or retrieval.
- Failure recovery: limited structured fallback or recovery modes.

## 4. Improvement Plan

### Milestone 1: Contract correctness and determinism
- Use the primary execution response as the final answer when no tool calls are returned. Done.
- Gate any “finalization” LLM pass behind an explicit feature flag, off by default.
- Make temperature configurable at session/runtime level and default it to a deterministic-safe value for coding workflows.

### Milestone 2: Real planning or remove the illusion
 - Implement backend-owned subtask state transitions for plan/advance/mark. Done.
 - Persist and emit subtask events in the runtime execution path. Done.
 - If that cannot ship quickly, remove planning tools from the default agent contract to avoid misleading behavior.

### Milestone 3: Minimum viable local code search
- Add a workspace-local grep/search tool with bounded output and explicit workspace scoping.
- Ensure tool availability is reflected in the allowed tool lists and registry definitions.

### Milestone 4: Safer mutations
 - Expand rollback coverage to command-driven edits where feasible. Done (best-effort bounded snapshot).
 - At minimum, record affected file paths and surface rollback limitations in events/telemetry.

### Milestone 5: Context policy upgrade
- Replace “first + last 15” with a structured compacted context that retains:
  - Task goal
  - Active subtask
  - Last error
  - Last file decisions
- Keep retrieval out of scope until a minimal policy proves insufficient.

## 5. Validation Strategy

### Baseline gates
- `cargo test agent -- --nocapture`
- `cargo test llm -- --nocapture`

### Determinism
- Same mocked inputs and tool outputs yield identical final answers and event sequences.
- No second LLM pass in the normal completion path.

### Planning
- Planning tool calls create persisted subtasks.
- Advancing/marking subtasks updates state and emits events in order.

### Local search
- Search tool validates inputs and respects workspace boundaries.
- Representative “find symbol” and “find file” tasks complete without shell commands.

### Rollback
- Direct writes rollback still passes.
- Command-driven edits either rollback or emit explicit non-rollback metadata.

### Context compaction
- After compaction, the system still preserves task goal, active subtask, last error, and latest file decisions.

## 6. Risks / Open Questions

- Planning runtime behavior may exist outside the examined execution path, but is not verified.
- The web search tool is feature-flagged and experimental, so tooling may appear absent in practice.
- Determinism changes may affect existing UX expectations if users rely on the finalization pass output.
