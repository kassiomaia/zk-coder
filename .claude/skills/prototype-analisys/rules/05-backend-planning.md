# Backend Planning Rules (Tauri)

These rules govern the analysis and planning of backend requirements for this Tauri desktop application.

## Tauri Architecture Overview

This application uses **Tauri v2** with:
- **Backend:** Rust (`src-tauri/src/`)
- **Frontend:** Svelte + TypeScript (`src-ui/`)
- **Communication:** Tauri commands (IPC between frontend and backend)
- **Storage:** SQLite via dedicated stores

## Backend Requirement Detection

A prototype likely requires Tauri backend changes if:

### 1. Data Persistence
- State includes data that should persist across sessions
- User-created content needs storage (conversations, threads, messages)
- Settings/configuration need to be saved
- **Solution:** Add to existing SQLite store or create new store

### 2. System Integration
- File system access (read/write files)
- Native dialogs (file picker, save dialog)
- System information (OS, paths, resources)
- Window management (new windows, tray icons)
- **Solution:** Use Tauri plugins or create custom commands

### 3. External API Integration
- Prototype simulates API calls to external services
- AI provider integration (Claude, OpenAI, etc.)
- HTTP requests to remote servers
- **Solution:** Add to `src-tauri/src/ai/` or create new module

### 4. Background Processing
- Long-running operations
- Async data processing
- Scheduled tasks
- **Solution:** Use tokio async runtime in Rust commands

### 5. Security-Sensitive Operations
- API key storage
- Credential management
- Encryption/decryption
- **Solution:** Use OS keychain via Tauri plugins or secure storage

## Backend Planning Process

### Step 1: Identify Backend Needs
Review prototype for:
- State that needs persistence → SQLite store
- Simulated API calls in `state.js` → Tauri commands
- File operations → Tauri file system APIs
- External service calls → New module in `src-tauri/src/`

### Step 2: Categorize by Implementation Area

| Category | Location in Codebase | Example |
|----------|---------------------|---------|
| Chat/Messages | `commands/chat.rs`, `storage/chat_store.rs` | New message type |
| NLP/AI | `commands/nlp.rs`, `nlp/` | Intent classification |
| Storage | `storage/` | New data model |
| AI Providers | `ai/` | New LLM integration |
| Utilities | `utils/` | Helper functions |

### Step 3: Ask Clarification
If backend needs are detected, ask:
```
"This feature appears to require Tauri backend changes for [reason].
Should I create a separate backend implementation plan?

Detected needs:
- [ ] Data persistence (SQLite)
- [ ] New Tauri commands
- [ ] File system access
- [ ] External API integration
- [ ] Other: [specify]"
```

## Tauri Command Implementation

### Command Definition Pattern

```rust
// In src-tauri/src/commands/<module>.rs

use serde::{Deserialize, Serialize};
use tauri::State;
use crate::storage::YourStore;

/// Request/response types
#[derive(Debug, Serialize, Deserialize)]
pub struct YourRequest {
    pub field1: String,
    pub field2: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YourResponse {
    pub success: bool,
    pub data: Vec<String>,
}

/// Tauri command
#[tauri::command]
pub async fn your_command(
    state: State<'_, YourState>,
    request: YourRequest,
) -> Result<YourResponse, String> {
    // Implementation
    Ok(YourResponse {
        success: true,
        data: vec![],
    })
}
```

### Registering Commands

Commands must be registered in `src-tauri/src/lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    // Existing commands...
    commands::your_module::your_command,
])
```

### State Management

State is managed via Tauri's state system:

```rust
// Define state wrapper
pub struct YourState {
    store: Mutex<YourStore>,
}

impl YourState {
    pub fn new(db_path: &str) -> Result<Self, String> {
        Ok(Self {
            store: Mutex::new(YourStore::new(db_path)?),
        })
    }
}

// Register in lib.rs
.manage(your_state)
```

## Frontend Integration

### TypeScript Types

Create matching types in `src-ui/src/lib/types/`:

```typescript
// src-ui/src/lib/types/yourFeature.ts
export interface YourRequest {
  field1: string;
  field2: number;
}

export interface YourResponse {
  success: boolean;
  data: string[];
}
```

### Tauri Invoke

Use `@tauri-apps/api` in Svelte components:

```typescript
import { invoke } from '@tauri-apps/api/core';

async function callYourCommand(request: YourRequest): Promise<YourResponse> {
  return await invoke<YourResponse>('your_command', { request });
}
```

### Error Handling

```typescript
try {
  const result = await invoke<YourResponse>('your_command', { request });
  // Handle success
} catch (error) {
  // Handle error (string from Rust)
  console.error('Command failed:', error);
}
```

## Backend Plan Structure (Tauri)

When backend planning is required:

### Required Sections

```markdown
## Backend Implementation (Tauri)

> **Status:** REQUIRED / NOT REQUIRED / NEEDS CLARIFICATION

## Backend Plan Status

| Status | Meaning |
|--------|---------|
| **NOT REQUIRED** | Feature is frontend-only (no Rust changes) |
| **REQUIRED** | Tauri backend changes needed, plan included |
| **NEEDS CLARIFICATION** | Backend needs unclear, questions listed |
| **SEPARATE PLAN** | Backend plan in separate document |

## Frontend-Backend Coordination

### Order of Implementation

1. **Define Types First:** Rust structs and TypeScript interfaces
2. **Backend Implementation:** Storage, commands, registration
3. **Frontend Integration:** API wrapper, component integration
4. **Testing:** Rust unit tests, then integration tests

### Type Synchronization

Keep Rust and TypeScript types in sync:
- Use same field names (snake_case in Rust → camelCase in TS via serde)
- Document type mappings in plan
- Consider using `ts-rs` crate for automatic TypeScript generation

### Testing Strategy

1. **Rust Unit Tests:** Test storage and logic in isolation
2. **Command Tests:** Test command handlers with mock state
3. **Integration Tests:** Test full flow from frontend to backend
4. **Manual Testing:** Use Tauri dev tools

## No Backend Scenario

If analysis confirms no backend is needed:
- State clearly: "Backend Implementation: NOT REQUIRED"
- Explain why (e.g., "Feature uses local Svelte state only")
- Note any future backend considerations

## Quick Reference

### When to Create New Command
- New data operation (CRUD)
- New system integration
- New async operation
- Security-sensitive operation

### When to Extend Existing Command
- Adding optional parameters
- Extending response fields
- Adding filtering/sorting

### When to Create New Storage
- New data domain (not fitting existing stores)
- Different persistence requirements
- Separate database file needed
