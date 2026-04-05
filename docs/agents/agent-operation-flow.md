# Agent Operation Flow

This document describes the complete sequence of operations that occur when an agent receives a user prompt, based on the actual codebase implementation (not the documentation).

## Table of Contents

- [Overview](#overview)
- [Phase 1: User Input Handling](#phase-1-user-input-handling)
- [Phase 2: Backend Processing](#phase-2-backend-processing)
- [Phase 3: Agent Bootstrap](#phase-3-agent-bootstrap)
- [Phase 4: Execution Loop](#phase-4-execution-loop)
- [Phase 5: Context Resolution](#phase-5-context-resolution)
- [Phase 6: LLM Call](#phase-6-llm-call)
- [Phase 7: Tool Execution](#phase-7-tool-execution)
- [Phase 8: Iteration and Completion](#phase-8-iteration-and-completion)
- [Approval Flow](#approval-flow)
- [File Writing Process](#file-writing-process)
- [Event Flow to Frontend](#event-flow-to-frontend)
- [YOLO Mode](#yolo-mode)
- [Summary](#summary)

## Overview

When an agent receives a prompt like "Write a nodejs project for an api to register users!", it goes through a complete lifecycle from user input to code execution. The agent never writes code directly—it uses tools (`fs_write_file`, etc.) to generate files, with permission checks and user approval for sensitive actions.

## Phase 1: User Input Handling

### Files Involved
- `src-ui/src/lib/components/chat/input/ChatInput.svelte`
- `src-ui/src/lib/stores/chat.ts`
- `src-ui/src/lib/stores/agent/store.ts`

### Steps

1. **User Types Prompt**
   - User types message in the `ChatInput` textarea (lines 425-438)
   - The textarea captures user input

2. **Keyboard Handler**
   - User presses Enter (non-shift key)
   - `handleKeydown()` in `ChatInput.svelte` detects the key press
   - If agent is already working, it triggers cancel
   - Otherwise, it calls `onsubmit()`

3. **Submit Handler**
   - `handleSubmit()` in `ChatArea.svelte` (lines 401-425)
   - Calls `onSendMessage(content, { modelId })` with the user's message

4. **Chat Store Processing**
   - `sendMessageWithLlm()` in `chat.ts` (lines 1283-1392):
     - Sends user message to database via `sendMessage(content, 'user')`
     - Checks if the current thread has an associated agent session
     - If session exists:
       - Loads session via `agentStore.loadSession()`
       - If session is cancelled, creates a replacement session
       - Routes request to agent via `agentStore.sendRequest(targetSessionId, content)`
     - If no agent session, uses normal LLM streaming flow via `LlmHandler`

5. **Agent Store Request Handling**
   - `sendRequest()` in `store.ts` (lines 169-172)
   - Calls `Actions.sendRequest(state, update, sessionId, content)`
   - In `sessionActions.ts` (lines 155-170), invokes Tauri command:
     ```typescript
     const { invoke } = await import('@tauri-apps/api/core');
     await invoke('send_agent_request', { sessionId, content });
     ```

## Phase 2: Backend Processing

### Files Involved
- `src-tauri/src/commands/agent/sessions.rs`
- `src-tauri/src/application/agent/service/runtime_queries.rs`
- `src-tauri/src/infrastructure/agent/runtime/mod.rs`

### Steps

6. **Tauri Command Handler**
   - `send_agent_request()` in `sessions.rs` (lines 137-149)
   - Receives `sessionId` and `content` parameters
   - Calls `state.service.spawn_execution(session_id, content, app_handle).await`

7. **Execution Spawning**
   - `spawn_execution()` in `runtime_queries.rs` (lines 22-120):
     - Validates session state (ensures not already processing)
     - Clears telemetry and approval queue
     - Prepares execution context via `prepare_execution()`:
       - Loads the agent session from database
       - Gets the agent configuration
     - Spawns a Tokio async task that runs the agent execution
     - Registers the execution handle for tracking/cancellation
     - Returns session ID to frontend

## Phase 3: Agent Bootstrap

### Files Involved
- `src-tauri/src/application/agent/execution/bootstrap.rs`
- `src-tauri/src/infrastructure/agent/events.rs`

### Steps

8. **Execution Initialization**
   - `start_execution()` in `bootstrap.rs` (lines 54-180):
     - Updates session state to `Running { current_step: 0 }`
     - Emits `ExecutionStarted` event to frontend
     - Makes a **single-shot LLM call** to plan the task:
       - Generates a **title** and **description** for the task
       - Generates **subtasks** to break down the work
       - Example subtasks: "Set up project structure", "Create user registration API", "Add authentication", "Write tests"
     - Creates a **root task** with the generated title and description
     - Creates all **subtasks** in the database, marking the first one as `InProgress`
     - Initializes the `ExecutionContext` and saves an initial snapshot
     - Emits `SubtaskCreated` events for each subtask (for status tracking)
     - Emits `TaskCreated` event with the LLM-generated title and full subtask list attached
     - Creates the LLM client for subsequent calls
     - Builds the initial LLM context (task info, subtasks, context window)
     - Updates the root task status to `InProgress` and emits `TaskUpdated` event

## Phase 4: Execution Loop

### Files Involved
- `src-tauri/src/application/agent/execution/coordinator.rs`

### Steps

9. **Main Execution Loop**
   - `execute()` in `coordinator.rs` (lines 65-132):
     - Runs in an infinite loop until the task is complete:
       ```
       Loop:
         1. begin_step() - Check for cancellation/termination
         2. resolve_context() - Prepare context for LLM (compaction if needed)
         3. call_llm() - Make API request to LLM
         4. handle_response() - Process LLM response
            - If finished: return result
            - If tools to execute: run_tools()
         5. run_tools() - Execute tool calls with permission checks
            - Emits events for each tool call/result
       ```

## Phase 5: Context Resolution

### Files Involved
- `src-tauri/src/application/agent/execution/context_phase.rs`
- `src-tauri/src/infrastructure/agent/runtime/execution.rs`

### Steps

10. **Context Preparation**
    - `resolve_context()` in `context_phase.rs`:
      - Loads relevant files from the workspace (current project files)
      - Builds context window by including:
        - Task information (root task + subtasks)
        - Workspace files (read-only)
        - Chat history (previous messages)
        - Recent tool calls (to prevent loops)
      - Performs **context compaction** if needed:
        - Saves relevant information to `saved_context`
        - Discards older context to fit within token limit
        - Keeps essential info for continued reasoning

## Phase 6: LLM Call

### Files Involved
- `src-tauri/src/application/agent/execution/llm_phase.rs`
- `src-tauri/src/infrastructure/agent/runtime/execution.rs`

### Steps

11. **LLM Request**
    - `call_llm()` in `llm_phase.rs` (lines 37-82):
      - Constructs `ChatRequest` with:
        - System prompt (agent's role and instructions)
        - User message: "Write a nodejs project for an api to register users!"
        - Context: task info, subtasks, workspace files
        - Tools available: `fs_write_file`, `fs_replace_in_file`, `system_execute_command`, etc.
      - Makes API call to the LLM with **retry logic** (up to N retries)
      - Records detailed traces of request and response for debugging
      - Updates execution phase to `CallingLlm`

12. **LLM Response Handling**
    - `handle_response()` in `response_phase.rs`:
      - Parses LLM response:
        - If finished: returns result (agent is done)
        - If tools requested: extracts tool calls
      - Handles **loop detection**:
        - Compares current tool calls with `recent_tool_calls`
        - If too similar, detects infinite loop and asks for clarification
      - For each tool call:
        - Updates execution phase to `ExecutingTools`
        - Saves current subtask ID for traceability

## Phase 7: Tool Execution

### Files Involved
- `src-tauri/src/application/agent/execution/tool_phase.rs`
- `src-tauri/src/infrastructure/agent/runtime/tool_execution/approval.rs`
- `src-tauri/src/infrastructure/agent/runtime/tool_execution/executor.rs`
- `src-tauri/src/infrastructure/tools/fs/write_file.rs`

### Steps

13. **Tool Execution**
    - `run_tools()` in `tool_phase.rs` (lines 48-99):
      - Adds new tool calls to `recent_tool_calls` (last 10)
      - Updates LLM context with assistant message containing tool calls
      - Updates execution phase to `ExecutingTools`
      - Records `ToolBatchStarted` trace event
      - Calls `process_tool_calls_with_permissions()`

14. **Permission Resolution**
    - `process_tool_calls_with_permissions()` checks each tool:
      - **AutoRun tools** (no approval needed):
        - `read_file` - Reading files (read-only)
        - `agent_plan_subtasks` - Task planning operations
        - `agent_narrate` - Text narration
      - **ApprovalRequired tools** (need user confirmation):
        - `fs_write_file` - Writing or modifying files
        - `fs_replace_in_file` - Text replacements in files
        - `system_execute_command` - Running shell commands
        - `web_request` - Making network requests
        - Deleting files

---

## Approval Flow

When approval is required, here's the detailed process:

### Step 14.1: Action Type Inference
- `infer_action_type()` in `approval.rs` analyzes tool name and arguments
- Returns `ActionType` enum: `write_file`, `delete_file`, `execute_command`, `web_request`, `read_file`

### Step 14.2: Generate Diff Preview
- For `fs_write_file` (lines 86-108 in `approval.rs`):
  - Calls `generate_diff()` to create unified diff
  - For `fs_replace_in_file` (lines 109-131):
    - Calls `generate_replace_preview_diff()`
  - Diff format:
    ```
    --- a/package.json
    +++ b/package.json
    @@ -1,3 +1,3 @@
    -"name": "old-name"
    +"name": "new-name"
    ```
  - Language detection from file extension (JS, TS, etc.)

### Step 14.3: Create Pending Action
- Creates `PendingAction` struct with:
  - Session ID
  - Tool name
  - Action type (e.g., "write_file")
  - Description (e.g., "Agent wants to call tool: fs_write_file")
  - Arguments JSON
  - **Preview** (diff or action details)
- Action ID is generated

### Step 14.4: Update Session State
- Changes session state to `UserInputRequired { pending_action_ids: vec![action_id] }`
- Saves state to database
- Session now waits for user input

### Step 14.5: Emit Approval Event
- Emits `AgentEvent::ApprovalRequired` to frontend
- Frontend receives event and renders `ApprovalMessage` component

### Step 14.6: Queue for Approval
- Adds to `ApprovalQueue` with 5-minute timeout
- Frontend shows approval card in chat

---

## Approval Card UI

**Files Involved:**
- `src-ui/src/lib/components/chat/messages/ApprovalMessage.svelte`

**UI Components:**

### Zone 1: Intent & Context
- Icon (ShieldCheck) indicating security confirmation
- Primary text: "Assistant wants permission for **modifying a file**"
- Secondary text: "The system will update the contents of a local file"

### Zone 2: Actions
- **Deny button** - Reject the action
- **Allow button** - Approve the action (with spinner while processing)

### Zone 3: Technical Details (collapsible)
- Click "Inspect technical details" to expand
- Shows:
  - **Proposed Changes** header for diffs
  - **Action Details** for non-file operations
  - File type/language (e.g., `js`, `ts`)
  - **DiffViewer** component rendering unified diff with syntax highlighting

**State Management:**
- `isLoading`: True while processing approval
- `isExpanded`: Controls diff visibility
- `approvalResolved`: True after user decides
- `approvalDecision`: 'approved' or 'denied'

---

## Approval Processing

### When User Clicks Allow

1. **Frontend Handler** (`sessionActions.ts`)
   ```typescript
   async function approveAction(sessionId: string, actionId: string) {
     await invoke('approve_action', { sessionId, actionId });
   }
   ```
   - Calls Tauri command: `approve_action(sessionId, actionId)`

2. **Backend Command Handler** (`approve_action` in approvals.rs)
   - Finds pending action in queue
   - Marks as approved
   - Emits `AgentEvent::ApprovalApproved`
   - Sends confirmation to backend

3. **Approval Loop Receives Event** (lines 180-210 in `approval.rs`)
   - `rx.recv()` returns `Ok(ApprovalEvent::ActionApproved)`
   - Restores session state to previous state (e.g., `Running`)
   - Emits `SessionStateChanged` event
   - Calls `execute_tool()` to run the actual tool

4. **Tool Execution** (`execute_tool` in executor.rs, lines 37-106)

   **Before command execution** (lines 59-72):
   - Captures workspace snapshot for rollback if command fails
   - Records file stats: scanned, captured, skipped_large, truncated

   **Before file write** (lines 74-86):
   - Captures file state with `file_snapshot_registry.capture()`
   - Stores original content for potential rollback
   - Snapshot format: `{ session_id, file_path, content, timestamp }`

   **Build context** (lines 88-96):
   - Creates `ToolExecutionContext` with:
     - Conversation ID
     - Thread ID
     - Session ID
     - Workspace path (from session)

   **Execute tool** (lines 99-106):
   - Passes call to tool registry
   - Gets `OrchestratorToolResult`

---

## File Writing Process

### WriteFileTool Execution (`fs_write_file`)

**Files Involved:**
- `src-tauri/src/infrastructure/tools/fs/write_file.rs`

**Steps:**

1. **Parse Arguments** (lines 79-80)
   - Parses JSON into `WriteFileArgs` struct
   - Validates path and content

2. **Resolve Path** (line 82)
   - Handles absolute paths (outside workspace)
   - Handles relative paths (relative to workspace root)
   - Returns `PathBuf` to target file

3. **Create Parent Directories** (lines 84-94)
   - If `create_dirs: true` (default), creates directories recursively
   - Example: `mkdir -p src/controllers`
   - Fails if permission denied

4. **Write File** (lines 96-99)
   - Opens file in write mode
   - Overwrites if exists, creates if not
   - Writes content as bytes
   - Returns `bytes_written` count

5. **Return Result** (lines 101-106)
   ```json
   {
     "path": "src/controllers/authController.js",
     "bytes_written": 1523,
     "created": true
   }
   ```

---

## File Diff Generation

**Files Involved:**
- `src-tauri/src/infrastructure/tools/fs/write_file.rs`

**Steps:**

1. **Read Existing Content** (line 121)
   - Reads file from disk
   - Returns `None` if file doesn't exist (new file)

2. **Create Text Diff** (line 123)
   - Uses `similar::TextDiff::from_lines()` to compare old vs new content
   - Computes line-by-line differences

3. **Generate Unified Diff** (lines 125-130)
   - Configures `context_radius: 3` (show 3 lines of context around changes)
   - Generates hunks with line numbers and change markers (`+`, `-`)

4. **Format Output** (lines 136-139)
   ```
   --- a/src/controllers/authController.js
   +++ b/src/controllers/authController.js
   @@ -10,7 +10,7 @@
    // Existing code...
    -const config = loadOldConfig();
    +const config = loadNewConfig();
   ```

5. **Return Result** (lines 132-141)
   - Returns `Some(diff_string)` if changes exist
   - Returns `None` if file is identical (no changes)

---

## Post-Execution Updates

### After Tool Execution

1. **Save Trace Event** (line 70-79 in `tool_phase.rs`)
   - Records `RuntimeTraceType::ToolBatchStarted` with tool names

2. **Update Context** (lines 57-61)
   - Adds assistant message with tool calls to LLM context
   - Enables agent to see previous tool calls in future iterations

3. **Continue Loop** (line 112-129 in `coordinator.rs`)
   - Coordinator loop continues
   - Next iteration: resolves context (includes file changes), calls LLM again
   - Agent sees the file it just created and can continue building

4. **Emit Events to Frontend**
   - **ToolCall** event: Shows collapsible card with tool name and arguments
   - **ToolResult** event: Shows result/output of tool execution
   - **FileTouched** event: Records which files were read/written
   - **SessionStateChanged**: Updates UI with new state

---

## Tool Result Display

**Files Involved:**
- `src-ui/src/lib/components/chat/messages/ToolCallMessage.svelte`
- `src-ui/src/lib/components/agent/ToolCallItem.svelte`

**ToolCallMessage** (main collapsible card):
- Shows tool name: `fs_write_file`
- Shows number of files affected
- Shows execution status (pending/completed/error)

**ToolCallItem** (detailed view):
- Tool name and icon
- Arguments preview (first 100 chars)
- Output/result preview (first 200 chars)
- Status indicator (success/error/pending)
- Expand button for full details

---

## YOLO Mode

### Files Involved
- `src-tauri/src/infrastructure/agent/runtime/permissions.rs`
- `src-tauri/src/commands/chat/threads.rs`

### How It Works

If YOLO mode is enabled (in thread settings):

1. **YOLO Mode Provider** checks session
2. If enabled, **bypasses approval checks** for auto-run tools
3. Executes tools immediately without user confirmation
4. Still shows tool execution in chat for visibility
5. Faster execution, less user intervention

**Configuration:**
- Enabled via `set_yolo_mode()` Tauri command
- Stored in thread metadata
- Can be toggled at any time during session

---

## Event Flow to Frontend

**Files Involved:**
- `src-tauri/src/infrastructure/agent/events.rs`
- `src-ui/src/lib/stores/agent/events/handler.ts`

### Events Emitted

Throughout the execution:

- `ExecutionStarted` - Agent begins working
- `StatusPhrase` - Status updates (e.g., "Planning task", "Writing files")
- `AgentNarration` - Persistent text in chat ("I'm creating a Node.js project...")
- `TaskCreated` - Task created with LLM-generated title and full subtask list
- `TaskPlanGenerated` - LLM-generated plan details (title, description, subtasks)
- `SubtaskCreated` / `SubtaskUpdated` - Subtask status changes
- `ToolCall` - Tool execution started (collapsible card)
- `ToolResult` - Tool execution completed
- `ApprovalRequired` - User approval needed
- `ApprovalApproved` - User approved action
- `ApprovalDenied` - User denied action
- `FileTouched` - Files read/write/delete operations
- `ExecutionComplete` - Final result with content
- `ExecutionError` - If execution fails
- `SessionStateChanged` - State transitions

### Event Handler

`handleAgentEvent()` in `handler.ts` routes events to specialized handlers:

- `SessionEvents` - Handles session state changes
- `ToolEvents` - Handles tool call/result events
- `TerminalEvents` - Handles command output
- `TaskEvents` - Handles task/subtask events
- `FileEvents` - Handles file operations
- `StreamingEvents` - Handles status phrases and narration

Events update the store state, which triggers UI updates.

---

## Phase 8: Iteration and Completion

### Loop Continuation

After tool execution, the coordinator loop continues:

1. **Resolve Context** - Loads updated workspace, includes file changes
2. **LLM Call** - Agent sees the files it just created
3. **Refine Response** - Agent can now:
   - Add more code to files
   - Create new files
   - Run commands to test
   - Read existing files to understand context
4. **Iterate** - Continues until:
   - Agent decides it's finished (returns final result)
   - User cancels (session state becomes `Cancelled`)
   - Loops detected (asks for clarification)

### Completion

If finished:

1. Updates session state to `Finished`
2. Emits `ExecutionComplete` event with final result
3. Frontend displays the final assistant response
4. User can now:
   - Continue the conversation (add more prompts)
   - Archive the thread
   - Start a new thread

### Error Handling

If something goes wrong:

1. Emits `ExecutionError` event
2. Updates session state to `Error`
3. Frontend shows error message
4. User can:
   - Try again (retry execution)
   - Cancel the session
   - Check logs for details

---

## Summary

### For the Prompt: "Write a nodejs project for an api to register users!"

**Complete Flow:**

1. **User types prompt** → ChatInput captures input
2. **Submit** → Chat store sends to backend
3. **Spawn execution** → Tauri command starts async task
4. **Bootstrap** → LLM plans task, creates subtasks
   - "Set up project structure"
   - "Create user registration API"
   - "Add authentication"
   - "Write tests"
5. **Execution loop** → Context resolution → LLM call
6. **LLM response** → Agent decides to write files
7. **Permission check** → `fs_write_file` requires approval
8. **Generate diff** → Shows proposed changes in diff format
9. **UI approval card** → User sees "Assistant wants permission for modifying a file"
10. **User clicks Allow** → Frontend calls `approve_action`
11. **Tool execution** → Creates directories, writes files
    - Captures snapshot before write
    - Writes original content
12. **Emit events** → Frontend shows tool call and result
13. **Loop continues** → Agent sees files created, continues building
14. **Iterate** → More tool calls for:
    - Routes setup
    - Controllers (userController, authController)
    - Middleware
    - Tests
    - Documentation
15. **Complete** → Agent finishes, emits `ExecutionComplete`

**Total Files Written:**
- `package.json` - Dependencies and scripts
- `package-lock.json` - Dependency tree
- `.env.example` - Environment variables template
- `src/app.js` - Express app setup
- `src/controllers/userController.js` - User registration controller
- `src/routes/userRoutes.js` - User routes
- `src/middleware/authMiddleware.js` - Authentication middleware
- `tests/user.test.js` - Test suite
- `README.md` - Project documentation
- Plus additional files as needed

**All files written with user approval** (unless YOLO mode is enabled).

### Key Architectural Patterns

1. **Tauri IPC**: Frontend calls backend via `invoke()`, backend emits events via `app_handle.emit()`
2. **Async Execution**: Agent runs in spawned Tokio task, allowing non-blocking UI
3. **Event-Driven UI**: All progress updates come through the event system
4. **Permission System**: Tool calls go through approval queue for user confirmation
5. **State Machine**: Session states control flow
6. **Tool-Based Execution**: Agent uses tools, never writes directly
7. **Diff Preview**: Users see exact changes before approving
8. **Loop Detection**: Prevents agent from going in circles
9. **Context Management**: Intelligent compaction for token limits
10. **Recovery**: Session state persists across interruptions

### Security Considerations

- All file operations require approval (unless AutoRun)
- Diff previews show exact changes
- Workspace snapshots capture state before writes
- Command execution sandboxed with snapshots
- Rollback capability for failed operations
- User can cancel at any time
- No unauthorized network access
- No file deletion without confirmation

---

## Files Referenced

### Backend (Rust)

- `src-tauri/src/application/agent/execution/coordinator.rs` - Main execution loop
- `src-tauri/src/application/agent/execution/bootstrap.rs` - Task planning
- `src-tauri/src/application/agent/execution/llm_phase.rs` - LLM calls
- `src-tauri/src/application/agent/execution/response_phase.rs` - Response handling
- `src-tauri/src/application/agent/execution/tool_phase.rs` - Tool execution
- `src-tauri/src/application/agent/execution/context_phase.rs` - Context resolution
- `src-tauri/src/application/agent/service/runtime_queries.rs` - Execution spawning
- `src-tauri/src/commands/agent/sessions.rs` - Tauri command handlers
- `src-tauri/src/infrastructure/agent/runtime/tool_execution/executor.rs` - Tool execution
- `src-tauri/src/infrastructure/agent/runtime/tool_execution/approval.rs` - Approval flow
- `src-tauri/src/infrastructure/agent/events.rs` - Event emission
- `src-tauri/src/infrastructure/tools/fs/write_file.rs` - File writing tool
- `src-tauri/src/infrastructure/agent/runtime/permissions.rs` - Permission system
- `src-tauri/src/core/agent/session.rs` - Session state machine
- `src-tauri/src/core/agent/permissions.rs` - Permission resolver

### Frontend (TypeScript/Svelte)

- `src-ui/src/lib/components/chat/input/ChatInput.svelte` - User input handling
- `src-ui/src/lib/components/chat/messages/ApprovalMessage.svelte` - Approval UI
- `src-ui/src/lib/components/chat/messages/ToolCallMessage.svelte` - Tool call UI
- `src-ui/src/lib/stores/chat.ts` - Chat store and message handling
- `src-ui/src/lib/stores/agent/store.ts` - Agent store and event handling
- `src-ui/src/lib/stores/agent/events/handler.ts` - Event dispatcher
- `src-ui/src/lib/stores/agent/events/toolEvents.ts` - Tool event handlers

### Database

- `src-tauri/src/infrastructure/sqlite/repositories/chat/threads.rs` - Thread management
- `src-tauri/src/infrastructure/sqlite/repositories/chat/messages.rs` - Message storage
- `src-tauri/src/infrastructure/sqlite/repositories/agent/sessions.rs` - Session persistence
