# LLM Chat Implementation Plan

> **ARCHIVED (2026-04-04)**: Historical plan from before command/module restructuring. References may include legacy file paths.

## Overview

This document outlines the phased implementation plan for connecting the chat interface to LLM providers using the "Code generation" operation configuration.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Frontend (Svelte)                              │
├─────────────────────────────────────────────────────────────────────────┤
│  ChatInput.svelte  →  +page.svelte  →  chatStore.sendMessageWithLlm()  │
│                                                           │              │
│                                                           ▼              │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    $lib/api/llm.ts                              │   │
│  │  - sendChatMessageStreaming(options)                           │   │
│  │  - Listens to 'llm:event' events                               │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ IPC (Tauri)
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       Rust Backend (Tauri)                              │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │              commands/llm.rs                                     │   │
│  │  - send_chat_message_streaming                                  │   │
│  │    ├─ Get model for 'code' operation                            │   │
│  │    ├─ Get provider & API key                                   │   │
│  │    ├─ Create LLM client                                        │   │
│  │    ├─ Stream response                                          │   │
│  │    └─ Emit 'llm:event' events                                  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ ConfigStore  │→ │  Keychain    │→ │  LlmClient   │→ │   OpenAI   │ │
│  │              │  │              │  │              │  │    API     │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

## Configuration Flow

To send a chat message, we need to:

1. **Get the model for 'code' operation**
   ```sql
   SELECT model_id FROM operation_defaults WHERE operation = 'code';
   ```

2. **Get the model details**
   ```sql
   SELECT id, provider_id, name FROM models WHERE id = ?;
   ```

3. **Get the provider details**
   ```sql
   SELECT id, api_type, base_url FROM providers WHERE id = ?;
   ```

4. **Get the API key from keychain**
   ```rust
   keychain::retrieve_api_key(provider_id)
   ```

5. **Create LLM client and stream response**
   ```rust
   let client = create_client(api_type, api_key, base_url);
   let stream = client.chat_stream(request);
   ```

## Phased Implementation

### Phase 1: Configuration Query Tests ✅

**Goal**: Verify we can read the "Code generation" operation configuration.

**Files to create/modify**:
- `src-tauri/src/commands/config.rs` - Add `get_code_operation_model()` command

**Tests**:
```rust
#[tokio::test]
async fn test_get_code_operation_model() {
    // Setup: Create in-memory store, seed providers, set code default
    let store = ConfigStore::in_memory().unwrap();
    let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
    store.save_models("openai", &[model]).unwrap();
    store.set_operation_default(OperationType::Code, "openai-gpt-4").unwrap();

    // Test: Get the code operation model
    let result = get_code_operation_model_impl(&store).unwrap();
    assert_eq!(result.name, "gpt-4");
    assert_eq!(result.provider_id, "openai");
}
```

**Acceptance criteria**:
- [ ] Command `get_code_operation_model` returns the model ID for 'code' operation
- [ ] Returns null/error if no model is configured for 'code'
- [ ] Test passes with in-memory database

---

### Phase 2: Provider and API Key Resolution

**Goal**: Get the full provider configuration and API key for the code model.

**Files to create/modify**:
- `src-tauri/src/commands/llm.rs` - Add helper functions
- `src-tauri/src/bin/test_provider_resolution.rs` - Test binary

**Implementation**:
```rust
/// Resolves the complete LLM configuration for the 'code' operation.
pub struct LlmConfig {
    pub model_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub api_type: ApiType,
    pub base_url: Option<String>,
    pub api_key: String,
}

pub async fn resolve_code_llm_config(
    config_store: &ConfigStore,
) -> Result<LlmConfig, String> {
    // 1. Get code operation default
    // 2. Get model details
    // 3. Get provider details
    // 4. Get API key from keychain
}
```

**Tests**:
```rust
#[tokio::test]
async fn test_resolve_code_llm_config() {
    // Setup: Store with provider, model, operation default, and API key
    // Test: Resolve returns complete configuration
}
```

**Acceptance criteria**:
- [ ] `resolve_code_llm_config()` returns complete configuration
- [ ] Returns error if any component is missing
- [ ] API key is retrieved from keychain
- [ ] Test demonstrates full configuration resolution

---

### Phase 3: Simple LLM API Test

**Goal**: Make a real API call to verify connectivity.

**Files to create/modify**:
- `src-tauri/src/bin/test_llm_connection.rs` - Test binary

**Test**:
```rust
#[tokio::test]
#[ignore]  // Only run with valid API key
async fn test_real_llm_request() {
    let config = resolve_code_llm_config(&store).await.unwrap();
    let client = create_client(config.api_type, config.api_key, config.base_url);

    let request = ChatRequest::new(&config.model_name, vec![
        Message::user("Say 'Hello, LLM!' and nothing else.")
    ]);

    let response = client.chat(request).await.unwrap();
    assert!(response.content.contains("Hello"));
}
```

**Acceptance criteria**:
- [ ] Test makes actual API call to provider
- [ ] Response is received and validated
- [ ] Test can be run manually with `cargo run --bin test_llm_connection`

---

### Phase 4: Streaming LLM Test

**Goal**: Test streaming response and event generation.

**Files to create/modify**:
- `src-tauri/src/bin/test_llm_streaming.rs` - Test binary

**Test**:
```rust
#[tokio::test]
#[ignore]
async fn test_llm_streaming() {
    let config = resolve_code_llm_config(&store).await.unwrap();
    let client = create_client(config.api_type, config.api_key, config.base_url);

    let request = ChatRequest::new(&config.model_name, vec![
        Message::user("Count from 1 to 5, one number per line.")
    ]).with_stream();

    let stream = client.chat_stream(request);
    let events = stream_to_events("test-123".to_string(), config.model_name, stream);

    use futures::StreamExt;
    let mut chunks = Vec::new();
    let mut event_types = Vec::new();

    while let Some(event) = events.next().await {
        event_types.push(std::mem::discriminant(&event));
        match &event {
            LlmEvent::Chunk { delta, .. } => chunks.push(delta.clone()),
            LlmEvent::Complete { content, .. } => {
                assert!(content.contains("1"));
                assert!(content.contains("5"));
            }
            _ => {}
        }
    }

    assert!(chunks.len() > 1); // Should receive multiple chunks
}
```

**Acceptance criteria**:
- [ ] Streaming works end-to-end
- [ ] Events are emitted: Started, Chunk(s), Complete
- [ ] Content is accumulated correctly

---

### Phase 5: Tauri Command Integration

**Goal**: Wire up the streaming command with code operation resolution.

**Files to modify**:
- `src-tauri/src/commands/llm.rs` - Update `send_chat_message_streaming`

**Changes**:
```rust
// Before: Model ID passed from frontend
pub async fn send_chat_message_streaming(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<String, String> {
    // Get model from request.model_id
}

// After: Use code operation if model_id not provided
pub async fn send_chat_message_streaming(
    request: SendChatRequest,  // model_id is now Optional
    app_handle: AppHandle,
) -> Result<String, String> {
    // If model_id not provided, resolve from 'code' operation
    let model_id = match request.model_id {
        Some(id) => id,
        None => resolve_code_model_id(&app_handle).await?,
    };
    // ... rest of implementation
}
```

**Acceptance criteria**:
- [ ] Command works without explicit model_id
- [ ] Uses 'code' operation configuration by default
- [ ] Falls back to explicit model_id if provided
- [ ] Error messages are clear when configuration is missing

---

### Phase 6: Frontend Integration

**Goal**: Connect chat UI to LLM streaming.

**Files to modify**:
- `src-ui/src/routes/+page.svelte` - Change `handleSendMessage`

**Changes**:
```typescript
// Before
async function handleSendMessage(content: string) {
    await chatStore.sendMessage(content);
}

// After
async function handleSendMessage(content: string) {
    await chatStore.sendMessageWithLlm(content);
}
```

**Acceptance criteria**:
- [ ] Sending a message triggers LLM streaming
- [ ] User message is saved and displayed
- [ ] Assistant message appears with streaming content
- [ ] Loading states work correctly

---

### Phase 7: End-to-End Testing

**Goal**: Verify complete chat flow with real LLM.

**Manual test plan**:
1. Start app with `cargo tauri dev`
2. Configure a provider (e.g., OpenAI) with API key
3. Set default model for 'code' operation
4. Create new conversation
5. Send message: "Hello, can you help me?"
6. Verify:
   - User message appears
   - Assistant response streams in
   - Both messages are persisted

**Acceptance criteria**:
- [ ] Full chat flow works end-to-end
- [ ] Streaming feels responsive
- [ ] Messages persist across restarts
- [ ] Error handling works (e.g., invalid API key)

---

## Implementation Order

1. **Phase 1**: Add configuration query, write tests
2. **Phase 2**: Add provider resolution, write tests
3. **Phase 3**: Verify API connectivity with real call
4. **Phase 4**: Verify streaming works
5. **Phase 5**: Integrate into Tauri command
6. **Phase 6**: Connect frontend
7. **Phase 7**: End-to-end testing

## Files to Create

```
src-tauri/src/bin/
├── test_config.rs              # Phase 1: Test config queries
├── test_provider_resolution.rs # Phase 2: Test provider resolution
├── test_llm_connection.rs      # Phase 3: Test API connection
└── test_llm_streaming.rs       # Phase 4: Test streaming
```

## Files to Modify

```
src-tauri/src/commands/
├── config.rs   # Phase 1: Add get_code_operation_model
└── llm.rs      # Phase 2-5: Add helpers, update command

src-ui/src/routes/
└── +page.svelte # Phase 6: Use sendMessageWithLlm
```

## Progress Tracking

- [x] Phase 1: Configuration Query Tests
  - [x] Added `get_code_operation_model()` function
  - [x] Added `get_code_llm_config()` function
  - [x] Tests pass: `test_get_code_operation_model_success`, `test_get_code_operation_model_not_configured`, `test_get_code_llm_config_full`
- [x] Phase 2: Provider and API Key Resolution
  - [x] Created `test_provider_resolution.rs` binary
  - [x] Config resolution works correctly
- [x] Phase 3: Simple LLM API Test
  - [x] Created `test_llm_connection.rs` binary
  - [x] Mock mode works (real API calls require API key)
- [x] Phase 4: Streaming LLM Test
  - [x] Streaming test in `test_llm_connection.rs`
  - [x] Event emission works correctly
- [x] Phase 5: Tauri Command Integration
  - [x] Added `resolve_model_id()` helper function
  - [x] Updated `SendChatRequest` to make `model_id` optional
  - [x] Both `send_chat_message` and `send_chat_message_streaming` now use code operation default when model_id not provided
- [x] Phase 6: Frontend Integration
  - [x] Updated `SendChatMessageOptions` interface to make `modelId` optional
  - [x] Updated `chatStore.sendMessageWithLlm()` to let backend resolve model
  - [x] Updated `+page.svelte` to use `sendMessageWithLlm()`
- [ ] Phase 7: End-to-End Testing

---

## Usage

To use the chat with LLM streaming:

1. **Start the app**: `cargo tauri dev`

2. **Configure a provider** (first time only):
   - Open the app → Settings → Providers
   - Click "Configure" on a provider (e.g., OpenAI)
   - Enter your API key
   - Click "Fetch Models" to get available models
   - Select a model for the "Code generation" operation

3. **Start chatting**:
   - Create a new conversation
   - Type a message and send
   - The assistant response will stream in real-time

---

## Testing Commands

### Run Unit Tests
```bash
cd src-tauri
cargo test test_get_code
```

### Run Provider Resolution Test
```bash
cargo run --bin test_provider_resolution
```

### Run LLM Connection Test (requires API key)
```bash
# Set your API key first
export OPENAI_API_KEY="sk-..."  # or ANTHROPIC_API_KEY

cargo run --bin test_llm_connection
```

### Manual End-to-End Test
1. Run `cargo tauri dev`
2. Configure a provider with API key
3. Set default model for 'code' operation
4. Send a message in the chat
5. Verify streaming response appears

---

## Summary of Changes

### Rust Backend (`src-tauri/`)

**`src/commands/config.rs`:**
- Added `get_code_operation_model()` - Get model configured for 'code' operation
- Added `get_code_llm_config()` - Get full config (model + provider) for 'code' operation
- Added tests for config resolution

**`src/commands/llm.rs`:**
- Added `resolve_model_id()` helper - Resolves model from 'code' operation if not provided
- Changed `SendChatRequest.model_id` from `String` to `Option<String>`
- Updated both `send_chat_message` and `send_chat_message_streaming` to use code operation default

**Test binaries:**
- `src/bin/test_provider_resolution.rs` - Tests config resolution
- `src/bin/test_llm_connection.rs` - Tests LLM API connection and streaming

### Frontend (`src-ui/`)

**`src/lib/api/llm.ts`:**
- Changed `SendChatMessageOptions.modelId` from required to optional

**`src/lib/stores/chat.ts`:**
- Updated `sendMessageWithLlm()` to let backend resolve model
- Simplified validation logic

**`src/routes/+page.svelte`:**
- Changed `handleSendMessage()` to call `sendMessageWithLm()` instead of `sendMessage()`


## Summary of Changes

### Rust Backend (`src-tauri/`)

**`src/commands/config.rs`:**
- Added `get_code_operation_model()` - Get model configured for 'code' operation
- Added `get_code_llm_config()` - Get full config (model + provider) for 'code' operation
- Added tests for config resolution

**`src/commands/llm.rs`:**
- Added `resolve_model_id()` helper - Resolves model from 'code' operation if not provided
- Changed `SendChatRequest.model_id` from `String` to `Option<String>`
- Updated both `send_chat_message` and `send_chat_message_streaming` to use code operation default

**Test binaries:**
- `src/bin/test_provider_resolution.rs` - Tests config resolution
- `src/bin/test_llm_connection.rs` - Tests LLM API connection and streaming

### Frontend (`src-ui/`)

**`src/lib/api/llm.ts`:**
- Changed `SendChatMessageOptions.modelId` from required to optional

**`src/lib/stores/chat.ts`:**
- Updated `sendMessageWithLlm()` to let backend resolve model
- Simplified validation logic

**`src/routes/+page.svelte`:**
- Changed `handleSendMessage()` to call `sendMessageWithLlm()` instead of `sendMessage()`

