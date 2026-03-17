# Model Selection Per Conversation

## Overview

The Coder application supports selecting different LLM models for each conversation. This allows users to choose the most appropriate model for specific tasks while maintaining that selection across sessions.

## Feature Details

### Data Storage

Model selection is stored as part of the conversation settings in the SQLite database:

- **Table**: `conversations`
- **Column**: `settings` (JSON)
- **Field**: `settings.model`

### Settings Structure

```typescript
interface ConversationSettings {
  /** AI model to use */
  model: string;

  /** Temperature for response generation (0-1) */
  temperature: number;

  /** Maximum tokens for responses */
  maxTokens: number;

  /** Custom system prompt (optional) */
  systemPrompt?: string;
}
```

### Default Values

When a new conversation is created, it uses these default settings:

| Setting | Default Value |
|---------|---------------|
| model | `claude-sonnet-4-20250514` |
| temperature | `0.7` |
| maxTokens | `4096` |
| systemPrompt | `undefined` |

## User Flow

1. **View Current Model**: The ModelSelector component displays the currently selected model in the chat input area
2. **Change Model**: Click on the model selector to open the dropdown menu
3. **Select New Model**: Choose from available models (based on configured providers)
4. **Automatic Persistence**: The selection is automatically saved to the conversation settings

## Implementation

### Backend (Rust)

#### Storage Layer

```rust
// src-tauri/src/storage/chat_store.rs
pub fn update_conversation_settings(
    &self,
    id: &str,
    settings: &ConversationSettings,
) -> Result<Conversation, ChatStoreError>
```

#### Tauri Command

```rust
// src-tauri/src/commands/chat.rs
#[tauri::command]
pub async fn update_conversation_settings(
    id: String,
    settings: ConversationSettings,
    state: tauri::State<'_, ChatState>,
) -> Result<Conversation, String>
```

### Frontend (TypeScript/Svelte)

#### API Layer

```typescript
// src-ui/src/lib/api/chat.ts
export async function updateConversationSettings(
  id: string,
  settings: ConversationSettingsUpdate
): Promise<Conversation>
```

#### Store Layer

```typescript
// src-ui/src/lib/stores/chat.ts
async updateConversationSettings(
  id: string,
  settings: Partial<ConversationSettings>
): Promise<boolean>
```

#### Component Props

ChatArea and ThreadPanel components accept:
- `initialModelId`: The model ID from conversation settings
- `onModelChange`: Callback when user selects a different model

## Model ID Format

Model IDs follow the pattern: `{providerId}-{modelName}`

Examples:
- `openai-gpt-4`
- `anthropic-claude-3-opus`
- `openrouter-llama-2-70b`

## Error Handling

1. **Invalid Conversation ID**: Returns `ChatStoreError::NotFound`
2. **Invalid Model ID**: Frontend validation via `isValidModelId()` function
3. **Database Errors**: Wrapped in `ChatStoreError::Sqlite`

## Testing

### Backend Tests

Located in:
- `src-tauri/src/storage/chat_store.rs` - Storage layer tests
- `src-tauri/src/commands/chat.rs` - Command tests

Run with:
```bash
cd src-tauri
cargo test chat_store
cargo test commands::chat
```

### Frontend Tests

Located in:
- `src-ui/src/lib/api/__tests__/chat.test.ts` - API tests
- `src-ui/src/lib/stores/__tests__/chat-settings.test.ts` - Store tests

Run with:
```bash
cd src-ui
npm test
```

## Future Enhancements

Potential improvements for this feature:

1. **Per-Thread Model Override**: Allow individual threads to override the conversation model
2. **Model History**: Track which model was used for each message
3. **Model Recommendations**: Suggest appropriate models based on conversation context
4. **Settings UI**: Dedicated settings panel for conversation-level model configuration

## Related Documentation

- [Thread Locking](../CLAUDE.md#thread-locking)
- [Configuration System](./config-flow/)
- [Design System](../src-ui/src/lib/styles/design-system.css)
