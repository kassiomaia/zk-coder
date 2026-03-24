# PRD: System Agents - Background Autonomous Agent System

## 1. Executive Summary

### 1.1 Problem Statement

The current agent system in this application is entirely interactive and user-driven. Users must manually trigger every agent execution, monitor progress in real-time, and respond to approval prompts. There is no capability for:

- Automated background maintenance tasks
- Periodic system health reviews
- Proactive execution analysis and reporting

### 1.2 Solution Overview

Introduce a **System Agents** layer that runs autonomously in the background without user interaction. System agents execute scheduled tasks (sanitization, review, reporting) silently, with results available post-execution via desktop notifications and a dashboard view.

### 1.3 Target Users

- Power users who want automated maintenance
- Teams requiring periodic code review summaries
- Users who want proactive agent execution insights

---

## 2. Product Vision

> Enable the application to maintain itself autonomously through background agents that run scheduled tasks without user intervention, delivering insights and notifications post-execution.

---

## 3. Functional Requirements

### 3.1 Core Features

| ID | Feature | Description |
|----|---------|-------------|
| F-01 | **Background Scheduling** | System agents run on cron schedules without user trigger |
| F-02 | **Silent Execution** | No real-time events to frontend during execution |
| F-03 | **Post-Execution Notifications** | Desktop notifications after task completion/failure |
| F-04 | **Dashboard View** | Read-only panel showing task history and status |
| F-05 | **Task Configuration** | Enable/disable tasks, modify schedules |
| F-06 | **Reliability** | Auto-retry on failure, auto-pause after repeated failures |

### 3.2 Built-in System Tasks

| ID | Task | Default Schedule | Purpose |
|----|------|-----------------|---------|
| T-01 | `sanitize` | `0 0 * * *` (daily midnight) | Read-only health checks: disk space, log analysis, error patterns |
| T-02 | `review` | `0 */6 * * *` (every 6 hours) | Analyze recent agent executions: success rate, error patterns, recommendations |
| T-03 | `report` | `0 20 * * *` (daily 8pm) | Daily digest: task summaries, metrics, highlights |

### 3.3 User Interactions

| Interaction | Description |
|-------------|-------------|
| View Dashboard | Open System tab in Agent Panel to view task history |
| Configure Tasks | Enable/disable tasks, modify cron schedules |
| Manual Trigger | "Run Now" button to execute task immediately |
| Pause/Resume | Pause all system tasks or resume |
| View Results | View past execution results in dashboard |

### 3.4 Data Flow

```
Scheduler (cron)
    │
    ▼
SystemManager (creates session)
    │
    ▼
Executor (runs tools sequentially)
    │
    ├── Logs to system_task_runs
    │
    ▼
Desktop Notification (post-execution)
    │
    ▼
User opens dashboard → views results
```

---

## 4. Technical Specification

### 4.1 Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              APPLICATION                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   EXISTING LAYER                    NEW SYSTEM LAYER                        │
│   ┌─────────────────┐              ┌─────────────────────────┐             │
│   │  Chat/Agent UI  │              │   System Manager        │             │
│   │  (interactive)  │              │   • Scheduler           │             │
│   └────────┬────────┘              │   • Executor            │             │
│            │                        │   • Notifier           │             │
│            │ Events                 └────────────┬────────────┘             │
│            ▼                                    ▼                           │
│   ┌─────────────────┐              ┌─────────────────────────┐             │
│   │  Runtime        │              │   coder_system.db       │             │
│   │  (user agents)  │              │   (separate file)      │             │
│   └─────────────────┘              └─────────────────────────┘             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Database Schema

**Database File:** `coder_system.db` (in app data directory)

```sql
-- system_sessions: Task execution instances
CREATE TABLE system_sessions (
    id TEXT PRIMARY KEY,
    task_type TEXT NOT NULL,          -- 'sanitize' | 'review' | 'report'
    status TEXT NOT NULL,             -- 'pending' | 'running' | 'completed' | 'failed' | 'paused'
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    workspace_root TEXT,
    created_at TEXT NOT NULL
);

-- system_task_runs: Individual tool executions
CREATE TABLE system_task_runs (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES system_sessions(id),
    tool_name TEXT NOT NULL,
    tool_input TEXT,
    tool_output TEXT,
    executed_at TEXT NOT NULL,
    success INTEGER
);

-- system_schedules: Task schedules and config
CREATE TABLE system_schedules (
    task_type TEXT PRIMARY KEY,
    schedule TEXT NOT NULL,           -- cron expression
    enabled INTEGER DEFAULT 1,
    last_run TEXT,
    next_run TEXT,
    failure_count INTEGER DEFAULT 0,
    config TEXT
);
```

### 4.3 Module Structure

```
src-tauri/src/
├── system/                           # NEW MODULE
│   ├── mod.rs                       # Exports
│   ├── config.rs                    # SystemConfig
│   ├── store.rs                     # SQLite operations
│   ├── scheduler.rs                 # tokio-cron-scheduler
│   ├── manager.rs                   # SystemManager
│   ├── executor.rs                  # Tool executor
│   ├── notifier.rs                  # Desktop notifications
│   └── tasks/
│       ├── mod.rs                   # Task trait
│       ├── sanitize.rs              # Health check task
│       ├── review.rs                # Execution review task
│       └── report.rs                # Daily report task
│
├── commands/
│   └── system.rs                    # Tauri commands (config only)
│
└── lib.rs                           # Integrate SystemManager
```

### 4.4 Tauri Commands

| Command | Input | Output | Description |
|---------|-------|--------|-------------|
| `get_system_config` | - | `SystemConfig` | Get current configuration |
| `update_system_config` | `SystemConfig` | `()` | Update configuration |
| `get_system_task_status` | `task_type` | `TaskStatus` | Get task status |
| `get_system_history` | `task_type, limit` | `Vec<TaskRun>` | Get past executions |
| `pause_system` | - | `()` | Pause all scheduled tasks |
| `resume_system` | - | `()` | Resume scheduled tasks |

### 4.5 Configuration

```rust
struct SystemConfig {
    enabled: bool,                    // Default: false
    
    tasks: {
        sanitize: TaskConfig {
            enabled: bool,             // Default: true
            schedule: String,         // Default: "0 0 * * *"
        },
        review: TaskConfig {
            enabled: bool,             // Default: true  
            schedule: String,         // Default: "0 */6 * * *"
        },
        report: TaskConfig {
            enabled: bool,             // Default: true
            schedule: String,         // Default: "0 20 * * *"
        }
    },
    
    max_concurrent: u32,              // Default: 1
    max_duration_minutes: u32,        // Default: 30
    max_retries: u32,                // Default: 3
    pause_after_failures: u32,        // Default: 2
    
    notifications: NotificationConfig {
        on_failure: bool,              // Default: true
        on_success: bool,             // Default: false
    }
}
```

---

## 5. Non-Functional Requirements

### 5.1 Performance

| Requirement | Target |
|-------------|--------|
| Max concurrent system tasks | 1 |
| Max task duration | 30 minutes |
| Scheduler check interval | 60 seconds |

### 5.2 Reliability

| Requirement | Implementation |
|-------------|----------------|
| Crash recovery | On startup, mark orphaned "running" sessions as failed |
| Retry logic | 3 retries with exponential backoff (1min, 5min, 15min) |
| Auto-pause | Pause task after 2 consecutive failures |

### 5.3 Security

| Requirement | Implementation |
|-------------|----------------|
| Isolation | Separate database, no shared state with user agents |
| No approval bypass | System tasks run with restricted toolset (read-only) |
| Audit logging | Every tool execution logged to `system_task_runs` |

### 5.4 Platform

| Platform | Behavior |
|----------|----------|
| Windows | Runs while app window is open (tray not included in this release) |
| macOS | Runs while app is running (close window ≠ quit) |
| Linux | Runs while app window is open |

---

## 6. UI/UX Specification

### 6.1 Dashboard View

Located in **Agent Panel → System Tab**

```
┌─────────────────────────────────────────────────────┐
│  System Agents                              [×]   │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌─────────────────────────────────────────────┐   │
│  │  Tasks                                       │   │
│  ├─────────────────────────────────────────────┤   │
│  │  [●] sanitize   Daily   [Run Now] [⚙]       │   │
│  │      Last: Today 00:00  Status: Success    │   │
│  ├─────────────────────────────────────────────┤   │
│  │  [●] review    Every 6h [Run Now] [⚙]      │   │
│  │      Last: 6 hours ago  Status: Success    │   │
│  ├─────────────────────────────────────────────┤   │
│  │  [●] report    Daily 8pm [Run Now] [⚙]     │   │
│  │      Status: Paused (2 failures)            │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
│  ┌─────────────────────────────────────────────┐   │
│  │  Recent Activity                            │   │
│  ├─────────────────────────────────────────────┤   │
│  │  • sanitize - Success - Today 00:00        │   │
│  │  • review  - Success - 6 hours ago         │   │
│  │  • review  - Failed  - 12 hours ago        │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### 6.2 Status Bar

Add toggle button in status bar:
```
[Status] [Agent] [System] v0.1.0
```

### 6.3 Notifications

Desktop notification after task completion:
```
Title: System Task Complete
Body: sanitize completed successfully. 3 checks passed.
```

---

## 7. Dependencies

### 7.1 New Rust Dependencies

```toml
tokio-cron-scheduler = "0.13"
```

### 7.2 Existing Dependencies (Reused)

- `rusqlite` - SQLite database
- `tauri-plugin-notification` - Desktop notifications
- `tokio` - Async runtime
- `chrono` - Date/time handling

---

## 8. Out of Scope (v1.0)

| Feature | Reason |
|---------|--------|
| System tray | Future enhancement |
| Custom user tasks | Future enhancement |
| Email notifications | Future enhancement |
| Webhook integrations | Future enhancement |
| Auto-start on boot | Future enhancement |

---

## 9. Success Criteria

- [ ] System tasks run on scheduled times without user intervention
- [ ] No interference with existing interactive agent system
- [ ] Desktop notifications appear after task completion
- [ ] Dashboard displays accurate task history
- [ ] Failed tasks retry automatically
- [ ] Tasks pause after repeated failures
- [ ] App startup recovery handles crashed tasks

---

## 10. Timeline Estimate

| Phase | Description | Est. Time |
|-------|-------------|-----------|
| Phase 1 | Backend core (store, config, scheduler, manager, executor) | 2-3 days |
| Phase 2 | Built-in tasks (sanitize, review, report) | 1-2 days |
| Phase 3 | Commands & integration | 1 day |
| Phase 4 | Frontend dashboard | 1-2 days |
| Phase 5 | Testing & polish | 1 day |
| **Total** | | **6-9 days** |

---

## 11. Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Resource exhaustion | High | Semaphore for max 1 concurrent task |
| Database corruption | Medium | Separate database file |
| LLM rate limits | Medium | Add retry with backoff |
| Task takes too long | Medium | 30-minute timeout |

---

## 12. Open Questions

1. **Should system tasks use the same LLM as user agents?** Yes, share the same LLM configuration.

2. **What tools should system tasks have access to?** Read-only tools initially: file system read, grep, system info.

3. **Should users be able to create custom tasks in v1.0?** No, defer to future release.

4. **Platform priority for initial release?** Cross-platform from start (same codebase).
