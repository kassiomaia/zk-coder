# Bug: TaskCreated Emitted with Raw User Input — Wrong Title and Missing Subtasks on Live Execution

**Status**: OPEN
**Area**: Backend — `src-tauri/src/application/agent/execution/bootstrap.rs`
**Also affects**: `src-tauri/src/core/agent/task.rs`, frontend `taskEvents.ts`
**Severity**: High — visible to the user every time an agent session runs

---

## Problem Description

When a user sends a message to the agent (e.g. _"Do this or do that"_), the frontend
displays the task title and subtasks incorrectly **during live execution**. The task panel
shows the raw user message as the task title, and the subtasks list is empty. Only after
the user closes the thread and reopens it does the correct LLM-generated title and
subtask list appear.

This creates a confusing experience: the user sees one piece of information while the
agent is working, and a completely different piece of information when they return to the
thread. It looks like the system is inconsistent or has a bug where data is lost.

---

## Root Cause

The bug has a single origin: **`TaskCreated` is emitted at the wrong point in the
bootstrap sequence**, and its payload carries an object that was constructed **before the
LLM planning call ran** — meaning it has only the raw user input, not the generated title.

### The broken emission order in `bootstrap.rs`

```rust
// src-tauri/src/application/agent/execution/bootstrap.rs

// Step 1: Single-shot LLM call — generates title, description, subtasks
let plan = plan_new_task(deps.app_handle, &request.content, session.model_id.as_deref())
    .await?;

// Step 2: task created from AgentTask::new() — still carries raw request.content
let task = AgentTask::new(session.id.clone(), request.clone());
//        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//        request.content = "Do this or do that" (raw user input, no LLM title)

let core_task = crate::core::agent::entities::AgentTask {
    title: plan.title.clone(),   // LLM-generated title goes to the DB entity...
    // ...
};

// Step 3: TaskPlanGenerated fires — but the task doesn't exist in the frontend yet
let _ = deps.app_handle.emit(AGENT_EVENT_NAME, AgentEvent::TaskPlanGenerated {
    session_id: session_id.clone(),
    task_id: task.id.clone(),
    title: plan.title.clone(),
    subtasks: plan.subtasks.clone(),
    // ...
});

// Step 4: SubtaskCreated fires (N times) — but the task still doesn't exist in frontend
for subtask_plan in &plan.subtasks {
    // ...
    let _ = deps.app_handle.emit(AGENT_EVENT_NAME, AgentEvent::SubtaskCreated {
        session_id: session_id.clone(),
        subtask: event_subtask,
    });
}

// Step 5: TaskCreated fires LAST — task is created from the pre-plan snapshot
let _ = deps.app_handle.emit(AGENT_EVENT_NAME, AgentEvent::TaskCreated {
    session_id: session_id.clone(),
    task,  // <-- still carries request.content = raw user input, subtasks = None
});
```

### Why the frontend handlers cannot recover

The frontend `handleTaskPlanGenerated` tries to patch the title by finding the task in
the store and updating it:

```typescript
// src-ui/src/lib/stores/agent/events/taskEvents.ts

export function handleTaskPlanGenerated(state, event) {
    const session = state.sessions.get(event.sessionId);
    if (session && session.tasks) {
        const updatedTasks = session.tasks.map((task) =>
            task.id === event.taskId          // looks for the task...
                ? { ...task, title: event.title, subtasks: event.subtasks ?? task.subtasks }
                : task
        );
        // ...
    }
}
```

At the moment `TaskPlanGenerated` fires, `session.tasks` is either `undefined` or empty —
`TaskCreated` has not arrived yet. The `map()` call finds nothing. **The LLM-generated
title and the subtask plan are silently discarded.**

The same thing happens for every `SubtaskCreated` event:

```typescript
// src-ui/src/lib/stores/agent/events/taskEvents.ts

function handleSubtaskUpdate(state, event) {
    const tasks = session.tasks ?? [];
    const updatedTasks = tasks.map((task) => {
        if (task.id === event.subtask.taskId) {  // parent task not in store yet
            // ...attach subtask...
        }
        return task;   // all subtasks silently discarded
    });
}
```

When `TaskCreated` finally arrives, `toAgentTask()` is called on the raw `AgentTask`
object:

```typescript
// src-ui/src/lib/types/agent.ts

export function toAgentTask(bt: BackendAgentTask): AgentTask {
    if (bt.title) {
        // uses the LLM title — but this field is never set on the event payload
        return { id: bt.id, title: bt.title, ... };
    }

    // Legacy fallback: truncate first 80 chars of request.content
    const firstLine = bt.request.content.split('\n')[0].slice(0, 80);
    return { id: bt.id, title: firstLine, subtasks: [], ... };
    //                         ^^^^^^^^^
    //                         "Do this or do that" — the raw user message
}
```

Because the Rust `AgentTask` struct has no `title` field, the serialized event payload
never sets `bt.title`. The fallback always triggers. The task is created with the raw user
message as its title and an empty subtask list.

### Why reload shows the correct data

When the thread is reopened, `loadSession` calls `get_session_details`, which reads tasks
from the database. The database write path goes through `core_task_to_agents()`, which
correctly maps the LLM-generated `core_task.title` into `request.content`:

```rust
// src-tauri/src/infrastructure/sqlite/repositories/agent/conversions.rs

pub fn core_task_to_agents(task: &core_entities::AgentTask) -> AgentTask {
    AgentTask {
        request: AgentRequest {
            content: task.title.clone(),  // LLM title stored here at write time
            // ...
        },
        // ...
    }
}
```

So on reload, `request.content` already holds the LLM-generated title, and `toAgentTask()`
falls back to it correctly. The subtasks are also loaded from the database and attached.
The live event path and the reload path are inconsistent because the event carries a
pre-plan object while the database stores the post-plan data.

---

## Impact

| Scenario | Title shown | Subtasks shown |
|---|---|---|
| Live execution (current) | Raw user message ❌ | Empty ❌ |
| After thread reload (current) | LLM-generated title ✅ | Full list ✅ |

---

## Expected Behaviour

- `TaskCreated` must only fire **after** `plan_new_task()` completes successfully.
- The `AgentTask` payload in `TaskCreated` must carry:
  - The **LLM-generated title** — never the raw `request.content`.
  - The **full list of planned subtasks** so the frontend can display them immediately.
- There must be **no intermediate state** where the task exists in the frontend store
  with the raw user message as its title.
- The task panel must show the same title and subtask list regardless of whether the
  user is watching live or returns to the thread after the session completes.
- `SubtaskCreated` events emitted during the subtask loop remain valid for **subsequent
  status updates** (e.g. marking a subtask `InProgress` or `Completed`), but they must
  not be the mechanism by which the frontend first learns about a subtask — `TaskCreated`
  already carries the full list at creation time.

---

## Fix Plan

### 1. Add `title: Option<String>` to the runtime `AgentTask` struct

**File**: `src-tauri/src/core/agent/task.rs`

Add a `title` field that is omitted from serialization when `None`. When set, it
overrides `request.content` in the frontend.

```rust
pub struct AgentTask {
    pub id: String,
    pub session_id: SessionId,
    pub request: AgentRequest,
    /// LLM-generated title. When present, the frontend uses this instead of request.content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub status: TaskStatus,
    pub steps: Vec<ExecutionStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtasks: Option<Vec<SubTask>>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

Update `AgentTask::new()` and all struct-literal constructions to include `title: None`:

| File | Location |
|---|---|
| `src-tauri/src/core/agent/task.rs` | `AgentTask::new()` |
| `src-tauri/src/infrastructure/sqlite/repositories/agent/tasks.rs` | `get_session_tasks()` row mapping |
| `src-tauri/src/infrastructure/sqlite/repositories/agent/conversions.rs` | `core_task_to_agents()` |
| `src-tauri/src/application/agent/conversions.rs` | `core_task_to_runtime_task()` |

### 2. Rebuild the `TaskCreated` emission in `bootstrap.rs`

**File**: `src-tauri/src/application/agent/execution/bootstrap.rs`

After `plan_new_task()` succeeds and all subtasks are created in the database:

- Set `task.title = Some(plan.title.clone())` on the task before emitting.
- Collect `event_subtask` objects in the subtask loop and assign them to `task.subtasks`
  before emitting.
- Keep `TaskCreated` as the **last** event emitted in the bootstrap sequence — but now
  it carries complete, correct data.

```rust
// After plan_new_task() and DB writes...

let mut task = AgentTask::new(session.id.clone(), request.clone());
task.title = Some(plan.title.clone()); // LLM-generated, never raw input

let mut collected_subtasks: Vec<SubTask> = Vec::new();

if plan.is_non_trivial && !plan.subtasks.is_empty() {
    for subtask_plan in &plan.subtasks {
        // ... create in DB ...
        let event_subtask = core_task_to_event_subtask(subtask, &core_task.id);
        collected_subtasks.push(event_subtask.clone());
        // SubtaskCreated still emitted for status-tracking purposes
        let _ = deps.app_handle.emit(AGENT_EVENT_NAME, AgentEvent::SubtaskCreated {
            session_id: session_id.clone(),
            subtask: event_subtask,
        });
    }
}

// Attach the full subtask list to the task payload
if !collected_subtasks.is_empty() {
    task.subtasks = Some(collected_subtasks);
}

// TaskCreated fires once, with the real title and all subtasks
let _ = deps.app_handle.emit(AGENT_EVENT_NAME, AgentEvent::TaskCreated {
    session_id: session_id.clone(),
    task,
});
```

### 3. No frontend changes required

The frontend `toAgentTask()` function already handles `bt.title` correctly. Once the
Rust struct emits it, the frontend path is clean with no further modification needed.

`TaskPlanGenerated` can remain in the codebase — it becomes redundant for initial task
setup but may be useful in future scenarios where the plan is regenerated mid-execution.

---

## Files to Touch

| File | Change |
|---|---|
| `src-tauri/src/core/agent/task.rs` | Add `title: Option<String>` to `AgentTask` struct and `new()` |
| `src-tauri/src/application/agent/execution/bootstrap.rs` | Set title + subtasks on task before emitting `TaskCreated` |
| `src-tauri/src/infrastructure/sqlite/repositories/agent/tasks.rs` | Add `title: None` to row mapping in `get_session_tasks()` |
| `src-tauri/src/infrastructure/sqlite/repositories/agent/conversions.rs` | Add `title: None` to `core_task_to_agents()` |
| `src-tauri/src/application/agent/conversions.rs` | Add `title: None` to `core_task_to_runtime_task()` |

