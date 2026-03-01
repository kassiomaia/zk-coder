# Backend Implementation Plan: LLM Configuration Flow (Tauri)

> **Status:** READY TO IMPLEMENT
> **Generated:** 2026-03-01
> **Related:** `prototype-implementation-plan.md`

## ⚠️ MANDATORY: Prototype Reference

**The backend implementation MUST align with the behavior demonstrated in the prototype.**

Reference `prototypes/config-flow/js/state.js` for:
- Expected state properties and their types
- Method signatures and return values
- Validation logic and state transitions
- API call simulation patterns

### Key Prototype Behaviors to Implement

| Prototype Method | File Reference | Backend Command |
|------------------|----------------|-----------------|
| `validateApiKey()` | state.js:279-303 | `validate_api_key` |
| `testConnection()` | state.js:202-228 | `test_connection` |
| `fetchModels()` | state.js:230-263 | `fetch_models` |
| `saveProvider()` | state.js:305-310 | `save_provider` + `store_api_key` |
| `addCustomProvider()` | state.js:178-200 | `save_provider` |
| `removeProvider()` | state.js:265-277 | `remove_provider` + `delete_api_key` |
| `selectModel()` | state.js:312-314 | `set_operation_default` |
| `getAvailableModels()` | state.js:316-346 | Filter logic in `get_models` |

### Pre-Implementation Checklist

- [ ] Review `prototypes/config-flow/js/state.js` for all method behaviors
- [ ] Review `prototypes/config-flow/README.md` for functional requirements
- [ ] Understand validation states (info → success/error)
- [ ] Understand connection test flow (requires API key first)
- [ ] Understand model fetch flow (requires successful connection test)

---

## Summary

Backend implementation for the LLM configuration flow using Tauri commands and SQLite storage. This plan covers Rust implementation for provider management, API key storage in OS keychain, and model fetching from provider APIs.

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        Svelte Frontend                           │
├──────────────────────────────────────────────────────────────────┤
│  configStore.ts  │  api/config.ts  │  Types (config.ts)          │
└────────┬─────────┴────────┬─────────┴────────────────────────────┘
         │ invoke()         │ invoke()
         ▼                  ▼
┌──────────────────────────────────────────────────────────────────┐
│                        Tauri Commands                            │
├──────────────────────────────────────────────────────────────────┤
│  providers.rs    │  config.rs      │  keychain.rs               │
└────────┬─────────┴────────┬─────────┴────────────────────────────┘
         │                  │
         ▼                  ▼
┌──────────────────────────────────────────────────────────────────┐
│                        Storage Layer                             │
├──────────────────────────────────────────────────────────────────┤
│  ProviderStore   │  ModelStore     │  ConfigStore   │ Keychain  │
│  (SQLite)        │  (SQLite)       │  (SQLite)      │ (OS)      │
└──────────────────────────────────────────────────────────────────┘
```

---

## Data Models (Rust)

```rust
// src-tauri/src/models/config.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub is_custom: bool,
    pub api_type: ApiType,
    pub base_url: Option<String>,
    pub is_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiType {
    OpenAI,
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub capabilities: ModelCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub analysis: bool,
    pub code: bool,
    pub vision: bool,
    pub audio: bool,
    pub tips: bool,
    pub review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationDefault {
    pub operation: OperationType,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    Analysis,
    Code,
    Vision,
    Audio,
    Tips,
    Review,
}

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct SaveProviderRequest {
    pub id: Option<String>,
    pub name: String,
    pub icon: String,
    pub api_type: ApiType,
    pub base_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct FetchModelsResult {
    pub success: bool,
    pub models: Vec<String>,
    pub message: Option<String>,
}
```

---

## Tauri Commands

### 1. Provider Commands (`commands/providers.rs`)

```rust
use tauri::State;
use crate::storage::ProviderStore;
use crate::models::*;

/// Get all configured providers
#[tauri::command]
pub async fn get_providers(
    store: State<'_, ProviderStore>,
) -> Result<Vec<Provider>, String> {
    store.get_all().map_err(|e| e.to_string())
}

/// Save a provider (create or update)
#[tauri::command]
pub async fn save_provider(
    store: State<'_, ProviderStore>,
    request: SaveProviderRequest,
) -> Result<Provider, String> {
    let provider = Provider {
        id: request.id.unwrap_or_else(|| generate_id(&request.name)),
        name: request.name,
        icon: request.icon,
        is_custom: true,
        api_type: request.api_type,
        base_url: request.base_url,
        is_configured: false,
    };
    store.save(&provider).map_err(|e| e.to_string())?;
    Ok(provider)
}

/// Remove a custom provider
#[tauri::command]
pub async fn remove_provider(
    store: State<'_, ProviderStore>,
    provider_id: String,
) -> Result<(), String> {
    store.delete(&provider_id).map_err(|e| e.to_string())
}

/// Validate API key with provider
#[tauri::command]
pub async fn validate_api_key(
    provider_id: String,
    api_key: String,
    api_type: ApiType,
    base_url: Option<String>,
) -> Result<ValidationResult, String> {
    // Make test API call based on provider type
    match api_type {
        ApiType::OpenAI => validate_openai_key(&api_key, base_url).await,
        ApiType::Anthropic => validate_anthropic_key(&api_key, base_url).await,
    }
}

/// Test connection to custom provider
#[tauri::command]
pub async fn test_connection(
    base_url: String,
    api_key: String,
    api_type: ApiType,
) -> Result<ValidationResult, String> {
    // Test the connection endpoint
    validate_api_key("custom".to_string(), api_key, api_type, Some(base_url)).await
}

/// Fetch available models from provider
#[tauri::command]
pub async fn fetch_models(
    store: State<'_, ProviderStore>,
    provider_id: String,
    api_key: String,
    api_type: ApiType,
    base_url: Option<String>,
) -> Result<FetchModelsResult, String> {
    let models = match api_type {
        ApiType::OpenAI => fetch_openai_models(&api_key, base_url).await?,
        ApiType::Anthropic => fetch_anthropic_models(&api_key, base_url).await?,
    };

    // Store fetched models
    store.save_models(&provider_id, &models).map_err(|e| e.to_string())?;

    Ok(FetchModelsResult {
        success: true,
        models: models.iter().map(|m| m.name.clone()).collect(),
        message: None,
    })
}

fn generate_id(name: &str) -> String {
    format!("custom-{}", name.to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', ""))
}
```

### 2. Config Commands (`commands/config.rs`)

```rust
use tauri::State;
use crate::storage::ConfigStore;
use crate::models::*;

/// Get operation default model assignments
#[tauri::command]
pub async fn get_operation_defaults(
    store: State<'_, ConfigStore>,
) -> Result<Vec<OperationDefault>, String> {
    store.get_all().map_err(|e| e.to_string())
}

/// Set default model for an operation
#[tauri::command]
pub async fn set_operation_default(
    store: State<'_, ConfigStore>,
    operation: OperationType,
    model_id: String,
) -> Result<(), String> {
    store.set(&operation, &model_id).map_err(|e| e.to_string())
}
```

### 3. Keychain Commands (`commands/keychain.rs`)

```rust
use keyring::Entry;

const SERVICE_NAME: &str = "coder-ai";

/// Store API key in OS keychain
#[tauri::command]
pub async fn store_api_key(
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, &provider_id)
        .map_err(|e| format!("Keychain error: {}", e))?;
    entry.set_password(&api_key)
        .map_err(|e| format!("Failed to store key: {}", e))
}

/// Retrieve API key from keychain
#[tauri::command]
pub async fn retrieve_api_key(
    provider_id: String,
) -> Result<String, String> {
    let entry = Entry::new(SERVICE_NAME, &provider_id)
        .map_err(|e| format!("Keychain error: {}", e))?;
    entry.get_password()
        .map_err(|e| format!("Failed to retrieve key: {}", e))
}

/// Delete API key from keychain
#[tauri::command]
pub async fn delete_api_key(
    provider_id: String,
) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, &provider_id)
        .map_err(|e| format!("Keychain error: {}", e))?;
    entry.delete_credential()
        .map_err(|e| format!("Failed to delete key: {}", e))
}
```

---

## Storage Layer

### ProviderStore (`storage/provider_store.rs`)

```rust
use rusqlite::{Connection, params};
use crate::models::*;

pub struct ProviderStore {
    conn: Connection,
}

impl ProviderStore {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn get_all(&self) -> Result<Vec<Provider>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, is_custom, api_type, base_url, is_configured
             FROM providers ORDER BY name"
        )?;

        let providers = stmt.query_map([], |row| {
            Ok(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                is_custom: row.get(3)?,
                api_type: row.get::<_, String>(4)?.parse().unwrap_or(ApiType::OpenAI),
                base_url: row.get(5)?,
                is_configured: row.get(6)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(providers)
    }

    pub fn save(&self, provider: &Provider) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO providers (id, name, icon, is_custom, api_type, base_url, is_configured)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                icon = excluded.icon,
                api_type = excluded.api_type,
                base_url = excluded.base_url,
                is_configured = excluded.is_configured,
                updated_at = CURRENT_TIMESTAMP",
            params![
                provider.id,
                provider.name,
                provider.icon,
                provider.is_custom,
                provider.api_type.to_string(),
                provider.base_url,
                provider.is_configured,
            ],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM providers WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn set_configured(&self, id: &str, configured: bool) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE providers SET is_configured = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![configured, id],
        )?;
        Ok(())
    }

    pub fn save_models(&self, provider_id: &str, models: &[Model]) -> Result<(), rusqlite::Error> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM models WHERE provider_id = ?1", [provider_id])?;

        for model in models {
            tx.execute(
                "INSERT INTO models (id, provider_id, name, supports_analysis, supports_code,
                 supports_vision, supports_audio, supports_tips, supports_review)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    model.id,
                    model.provider_id,
                    model.name,
                    model.capabilities.analysis,
                    model.capabilities.code,
                    model.capabilities.vision,
                    model.capabilities.audio,
                    model.capabilities.tips,
                    model.capabilities.review,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}
```

---

## API Validation Implementation

### OpenAI Validation (`api/openai.rs`)

```rust
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Deserialize)]
struct OpenAIModel {
    id: String,
}

pub async fn validate_openai_key(api_key: &str, base_url: Option<&str>) -> Result<ValidationResult, String> {
    let client = Client::new();
    let url = format!("{}/models", base_url.unwrap_or("https://api.openai.com/v1"));

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        Ok(ValidationResult {
            valid: true,
            message: "API key validated successfully".to_string(),
        })
    } else {
        Ok(ValidationResult {
            valid: false,
            message: format!("Validation failed: {}", response.status()),
        })
    }
}

pub async fn fetch_openai_models(api_key: &str, base_url: Option<&str>) -> Result<Vec<Model>, String> {
    let client = Client::new();
    let url = format!("{}/models", base_url.unwrap_or("https://api.openai.com/v1"));

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json::<OpenAIModelsResponse>()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(response.data.into_iter().map(|m| Model {
        id: format!("openai-{}", m.id),
        provider_id: "openai".to_string(),
        name: m.id,
        capabilities: infer_capabilities(&m.id),
    }).collect())
}
```

### Anthropic Validation (`api/anthropic.rs`)

```rust
use reqwest::Client;

pub async fn validate_anthropic_key(api_key: &str, base_url: Option<&str>) -> Result<ValidationResult, String> {
    let client = Client::new();
    let url = format!("{}/v1/messages", base_url.unwrap_or("https://api.anthropic.com"));

    // Anthropic doesn't have a validation endpoint, so we make a minimal request
    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-3-haiku-20240307",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}]
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    // 200 or 400 (rate limit) means key is valid
    // 401 means invalid key
    let status = response.status();
    if status.as_u16() == 401 {
        Ok(ValidationResult {
            valid: false,
            message: "Invalid API key".to_string(),
        })
    } else {
        Ok(ValidationResult {
            valid: true,
            message: "API key validated successfully".to_string(),
        })
    }
}

pub async fn fetch_anthropic_models(_api_key: &str, _base_url: Option<&str>) -> Result<Vec<Model>, String> {
    // Anthropic doesn't have a models endpoint, return known models
    Ok(vec![
        Model {
            id: "anthropic-claude-opus".to_string(),
            provider_id: "anthropic".to_string(),
            name: "Claude Opus".to_string(),
            capabilities: ModelCapabilities {
                analysis: true, code: true, vision: true,
                audio: false, tips: true, review: true,
            },
        },
        Model {
            id: "anthropic-claude-sonnet".to_string(),
            provider_id: "anthropic".to_string(),
            name: "Claude Sonnet".to_string(),
            capabilities: ModelCapabilities {
                analysis: true, code: true, vision: true,
                audio: false, tips: true, review: true,
            },
        },
        Model {
            id: "anthropic-claude-haiku".to_string(),
            provider_id: "anthropic".to_string(),
            name: "Claude Haiku".to_string(),
            capabilities: ModelCapabilities {
                analysis: true, code: true, vision: true,
                audio: false, tips: true, review: true,
            },
        },
    ])
}
```

---

## Database Migration

```sql
-- migration.sql

CREATE TABLE IF NOT EXISTS providers (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    icon          TEXT,
    is_custom     BOOLEAN DEFAULT FALSE,
    api_type      TEXT CHECK(api_type IN ('openai', 'anthropic')),
    base_url      TEXT,
    is_configured BOOLEAN DEFAULT FALSE,
    created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS models (
    id                  TEXT PRIMARY KEY,
    provider_id         TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    supports_analysis   BOOLEAN DEFAULT TRUE,
    supports_code       BOOLEAN DEFAULT TRUE,
    supports_vision     BOOLEAN DEFAULT FALSE,
    supports_audio      BOOLEAN DEFAULT FALSE,
    supports_tips       BOOLEAN DEFAULT TRUE,
    supports_review     BOOLEAN DEFAULT TRUE,
    last_fetched        TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS operation_defaults (
    operation   TEXT PRIMARY KEY CHECK(operation IN ('analysis', 'code', 'vision', 'audio', 'tips', 'review')),
    model_id    TEXT REFERENCES models(id) ON DELETE SET NULL
);

-- Seed built-in providers
INSERT OR IGNORE INTO providers (id, name, icon, is_custom, api_type, is_configured) VALUES
    ('anthropic', 'Anthropic', 'AC', FALSE, 'anthropic', FALSE),
    ('openai', 'OpenAI', 'OA', FALSE, 'openai', FALSE),
    ('google', 'Google', 'GC', FALSE, 'openai', FALSE);
```

---

## Command Registration

```rust
// src-tauri/src/lib.rs

fn run() {
    tauri::Builder::default()
        // ... other setup
        .invoke_handler(tauri::generate_handler![
            // Provider commands
            commands::providers::get_providers,
            commands::providers::save_provider,
            commands::providers::remove_provider,
            commands::providers::validate_api_key,
            commands::providers::test_connection,
            commands::providers::fetch_models,
            // Config commands
            commands::config::get_operation_defaults,
            commands::config::set_operation_default,
            // Keychain commands
            commands::keychain::store_api_key,
            commands::keychain::retrieve_api_key,
            commands::keychain::delete_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Implementation Steps

### Phase 1: Data Models & Storage
- [ ] Create `models/config.rs` with all data types
- [ ] Create database migration file
- [ ] Implement `ProviderStore` with SQLite operations
- [ ] Implement `ModelStore` with SQLite operations
- [ ] Implement `ConfigStore` with SQLite operations
- [ ] Write unit tests for stores

### Phase 2: Keychain Integration
- [ ] Add `keyring` dependency to Cargo.toml
- [ ] Implement `keychain.rs` commands
- [ ] Test on macOS, Windows, Linux
- [ ] Handle keychain errors gracefully

### Phase 3: API Validation
- [ ] Add `reqwest` dependency to Cargo.toml
- [ ] Implement OpenAI validation and model fetching
- [ ] Implement Anthropic validation and model fetching
- [ ] Add error handling for network failures
- [ ] Add timeout configuration

### Phase 4: Commands & Integration
- [ ] Implement all provider commands
- [ ] Implement all config commands
- [ ] Register commands in `lib.rs`
- [ ] Write integration tests

### Phase 5: Testing
- [ ] Unit tests for all stores
- [ ] Mock API tests for validation
- [ ] Integration tests for command flow
- [ ] Manual testing on all platforms

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Keychain not available on some Linux distros | Fallback to encrypted file storage |
| API rate limiting during validation | Add retry logic with exponential backoff |
| Network timeouts | Configurable timeout with sensible defaults |
| Custom provider API variations | Document supported API patterns |

---

## Acceptance Criteria

### Functional Requirements
- [ ] All Tauri commands work correctly
- [ ] API keys stored securely in OS keychain
- [ ] Provider validation works for all supported providers
- [ ] Model fetching and caching works correctly
- [ ] Operation defaults persist across app restarts
- [ ] Unit and integration tests pass
- [ ] Works on macOS, Windows, and Linux

### Prototype Behavior Compliance (MANDATORY)
- [ ] `validate_api_key` returns validation states matching prototype (info, success, error)
- [ ] `test_connection` requires API key (prototype: shows error if empty)
- [ ] `fetch_models` requires successful connection test first (prototype: enforces order)
- [ ] `fetch_models` returns model names matching provider type (prototype: OpenAI vs Anthropic models)
- [ ] Model capabilities are correctly inferred based on model name patterns
- [ ] Provider removal clears associated selected models (prototype: `removeProvider` behavior)

---

## Prototype Behavior Verification

### Validation Flow (from state.js:279-303)

Expected behavior from prototype:
```
1. Empty API key → error state with "Please enter an API key"
2. Validation starts → info state with "Validating..." and spinner icon
3. Validation succeeds → success state with "API key validated successfully!"
4. Validation fails → error state with "Invalid API key..."
```

Backend implementation must return states that frontend can map to these.

### Connection Test Flow (from state.js:202-228)

Expected behavior from prototype:
```
1. Empty API key → error, "Please enter an API key first"
2. Test starts → loading state
3. Test succeeds → success state (key length >= 5 in prototype demo)
4. Test fails → error state with message
```

### Model Fetch Flow (from state.js:230-263)

Expected behavior from prototype:
```
1. Empty API key → error, "Please enter an API key first"
2. No prior connection test → error, "Please test connection first"
3. Fetch starts → loading state
4. Fetch succeeds → models populated based on apiType:
   - OpenAI: "GPT-4 Turbo, GPT-4, GPT-3.5 Turbo"
   - Anthropic: "Claude Opus, Claude Sonnet, Claude Haiku"
```

### Provider Removal Flow (from state.js:265-277)

Expected behavior from prototype:
```
1. Remove provider from list
2. Clear any selected models that reference this provider:
   - Check each operation in selectedModels
   - If modelId starts with providerId, set to null
```
