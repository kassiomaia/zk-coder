# Implementation Guide

## Step-by-Step for Development Team

---

## 1. Prerequisites

### 1.1 Add Dependencies

```bash
cd src-tauri
cargo add tokio-cron-scheduler
cargo add async-trait
```

### 1.2 Verify Compiles

```bash
cargo build
```

---

## 2. Phase 1: Foundation

### 2.1 Create Module Structure

```bash
mkdir -p src-tauri/src/system/tasks
```

### 2.2 Create Files (in order)

#### Step 1: `src-tauri/src/system/mod.rs`

```rust
//! System Agent Module
//! 
//! Background autonomous agents that run scheduled tasks.

pub mod config;
pub mod store;
pub mod scheduler;
pub mod manager;
pub mod executor;
pub mod notifier;
pub mod tasks;

// Re-exports
pub use config::{SystemConfig, TaskConfig, TaskConfigs, Limits, NotificationSettings};
pub use store::{SystemStore, SystemSession, Schedule, StoreError};
pub use manager::SystemManager;
```

#### Step 2: `src-tauri/src/system/config.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub enabled: bool,
    pub tasks: TaskConfigs,
    pub limits: Limits,
    pub notifications: NotificationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfigs {
    pub sanitize: TaskConfig,
    pub review: TaskConfig,
    pub report: TaskConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub enabled: bool,
    pub schedule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    pub max_concurrent: u32,
    pub max_duration_minutes: u32,
    pub max_retries: u32,
    pub pause_after_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub on_failure: bool,
    pub on_success: bool,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tasks: TaskConfigs {
                sanitize: TaskConfig {
                    enabled: true,
                    schedule: "0 0 * * *".to_string(),
                },
                review: TaskConfig {
                    enabled: true,
                    schedule: "0 */6 * * *".to_string(),
                },
                report: TaskConfig {
                    enabled: true,
                    schedule: "0 20 * * *".to_string(),
                },
            },
            limits: Limits {
                max_concurrent: 1,
                max_duration_minutes: 30,
                max_retries: 3,
                pause_after_failures: 2,
            },
            notifications: NotificationSettings {
                on_failure: true,
                on_success: false,
            },
        }
    }
}
```

#### Step 3: `src-tauri/src/system/store.rs`

See the detailed implementation in the PRD (Section 4.2).

Key methods to implement:
- `new(db_path) -> Self`
- `init_schema(conn)` - Creates tables
- `create_session(task_type, workspace_root) -> SystemSession`
- `update_session_status(id, status, error_message)`
- `get_session(id) -> SystemSession`
- `get_task_history(task_type, limit) -> Vec<SystemSession>`
- `get_running_sessions() -> Vec<SystemSession>`
- `log_tool_run(session_id, tool_name, input, output, success)`
- `get_schedule(task_type) -> Option<Schedule>`
- `update_schedule(schedule)`

#### Step 4: Update `src-tauri/src/lib.rs`

Add SystemManager initialization (see Integration Specification).

### 2.3 Verify

```bash
cargo build
# Should compile with warnings about unused code
```

---

## 3. Phase 2: Scheduler & Executor

### 3.1 Create `src-tauri/src/system/scheduler.rs`

```rust
use tokio_cron_scheduler::{JobScheduler, Job};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct TaskScheduler {
    scheduler: JobScheduler,
    enabled: Arc<RwLock<bool>>,
}

impl TaskScheduler {
    pub fn new() -> Result<Self, SchedulerError> {
        let scheduler = JobScheduler::new();
        Ok(Self {
            scheduler,
            enabled: Arc::new(RwLock::new(true)),
        })
    }
    
    pub fn schedule_task<F>(&self, cron: &str, _task_type: &str, callback: F) -> Result<(), SchedulerError>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let job = Job::new(cron, callback)?;
        self.scheduler.add(job)?;
        Ok(())
    }
    
    pub async fn start(&self) -> Result<(), SchedulerError> {
        self.scheduler.start().await.map_err(|e| SchedulerError::Schedule(e.to_string()))
    }
    
    pub async fn stop(&self) -> Result<(), SchedulerError> {
        self.scheduler.stop().await.map_err(|e| SchedulerError::Schedule(e.to_string()))
    }
    
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }
    
    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("Cron parse error: {0}")]
    CronParse(String),
    #[error("Scheduler error: {0}")]
    Schedule(String),
}
```

### 3.2 Create `src-tauri/src/system/executor.rs`

```rust
use crate::tools::ToolRegistry;
use serde_json::Value;

const SYSTEM_TASK_ALLOWED_TOOLS: &[&str] = &[
    "fs_read_file",
    "fs_list_files",
    "system_get_app_info",
    "system_execute",
    "search_grep",
    "google_scrape",
    "conversation_search_messages",
];

pub struct SystemExecutor {
    tool_registry: ToolRegistry,
}

impl SystemExecutor {
    pub fn new(tool_registry: ToolRegistry) -> Self {
        Self { tool_registry }
    }
    
    pub fn can_execute_tool(&self, tool_name: &str) -> bool {
        SYSTEM_TASK_ALLOWED_TOOLS.contains(&tool_name)
    }
    
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        workspace_root: Option<&str>,
    ) -> Result<ToolResult, ExecutorError> {
        if !self.can_execute_tool(tool_name) {
            return Err(ExecutorError::ToolNotAllowed(tool_name.to_string()));
        }
        
        // Execute via tool registry
        // ... implementation
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: Value,
    pub duration_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Tool not allowed: {0}")]
    ToolNotAllowed(String),
    #[error("Execution error: {0}")]
    Execution(String),
    #[error("Timeout")]
    Timeout,
}
```

### 3.3 Create `src-tauri/src/system/manager.rs`

This is the core orchestrator - see Integration Specification for details.

Key methods:
- `new(...) -> Result<Self, ManagerError>`
- `start() -> Result<(), ManagerError>`
- `shutdown() -> Result<(), ManagerError>`
- `run_task(task_type) -> Result<String, ManagerError>`
- `run_task_with_retry(task_type) -> Result<String, ManagerError>`
- `recover_orphaned_sessions() -> Result<(), ManagerError>`

### 3.4 Create `src-tauri/src/commands/system.rs`

```rust
use tauri::State;
use crate::system::SystemManager;

#[tauri::command]
pub async fn run_system_task(
    task_type: String,
    state: State<'_, SystemManager>,
) -> Result<String, String> {
    state.run_task(&task_type).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_system_status(
    state: State<'_, SystemManager>,
) -> Result<SystemStatus, String> {
    state.get_status().await.map_err(|e| e.to_string())
}
```

### 3.5 Verify

```bash
cargo build
```

---

## 4. Phase 3: Built-in Tasks

### 4.1 Create `src-tauri/src/system/tasks/mod.rs`

```rust
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait SystemTask: Send + Sync {
    fn task_type(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, ctx: &TaskContext) -> Result<TaskResult, TaskError>;
}

pub struct TaskContext {
    pub store: Arc<SystemStore>,
    pub executor: Arc<SystemExecutor>,
    pub workspace_root: Option<String>,
    pub session_id: String,
}

pub struct TaskResult {
    pub success: bool,
    pub summary: String,
    pub details: Value,
}

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Execution failed: {0}")]
    Execution(String),
}
```

### 4.2 Create Tasks

- `sanitize.rs` - Health checks
- `review.rs` - Execution analysis  
- `report.rs` - Daily digest

### 4.3 Connect to Scheduler

Update manager.rs to schedule tasks on startup.

---

## 5. Phase 4: Notifications & Recovery

### 5.1 Create `src-tauri/src/system/notifier.rs`

```rust
use tauri_plugin_notification::NotificationExt;

pub struct SystemNotifier {
    app_handle: tauri::AppHandle,
    settings: NotificationSettings,
}

impl SystemNotifier {
    pub fn new(app_handle: tauri::AppHandle, settings: NotificationSettings) -> Self {
        Self { app_handle, settings }
    }
    
    pub fn notify_success(&self, task_type: &str, summary: &str) -> Result<(), NotificationError> {
        if !self.settings.on_success {
            return Ok(());
        }
        
        self.app_handle
            .notification()
            .builder()
            .title(&format!("System Task Complete: {}", task_type))
            .body(summary)
            .show()
            .map_err(|e| NotificationError::Send(e.to_string()))?;
        
        Ok(())
    }
    
    pub fn notify_failure(&self, task_type: &str, error: &str) -> Result<(), NotificationError> {
        if !self.settings.on_failure {
            return Ok(());
        }
        
        self.app_handle
            .notification()
            .builder()
            .title(&format!("System Task Failed: {}", task_type))
            .body(error)
            .show()
            .map_err(|e| NotificationError::Send(e.to_string()))?;
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("Send error: {0}")]
    Send(String),
}
```

### 5.2 Add Recovery Logic

In manager.rs:
- `recover_orphaned_sessions()` - Call on startup
- `run_task_with_retry()` - Replace direct run
- `should_pause()` - Check failure count

---

## 6. Phase 5: Commands

Add all commands from the PRD (Section 4.4).

---

## 7. Phase 6: Frontend

### 7.1 Create Store

`src-ui/src/lib/stores/systemStore.ts`

### 7.2 Create API

`src-ui/src/lib/api/system.ts`

### 7.3 Create Panel

`src-ui/src/lib/components/agent/SystemPanel.svelte`

### 7.4 Integrate

Update AgentPanel.svelte and +page.svelte.

---

## 8. Verification Commands

### 8.1 Build

```bash
cargo build
```

### 8.2 Run Dev

```bash
cargo tauri dev
```

### 8.3 Check Database

```bash
sqlite3 ~/.local/share/com.coder.app/coder_system.db ".tables"
```

### 8.4 View Logs

```bash
tail -f ~/.local/share/com.coder.app/logs/*.log
```

---

## 9. Common Issues

### 9.1 "ToolRegistry not found"

**Solution**: Expose from AgentState (see Integration Spec)

### 9.2 "Database locked"

**Solution**: Use proper locking or separate connections

### 9.3 "Scheduler not starting"

**Solution**: Check tokio runtime is started before scheduler.start()

---

## 10. Checklist

- [ ] Dependencies added
- [ ] Module structure created
- [ ] Config compiles
- [ ] Store creates database
- [ ] Scheduler starts
- [ ] Executor runs tools
- [ ] Manager orchestrates
- [ ] Tasks implemented
- [ ] Notifications work
- [ ] Commands registered
- [ ] Frontend integrated
- [ ] End-to-end test passes
