# Investigation: Chat, Thread, and Message Relationships

**Date:** 2026-03-02
**Status:** Investigation Complete

## Executive Summary

This document investigates the relationship between chats (conversations), threads, and messages in the Coder application, and analyzes two reported bugs:

1. Messages from one chat appearing in another chat
2. New threads not becoming visible until app restart

---

## Architecture Overview

### Data Model

```
┌─────────────────┐
│ Conversation    │  (Chat)
│ - id            │
│ - title         │
│ - threads[]     │
└────────┬────────┘
         │ 1:N
         ▼
┌─────────────────┐
│ Thread          │
│ - id            │
│ - conversationId│
│ - parentId      │  (for nested threads)
│ - messageCount  │
└────────┬────────┘
         │ 1:N
         ▼
┌─────────────────┐
│ Message         │
│ - id            │
│ - conversationId│
│ - threadId      │
│ - role          │
│ - content       │
└─────────────────┘
```

### Key Relationships

| Question | Answer |
|----------|--------|
| Chats have N threads? | **Yes** - Each conversation can contain multiple threads |
| Threads have N messages? | **Yes** - Each thread contains multiple messages |
| LLM context per Chat or Thread? | **Per Thread** - Context is maintained at the thread level |

### Database Schema (SQLite)

```sql
-- Conversations table
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    settings TEXT NOT NULL
);

-- Threads table
CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    parent_id TEXT,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    metadata TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

-- Messages table
CREATE TABLE chat_messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    thread_id TEXT,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'sent',
    metadata TEXT,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE SET NULL
);
```

---

## Frontend State Management

### Store Structure (`src-ui/src/lib/stores/chat.ts`)

```typescript
interface ChatState {
    conversations: Conversation[];       // All conversations
    selectedConversationId: string | null;
    selectedThreadId: string | null;
    messages: Message[];                 // Messages for current thread only
    threads: Thread[];                   // Threads for current conversation
    // ... other state
}
```

### Data Flow

1. **App Load**: `chatStore.initialize()` → `loadConversations()` from backend
2. **Select Conversation**: `selectConversation(id)` → `loadThreads(id)` → auto-select most recent thread → `loadThreadMessages(threadId)`
3. **Send Message**: If no thread exists → `createThreadWithMessage()` → else → `sendMessage()` with threadId

---

## Bug Analysis

### Bug #3: Manually Created Thread Empty, Messages Go to New Thread (NEW)

**Symptoms:**
- User creates a thread manually via UI
- User sends messages, they appear to work
- After restart, manually created thread is empty, messages are in a DIFFERENT thread

**Root Cause Analysis:**

This is a **race condition** in the frontend:

```
Timeline:
1. User clicks "New Thread" → createThread() called
2. createThread calls apiCreateThread() ← ASYNC, takes time
3. User clicks "Send" BEFORE apiCreateThread() returns
4. sendMessage() checks state.threads ← STILL EMPTY
5. existingThreads.length === 0 → creates NEW thread
6. Later, apiCreateThread() returns, adds user's thread
7. Result: TWO threads - user's empty + auto-created with messages
```

**Evidence Location:**
- `src-ui/src/lib/stores/chat.ts:379-410` - `createThread` is async
- `src-ui/src/lib/stores/chat.ts:459-486` - Creates thread if none exist

**Why messages appear to work:**
- The optimistic UI updates show messages in the current view
- But the actual backend message goes to the newly created thread
- On restart, true backend state is revealed

**Severity:** High - Data appears lost/misplaced from user perspective

---

### Bug #1: Messages Appearing in Wrong Chat

**Symptoms:**
- User sees messages from one conversation/thread appearing in another
- Breaks conversation continuity

**Root Cause Analysis:**

The issue is in the frontend state management during streaming operations. Key problem areas:

1. **Streaming State Isolation** (`chat.ts:741-1009`):
   - When `sendMessageWithLlm` is called, it creates an optimistic assistant message
   - Streaming events update messages by `assistantMessageId`
   - Request ID filtering exists but doesn't fully prevent cross-contamination

2. **Race Condition During Conversation Switch**:
   ```typescript
   // In selectConversation - messages are cleared
   update((state) => ({
       ...state,
       selectedConversationId: id,
       selectedThreadId: null,
       messages: [],  // Cleared here
       // ...
   }));
   ```
   If streaming is in progress and user switches conversations, stale event listeners may still fire.

3. **Message ID Collision Potential**:
   ```typescript
   // Message ID generation uses timestamp
   const assistantMessageId = `streaming-${crypto.randomUUID()}`;
   ```
   While UUID is unique, the optimistic message replacement logic could fail if messages array is replaced during streaming.

**Evidence Locations:**
- `src-ui/src/lib/stores/chat.ts:885-905` - Chunk event handler updates messages array
- `src-ui/src/lib/stores/chat.ts:854-863` - Request ID filtering (incomplete)

**Severity:** High - Breaks core functionality

---

### Bug #2: New Threads Not Visible Until App Restart

**Symptoms:**
- When starting a chat in a new conversation, a thread is created
- The thread becomes visible with messages only after reopening the app
- Thread count in sidebar doesn't update

**Root Cause Analysis:**

The issue is in the state update when creating a thread with a message:

**Location:** `src-ui/src/lib/stores/chat.ts:463-477`

```typescript
// When creating thread with message atomically
const { thread, message } = await apiCreateThreadWithMessage({...});

update((s) => ({
    ...s,
    threads: [...s.threads, thread],        // ✓ Updates store.threads
    selectedThreadId: thread.id,            // ✓ Selects the thread
    messages: [...s.messages, message]      // ✓ Adds message
    // ❌ MISSING: Update conversation.threads array!
}));
```

The problem is that `ConversationItem.svelte` reads thread count from `conversation.threads`:

```svelte
<!-- ConversationItem.svelte:132-135 -->
{#if conversation.threads && conversation.threads.length > 0}
    <span class={styles.threadCount}>{formatThreadCount(conversation.threads.length)}</span>
{/if}
```

The `conversations` array in the store is NOT updated with the new thread, causing:
1. Sidebar thread count to show stale data
2. Thread panel to potentially not reflect the new thread

**Compare with `selectConversation` which DOES update correctly:**

```typescript
// In selectConversation - this IS correct
update((state) => ({
    ...state,
    threads,
    conversations: state.conversations.map((c) =>
        c.id === id ? { ...c, threads } : c  // ✓ Updates conversation.threads
    )
}));
```

**Severity:** Medium - UI shows incorrect state, data is persisted correctly

---

## Task List for Fixes

### Priority 1: Fix Race Condition - Thread Creation (Bug #3)

| Task | Description | Files |
|------|-------------|-------|
| T1 | Add pending thread state to track thread creation in progress | `src-ui/src/lib/stores/chat.ts` |
| T2 | Wait for pending thread creation in `sendMessage` before creating new thread | `src-ui/src/lib/stores/chat.ts:459-486` |
| T3 | Add loading/disabled state to "Send" button while thread is being created | `src-ui/src/lib/components/chat/input/ChatInput.svelte` |

### Priority 2: Fix Thread Visibility Bug (Bug #2)

| Task | Description | Files |
|------|-------------|-------|
| T4 | Update `sendMessage` function to also update `conversations` array with new thread | `src-ui/src/lib/stores/chat.ts:463-477` |
| T5 | Update `createThread` function to also update `conversations` array | `src-ui/src/lib/stores/chat.ts:379-410` |

### Priority 3: Fix Message Cross-Contamination Bug (Bug #1)

| Task | Description | Files |
|------|-------------|-------|
| T6 | Add conversation/thread ID validation in streaming event handlers | `src-ui/src/lib/stores/chat.ts:852-975` |
| T7 | Cancel active stream when switching conversations | `src-ui/src/lib/stores/chat.ts:298-344` |
| T8 | Fix stale state in `sendMessageWithLlm` - get fresh state for assistant message | `src-ui/src/lib/stores/chat.ts:814-815` |

### Priority 4: Testing & Documentation

| Task | Description | Files |
|------|-------------|-------|
| T9 | Add unit tests for thread creation race condition | `src-ui/src/lib/stores/__tests__/` |
| T10 | Add unit tests for message isolation between threads | `src-ui/src/lib/stores/__tests__/` |
| T11 | Add integration tests for cross-conversation switching during streaming | New test file |

---

## Recommended Solution Approach

### For Bug #3 (Race Condition - Critical Fix)

Add pending thread tracking to prevent race condition:

```typescript
// In createChatStore, add state tracking
let pendingThreadCreation: Promise<Thread> | null = null;

// In createThread
async createThread(title: string = 'New Thread'): Promise<Thread | null> {
    // ... validation ...

    try {
        // Track the pending creation
        const creationPromise = apiCreateThread(state.selectedConversationId, title);
        pendingThreadCreation = creationPromise;

        const thread = await creationPromise;

        update((s) => ({
            ...s,
            threads: [...s.threads, thread],
            conversations: s.conversations.map((c) =>
                c.id === state.selectedConversationId
                    ? { ...c, threads: [...(c.threads || []), thread] }
                    : c
            )
        }));

        await this.selectThread(thread.id);

        pendingThreadCreation = null;
        return thread;
    } catch (error) {
        pendingThreadCreation = null;
        // ... error handling ...
    }
}

// In sendMessage - wait for pending thread creation
async sendMessage(content: string, role = 'user') {
    // Wait for any pending thread creation to complete
    if (pendingThreadCreation) {
        await pendingThreadCreation;
    }

    const state = get({ subscribe });
    // ... rest of function ...
}
```

### For Bug #2 (Quick Fix)

In `sendMessage` function, when creating a thread with message:

```typescript
update((s) => ({
    ...s,
    threads: [...s.threads, thread],
    selectedThreadId: thread.id,
    messages: [...s.messages, message],
    // ADD THIS: Update the conversation's threads array
    conversations: s.conversations.map((c) =>
        c.id === state.selectedConversationId
            ? { ...c, threads: [...(c.threads || []), thread] }
            : c
    )
}));
```

### For Bug #1 (Comprehensive Fix)

1. **Get fresh state for assistant message threadId:**
   ```typescript
   // In sendMessageWithLlm, after sendMessage returns
   const freshState = get({ subscribe });
   const optimisticAssistant: Message = {
       // ...
       threadId: freshState.selectedThreadId ?? '',  // Use FRESH state
       // ...
   };
   ```

2. **Cancel stream on conversation switch:**
   ```typescript
   async selectConversation(id: string | null) {
       // Cancel any active stream before switching
       if (activeStream) {
           await activeStream.cancel();
           activeStream.unsubscribe();
           activeStream = null;
       }
       // ... rest of function
   }
   ```

---

## Files Affected

| File | Changes Needed |
|------|----------------|
| `src-ui/src/lib/stores/chat.ts` | Primary fixes for both bugs |
| `src-ui/src/lib/components/chat/sidebar/ConversationItem.svelte` | May need to read from store.threads instead of conversation.threads |
| `src-tauri/src/storage/chat_store.rs` | No changes needed - backend is correct |
| `src-tauri/src/commands/chat.rs` | No changes needed - backend is correct |

---

## Testing Recommendations

1. **Unit Tests:**
   - Test thread creation updates both `threads` and `conversations[x].threads`
   - Test message isolation between different threads
   - Test stream cancellation on conversation switch

2. **Integration Tests:**
   - Create conversation, send message (creates thread), verify sidebar shows "1 thread"
   - Start streaming, switch conversation, verify no message cross-contamination
   - Create multiple threads, verify all appear in UI immediately

3. **Manual Testing:**
   - Test rapid conversation switching during LLM response
   - Test creating new conversation and sending first message
   - Verify thread counts in sidebar update in real-time

---

## Conclusion

Both bugs are **frontend state management issues**, not backend problems:

1. **Bug #1** is caused by inadequate isolation of streaming state during conversation switches
2. **Bug #2** is caused by incomplete state updates when creating threads

The backend correctly stores and retrieves data, but the frontend's state management needs improvement to properly isolate contexts and maintain consistent state across all derived data structures.
