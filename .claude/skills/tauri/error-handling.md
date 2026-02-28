# Tauri Error Handling Patterns

## Command Error Types

### Simple String Errors

For quick prototyping, return `Result<T, String>`:

```rust
#[command]
async fn read_config(path: String) -> Result<Config, String> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read config: {}", e))?;

    toml::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))
}
```

### Custom Error Types

For production code, define custom error types:

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum CommandError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// Implement From for common errors
impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => CommandError::FileNotFound(e.to_string()),
            std::io::ErrorKind::PermissionDenied => CommandError::PermissionDenied(e.to_string()),
            _ => CommandError::Internal(e.to_string()),
        }
    }
}

#[command]
async fn read_file(path: String) -> Result<String, CommandError> {
    let content = tokio::fs::read_to_string(&path).await?;
    Ok(content)
}
```

## Error Propagation

### Using anyhow

For flexible error handling:

```rust
use anyhow::{Context, Result};

#[command]
async fn load_project(path: String) -> Result<Project, String> {
    internal_load_project(&path)
        .await
        .map_err(|e| e.to_string())
}

async fn internal_load_project(path: &str) -> Result<Project> {
    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read project file")?;

    let project: Project = toml::from_str(&content)
        .context("Failed to parse project configuration")?;

    Ok(project)
}
```

### Error Context

Always add context to errors:

```rust
#[command]
async fn save_settings(settings: Settings) -> Result<(), String> {
    let path = settings.config_path();

    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    tokio::fs::write(&path, content)
        .await
        .map_err(|e| format!("Failed to write settings to {}: {}", path, e))?;

    Ok(())
}
```

## Frontend Error Handling

Match Rust error structure in TypeScript:

```typescript
interface CommandError {
  FileNotFound?: string;
  PermissionDenied?: string;
  InvalidInput?: string;
  Internal?: string;
}

async function invokeWithErrorHandling<T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (error) {
    if (typeof error === 'string') {
      throw new Error(error);
    }
    throw error;
  }
}

// Usage
try {
  const project = await invokeWithErrorHandling<Project>('load_project', { path });
} catch (error) {
  if (error.message.includes('not found')) {
    // Handle missing file
  } else {
    // Handle other errors
  }
}
```

## Validation Errors

Return detailed validation errors:

```rust
#[derive(Serialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[command]
async fn create_user(name: String, email: String) -> Result<User, Vec<ValidationError>> {
    let mut errors = Vec::new();

    if name.is_empty() {
        errors.push(ValidationError {
            field: "name".to_string(),
            message: "Name cannot be empty".to_string(),
        });
    }

    if !email.contains('@') {
        errors.push(ValidationError {
            field: "email".to_string(),
            message: "Invalid email format".to_string(),
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Create user...
    Ok(user)
}
```

## Logging Errors

Log errors for debugging:

```rust
use tracing::{error, info, instrument};

#[command]
#[instrument(skip(state))]
async fn process_file(state: State<'_, AppState>, path: String) -> Result<ProcessedFile, String> {
    info!("Processing file: {}", path);

    match internal_process(&path).await {
        Ok(result) => {
            info!("Successfully processed: {}", path);
            Ok(result)
        }
        Err(e) => {
            error!("Failed to process {}: {}", path, e);
            Err(format!("Processing failed: {}", e))
        }
    }
}
```
