# Issues

This folder tracks investigations into bugs that degrade the agent execution
flow: performance regressions, reliability problems, correctness bugs, or
usability issues in the core agent runtime.

Each issue is a self-contained folder with:

- `issue.md` — the problem statement, evidence, reproduction steps, and
  proposed direction.
- Supporting artifacts (logs, DB dumps, screenshots) where useful.

## Index

| ID | Area | Status | Title |
|----|------|--------|-------|
| `agent-plan-subtasks-malformed-json` | LLM runtime | fixed (recovery path shipped) | `agent_plan_subtasks` malformed-JSON error aborts agent session |
| `agent-final-summary-tool-call-discard` | Agent coordinator | investigated — awaiting team review | Final-summary path silently discards LLM responses when tool calls slip through `ToolChoiceMode::None` |
| `agent-recovery-banner-in-yolo-and-live-streaming` | Agent pause/recovery | fixed 2026-06-05 | "Your progress is saved" card surfaces in yolo mode and during live streaming |
