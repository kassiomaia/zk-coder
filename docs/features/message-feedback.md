# Message Feedback (Like/Dislike) Feature

## Overview

The message feedback feature allows users to provide feedback on LLM responses (like/dislike). This feedback is persisted per message in the database and displayed visually in the UI.

## Features

- **Toggleable feedback**: Clicking the same button removes the feedback
- **Mutually exclusive**: Only like OR dislike can be active at a time
- **Persistent**: Feedback is saved to the database and survives page reloads
- **Visual feedback**: Active states show filled icons with appropriate colors

## Architecture

### Backend (Rust)

#### Types (`src-tauri/src/nlp/chat.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageFeedback {
    Like,
    Dislike,
}

pub struct MessageMetadata {
    // ... other fields
    pub feedback: Option<MessageFeedback>,
}
```

#### Storage (`src-tauri/src/storage/chat_store.rs`)

- `update_message_feedback(id, feedback)`: Updates feedback for a message
- Feedback is stored in the `metadata` JSON column of `chat_messages` table

#### Command (`src-tauri/src/commands/chat.rs`)

```rust
#[tauri::command]
pub async fn update_message_feedback(
    id: String,
    feedback: Option<String>,  // "like", "dislike", or null
    state: tauri::State<'_, ChatState>,
) -> Result<(), String>
```

### Frontend (TypeScript/Svelte)

#### Types (`src-ui/src/lib/types/message.ts`)

```typescript
export type MessageFeedback = 'like' | 'dislike';

export interface MessageMetadata {
    // ... other fields
    feedback?: MessageFeedback;
}
```

#### API (`src-ui/src/lib/api/chat.ts`)

```typescript
export async function updateMessageFeedback(
    id: string,
    feedback: 'like' | 'dislike' | null
): Promise<void>
```

#### Store (`src-ui/src/lib/stores/chat.ts`)

```typescript
async updateMessageFeedback(
    id: string,
    feedback: 'like' | 'dislike' | null
): Promise<boolean>
```

#### Components

1. **MessageActions.svelte**: Displays thumbs up/down buttons with controlled state
2. **ChatArea.svelte**: Passes feedback from message metadata to MessageActions
3. **ThreadPanel.svelte**: Propagates feedback handler to ChatArea
4. **ChatLayout.svelte**: Propagates feedback handler to child components
5. **+page.svelte**: Connects feedback handler to chat store

## User Flow

1. User sees an assistant message with thumbs up/down buttons
2. Clicking thumbs up:
   - Sets feedback to "like"
   - Icon fills with success color
   - If "dislike" was active, it becomes inactive
3. Clicking thumbs up again:
   - Clears feedback (null)
   - Icon returns to unfilled state
4. Same pattern applies for thumbs down with warning color

## Database Storage

Feedback is stored in the `metadata` JSON column of the `chat_messages` table:

```json
{
  "feedback": "like"
}
```

No migration was required since `metadata` is a JSON column.

## LLM Context Injection (Implemented)

Feedback is injected into the LLM context using an **ID-based approach** for token efficiency.

### Context Structure

When sending messages to the LLM, the context is structured as follows:

```
[SYSTEM - Core Instructions]

You are a helpful AI assistant. Each message in this conversation has a unique
identifier prefixed with [ID:xxxxxxxx]. When the user provides feedback (like/dislike)
on a message, it will be indicated in a feedback summary section. Use this
feedback to improve your responses.

[SYSTEM - Feedback Summary]

Feedback on previous messages:
- [ID:7f3a2c1d] LIKED by user
- [ID:4b1c9e8f] DISLIKED by user

[CONVERSATION]

[USER - ID:1e8f4a2b]:
How do I implement a binary search tree in Rust?

[ASSISTANT - ID:7f3a2c1d]:
A binary search tree is a data structure...

[USER - ID:9d5e1f3a]:
Can you show me a more practical example?
```

### Token Efficiency

| Approach | Tokens per Feedback |
|----------|---------------------|
| Content preview | ~40-60 tokens |
| **ID-based** | ~8-12 tokens |

For 5 feedback items: **~200-300 tokens saved per request**

### Implementation Details

- Messages are prefixed with `[ROLE - ID:{first_8_chars_of_uuid}]`
- Feedback summary references only the message ID
- Only the most recent 5 feedback items are included
- Feedback is loaded via `load_thread_feedback(thread_id, 5)`

## Future Enhancements

1. **Feedback Analytics**: Aggregate feedback statistics for model performance monitoring
2. **Feedback Export**: Export feedback data for fine-tuning purposes

---

## LLM Context Injection (Planned - Phase 3)

Feedback will be injected into the LLM context using an **ID-based approach** for token efficiency.

### Context Structure

When sending messages to the LLM, the context will be structured as follows:

```
[SYSTEM - Core Instructions]

You are a helpful AI assistant. Each message in this conversation has a unique
identifier prefixed with [ID:xxxx]. When the user provides feedback (like/dislike)
on a message, it will be indicated in the feedback summary section. Use this
feedback to improve your responses.

[SYSTEM - Feedback Summary]

Feedback on previous messages:
- [ID:7f3a2c1d] DISLIKED by user
- [ID:4b1c9e8f] LIKED by user

[CONVERSATION]

[USER - ID:1e8f4a2b]:
How do I implement a binary search tree in Rust?

[ASSISTANT - ID:7f3a2c1d]:
A binary search tree is a data structure...

[USER - ID:9d5e1f3a]:
Can you show me a more practical example?
```

### Token Efficiency

| Approach | Tokens per Feedback |
|----------|---------------------|
| Content preview | ~40-60 tokens |
| **ID-based** | ~8-12 tokens |

For 5 feedback items: **~200-300 tokens saved per request**

### Implementation Details

- Messages are prefixed with `[ROLE - ID:{first_8_chars_of_uuid}]`
- Feedback summary references only the message ID
- Only the most recent 5 feedback items are included
- Feedback is loaded via `load_thread_feedback(thread_id, 5)`
