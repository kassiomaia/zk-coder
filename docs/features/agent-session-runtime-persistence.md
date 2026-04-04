# Agent Session Runtime Persistence

**Status**: ACTIVE
**Last reviewed**: 2026-04-03
**Purpose**: Reference for how agent session runtime data is persisted, rehydrated on thread/app reload, and protected by regression tests

---

## Problem

When a thread was reopened, or when the user switched away from a thread and came back, the Code Agent panel could lose visibility into:

- files modified during the session
- tool calls already executed
- other runtime telemetry loaded through `get_session_details`

The loss was especially visible after a session had already been cached once in the frontend store.

## Root Cause

Two defects combined to create the regression:

1. The frontend `agentStore.loadSession()` path treated any cached session as fully loaded.
   - `create_agent_session` and live event handlers populate a lightweight in-memory session.
   - That lightweight session does not necessarily include persisted runtime details such as `filesTouched`, `toolCalls`, `terminalOutput`, `executionSteps`, and `statusPhrases`.
   - On later thread selection, `loadSession()` returned the cached session and skipped `get_session_details`, so persisted telemetry never came back into view.

2. Persisted tool-call data did not fully match the frontend contract.
   - Backend `ToolCallView` serialized snake_case field names.
   - The response also omitted the stored timestamp.
   - Frontend hydration expected camelCase fields and a `timestamp`.

## Persistence Contract

Runtime session data must survive:

- switching between threads
- switching between conversations
- closing and reopening the application

The source of truth for persisted runtime telemetry is the agent SQLite store:

- `agent_files_touched`
- `agent_tool_calls`
- `agent_terminal_output`
- `agent_status_phrases`
- execution-step/task tables already used by `get_session_details`

The frontend store is a cache, not the source of truth.

## Fix Strategy

### Frontend

- Treat cached sessions as either:
  - hydrated: runtime arrays already loaded
  - lightweight: session shell only
- Rehydrate lightweight sessions through `get_session_details` before using them as the active thread session.
- Merge persisted data into existing in-memory session state so volatile UI-only fields such as streaming buffers are not lost.

### Backend

- Return complete persisted tool-call data from `load_session_runtime_details()`.
- Serialize `ToolCallView` with camelCase field names.
- Include the persisted tool-call `timestamp` in repository and IPC read models.

## Regression Coverage

### Frontend

`src-ui/src/lib/stores/agentStore.test.ts`

- verifies cached lightweight sessions are rehydrated from `get_session_details`
- verifies already-hydrated sessions do not refetch unnecessarily

### Backend

`src-tauri/src/application/agent/service_tests.rs`

- verifies `load_session_runtime_details()` returns persisted touched files
- verifies `load_session_runtime_details()` returns persisted tool calls including arguments, result, duration, and timestamp

## Expected Behavior After Fix

When a thread is loaded, its linked agent session should restore the same persisted runtime context that existed before the thread switch or app restart:

- touched files remain visible
- tool-call history remains visible
- session runtime details are reconstructed from persisted storage instead of relying on an in-memory cache
