---
name: tauri
description: Tauri v2 desktop application development patterns. Use when working with Rust backend, Tauri commands, IPC, window management, or native desktop features.
---

# Tauri v2 Development Skill

Guide for developing the Rust backend of the Coder desktop application using Tauri v2.

## Core Concepts

### Application Architecture

Tauri v2 follows a multi-process architecture:
- **Core Process**: Rust backend with full system access
- **WebView**: Frontend (Svelte) running in the OS webview
- **IPC**: Communication bridge between processes via commands and events

### Commands (Rust → Frontend callable)

Commands are Rust functions exposed to the frontend via `#[tauri::command]`:

```rust
use tauri::command;

#[command]
async fn fetch_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let projects = state.projects.lock().await.clone();
    Ok(projects)
}
```

Register commands in `lib.rs`:
```rust
fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            fetch_projects,
            create_project,
            delete_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### State Management

Use `tauri::State` for sharing state across commands:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub projects: Arc<Mutex<Vec<Project>>>,
    pub settings: Arc<Mutex<Settings>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            projects: Arc::new(Mutex::new(Vec::new())),
            settings: Arc::new(Mutex::new(Settings::default())),
        }
    }
}

#[command]
async fn update_settings(
    state: State<'_, AppState>,
    new_settings: Settings,
) -> Result<(), String> {
    let mut settings = state.settings.lock().await;
    *settings = new_settings;
    Ok(())
}
```

### Error Handling

Return `Result<T, String>` from commands for proper error propagation:

```rust
#[command]
async fn read_file(path: String) -> Result<String, String> {
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))
}
```

For detailed error patterns, see [error-handling.md](error-handling.md).

## IPC Patterns

### Request-Response

Standard command invocation from frontend:
```typescript
import { invoke } from '@tauri-apps/api/core';

const projects = await invoke<Project[]>('fetch_projects');
```

### Events (Bidirectional)

Rust can emit events to frontend:
```rust
use tauri::Emitter;

#[command]
async fn process_file(app: tauri::AppHandle, path: String) -> Result<(), String> {
    // Process file...
    app.emit("file-processed", &result).map_err(|e| e.to_string())?;
    Ok(())
}
```

Frontend listens:
```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen('file-processed', (event) => {
    console.log('File processed:', event.payload);
});

// Cleanup when done
unlisten();
```

## Window Management

### Creating Windows

```rust
use tauri::WebviewWindowBuilder;

#[command]
async fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("settings.html".into())
    )
    .title("Settings")
    .inner_size(600.0, 400.0)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}
```

### Window Labels

Use descriptive window labels for multi-window apps:
- `"main"` - Primary application window
- `"settings"` - Settings/preferences window
- `"about"` - About dialog

## Security

### Command Allowlisting

Define allowed commands in `tauri.conf.json`:
```json
{
  "security": {
    "csp": "default-src 'self'",
    "dangerousDisableAssetCspModification": false
  }
}
```

### Input Validation

Always validate inputs in commands:
```rust
#[command]
async fn create_project(name: String, path: String) -> Result<Project, String> {
    if name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    if !std::path::Path::new(&path).exists() {
        return Err("Invalid project path".to_string());
    }
    // Create project...
}
```

## Async Patterns

### Using Tokio

Tauri v2 uses Tokio for async operations:
```rust
#[command]
async fn fetch_remote_data(url: String) -> Result<Vec<u8>, String> {
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let bytes = response.bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(bytes.to_vec())
}
```

### Blocking Operations

Use `spawn_blocking` for CPU-intensive work:
```rust
#[command]
async fn process_large_file(path: String) -> Result<ProcessedData, String> {
    tokio::task::spawn_blocking(move || {
        // CPU-intensive processing here
        let data = std::fs::read(&path).map_err(|e| e.to_string())?;
        // Process data...
        Ok(processed)
    })
    .await
    .map_err(|e| e.to_string())?
}
```

## File Structure

Organize Rust code in the `src-tauri/` directory:
```
src-tauri/
├── src/
│   ├── main.rs          # Entry point
│   ├── lib.rs           # Library root, command registration
│   ├── commands/        # Command modules
│   │   ├── mod.rs
│   │   ├── projects.rs
│   │   └── settings.rs
│   ├── state.rs         # AppState definition
│   └── error.rs         # Error types
├── Cargo.toml
├── tauri.conf.json
└── capabilities/
    └── default.json
```

## Dependencies

Key crates for the Coder project:
```toml
[dependencies]
tauri = { version = "2", features = ["devtools"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
```

## Best Practices

1. **Keep commands focused**: Each command does one thing well
2. **Use typed state**: Wrap state in typed structs with Arc<Mutex>
3. **Return Result types**: Properly propagate errors to frontend
4. **Validate all inputs**: Never trust input from frontend
5. **Use async appropriately**: Tokio for I/O, spawn_blocking for CPU work
6. **Organize by domain**: Group commands by feature area

## Additional Resources

- For error handling patterns, see [error-handling.md](error-handling.md)
- For command examples, see [examples/](examples/)
- For Tauri v2 official docs: https://v2.tauri.app/
