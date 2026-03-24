# Integration Specification

## G2, G3, G5: How System Connects to Existing App

---

## 1. Overview

This document describes how the System Agent module integrates with the existing application architecture.

---

## 2. Integration Points

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           INTEGRATION POINTS                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. APP STATE                                                               │
│     • Add SystemManager to Tauri app state                                  │
│     • Initialize on startup, shutdown on exit                              │
│                                                                              │
│  2. TOOL REGISTRY                                                           │
│     • Get ToolRegistry instance for executor                               │
│     • Share between AgentState and SystemManager                          │
│                                                                              │
│  3. AGENT STORE (READ)                                                      │
│     • Review task needs to query agent_sessions                            │
│     • Pass SessionStore to SystemManager                                   │
│                                                                              │
│  4. NOTIFICATIONS                                                           │
│     • Use existing tauri-plugin-notification                              │
│     • Already initialized in lib.rs                                        │
│                                                                              │
│  5. COMMANDS                                                                │
│     • Register system commands in lib.rs                                   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Dependencies

### 3.1 Rust Crate Dependencies

```toml
# Cargo.toml additions
[dependencies]
tokio-cron-scheduler = "0.13"
async-trait = "0.1"  # For Task trait
```

### 3.2 Internal Dependencies

The system module depends on:

| Module | Purpose |
|--------|---------|
| `tools` | ToolRegistry for execution |
| `storage` | SQLite operations |
| `tauri` | App handle, notifications |

---

## 4. lib.rs Integration

### 4.1 Current State

```rust
// Current lib.rs setup
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Initialize databases
            let config_db = data_dir.join("coder_config.db");
            let chat_db = data_dir.join("coder_chat.db");
            let agent_db = data_dir.join("coder_agents.db");
            
            // Initialize states
            let nlp_state = NlpState::new(...)?;
            let chat_state = ChatState::new(...)?;
            let config_state = ConfigState::new(...)?;
            let llm_state = LlmState::new(...)?;
            let agent_state = AgentState::new(...)?;
            
            // Manage states
            app.manage(nlp_state);
            app.manage(chat_state);
            app.manage(config_state);
            app.manage(llm_state);
            app.manage(agent_state);
            
            // Plugins
            app.handle().plugin(tauri_plugin_notification::init())?;
            app.handle().plugin(tauri_plugin_dialog::init())?;
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![...])
        .run(...)
}
```

### 4.2 Required Changes

```rust
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // === EXISTING SETUP (unchanged) ===
            let data_dir = get_app_data_dir(app.handle())?;
            let config_db = data_dir.join("coder_config.db");
            let chat_db = data_dir.join("coder_chat.db");
            let agent_db = data_dir.join("coder_agents.db");
            
            // Initialize existing states
            let nlp_state = NlpState::new(&config_db.to_string_lossy(), &context_db.to_string_lossy())?;
            let chat_state = ChatState::new(&chat_db.to_string_lossy())?;
            let config_state = ConfigState::new(&config_db.to_string_lossy())?;
            let llm_state = LlmState::new(&config_db.to_string_lossy())?;
            let agent_state = AgentState::new(&agent_db.to_string_lossy())?;
            
            // === NEW: System Database ===
            let system_db = data_dir.join("coder_system.db");
            let system_store = SystemStore::new(&system_db.to_string_lossy())?;
            
            // === NEW: Create ToolRegistry ===
            // Need to expose from AgentState or create here
            let tool_registry = agent_state.get_tool_registry()?;
            
            // === NEW: System Manager ===
            let system_config = SystemConfig::default();
            let system_manager = SystemManager::new(
                app.handle().clone(),
                system_store,
                tool_registry,
                agent_state.get_session_store()?,  // For review task
                system_config,
            )?;
            
            // Manage all states
            app.manage(nlp_state);
            app.manage(chat_state);
            app.manage(config_state);
            app.manage(llm_state);
            app.manage(agent_state);
            app.manage(system_manager);
            
            // === NEW: Start System Manager ===
            let system_manager_clone = system_manager.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = system_manager_clone.start().await {
                    log::error!("Failed to start system manager: {}", e);
                }
            });
            
            // === EXISTING PLUGINS (unchanged) ===
            app.handle().plugin(tauri_plugin_notification::init())?;
            app.handle().plugin(tauri_plugin_dialog::init())?;
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ... existing commands ...
            
            // === NEW: System Commands ===
            commands::system::get_system_config,
            commands::system::update_system_config,
            commands::system::run_system_task,
            commands::system::get_system_task_status,
            commands::system::get_system_history,
            commands::system::pause_system,
            commands::system::resume_system,
        ])
        .build(...)
        .expect("error while running tauri application");
}
```

---

## 5. AgentState Changes

### 5.1 Required Additions

The current `AgentState` needs to expose:

```rust
impl AgentState {
    /// Get the tool registry for system tasks
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.runtime.tool_registry
    }
    
    /// Get the session store for review task
    pub fn session_store(&self) -> &SessionStore {
        &self.store
    }
}
```

### 5.2 Alternative: Create ToolRegistry in lib.rs

If ToolRegistry is complex to extract, create it in lib.rs:

```rust
// In lib.rs
let tool_registry = ToolRegistry::new(
    app.handle().clone(),
    Some(workspace_root.clone()),
)?;
tool_registry.register_all_tools();
```

---

## 6. System Manager Constructor

### 6.1 Signature

```rust
impl SystemManager {
    pub fn new(
        app_handle: tauri::AppHandle,
        store: SystemStore,
        tool_registry: ToolRegistry,
        agent_store: SessionStore,  // For review task
        config: SystemConfig,
    ) -> Result<Self, ManagerError> {
        // Initialize components
        let scheduler = TaskScheduler::new()?;
        let executor = SystemExecutor::new(tool_registry);
        let notifier = SystemNotifier::new(
            app_handle.clone(),
            config.notifications.clone(),
        );
        
        Ok(Self {
            store: Arc::new(store),
            tool_registry,
            agent_store: Arc::new(agent_store),
            scheduler: Arc::new(scheduler),
            executor: Arc::new(executor),
            notifier: Arc::new(notifier),
            config: Arc::new(RwLock::new(config)),
            app_handle,
            is_running: Arc::new(RwLock::new(false)),
            is_paused: Arc::new(RwLock::new(false)),
        })
    }
}
```

---

## 7. App Shutdown Handling

### 7.1 Graceful Shutdown

```rust
// In lib.rs - handle window close
// The SystemManager should handle shutdown when app closes

impl Drop for SystemManager {
    fn drop(&mut self) {
        // Signal shutdown
        let _ = self.shutdown();
    }
}

// In SystemManager
impl SystemManager {
    pub async fn shutdown(&self) -> Result<(), ManagerError> {
        log::info!("Shutting down SystemManager...");
        
        // Stop scheduler
        self.scheduler.stop().await?;
        
        // Mark as not running
        *self.is_running.write().await = false;
        
        log::info!("SystemManager shutdown complete");
        Ok(())
    }
}
```

---

## 8. State Management

### 8.1 SystemManager in Tauri State

```rust
// commands/system.rs
#[tauri::command]
pub async fn get_system_config(
    state: State<'_, SystemManager>,
) -> Result<SystemConfig, String> {
    state.get_config().await.map_err(|e| e.to_string())
}
```

---

## 9. Error Handling

### 9.1 Startup Errors

If SystemManager fails to start:

```rust
let system_manager_clone = system_manager.clone();
tauri::async_runtime::spawn(async move {
    if let Err(e) = system_manager_clone.start().await {
        log::error!("Failed to start system manager: {}", e);
        // Don't crash app - system is optional
    }
});
```

### 9.2 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ManagerError {
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    
    #[error("Scheduler error: {0}")]
    Scheduler(#[from] SchedulerError),
    
    #[error("Executor error: {0}")]
    Executor(#[from] ExecutorError),
    
    #[error("Not running")]
    NotRunning,
    
    #[error("Configuration error: {0}")]
    Config(String),
}
```

---

## 10. Configuration Storage

### 10.1 Config Location

System configuration is stored in `system_schedules` table:

```sql
CREATE TABLE system_schedules (
    task_type TEXT PRIMARY KEY,
    schedule TEXT NOT NULL,
    enabled INTEGER DEFAULT 1,
    last_run TEXT,
    next_run TEXT,
    failure_count INTEGER DEFAULT 0,
    config TEXT  -- JSON for task-specific config
);
```

### 10.2 Loading Config

```rust
impl SystemManager {
    async fn load_config(&self) -> Result<SystemConfig, ManagerError> {
        // Load from store or return default
        let schedule = self.store.get_schedule("sanitize")?;
        
        if let Some(schedule) = schedule {
            // Reconstruct config from schedules
            // ...
        }
        
        Ok(SystemConfig::default())
    }
}
```

---

## 11. Testing Integration

### 11.1 Mock Dependencies

For testing, use trait objects:

```rust
pub trait Store: Send + Sync {
    fn create_session(&self, ...) -> Result<...>;
    fn update_session_status(&self, ...) -> Result<...>;
}

pub trait ToolRunner: Send + Sync {
    async fn execute(&self, ...) -> Result<...>;
}
```

---

## 12. Summary

| Integration Point | Approach |
|-------------------|----------|
| App State | Add via `app.manage()` |
| ToolRegistry | Expose from AgentState or create in lib.rs |
| Agent Store | Pass SessionStore to SystemManager |
| Notifications | Use existing tauri-plugin-notification |
| Commands | Register in lib.rs invoke_handler |
| Shutdown | Graceful shutdown in Drop impl |
