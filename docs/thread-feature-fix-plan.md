# Thread Feature Fix Plan

**Created**: 2026-03-01
**Last Updated**: 2026-03-01
**Status**: Phase 1 COMPLETE
**Related**: `docs/progress.md`, `docs/backend-progress.md`

---

## Implementation Progress

### Phase 1: Fix Thread Display ✅ COMPLETE
**Priority**: High
**Completed**: 2026-03-01

| Task | Status | Notes |
|------|--------|-------|
| Add `threads` prop to ChatLayout | ✅ Done | Separate from conversations |
| Update +page.svelte to pass threads | ✅ Done | Subscribe to currentThreads |
| ThreadView receives correct threads | ✅ Done | Uses threads prop directly |
| Make `threads` optional in Conversation type | ✅ Done | Added deprecation note |
| Add thread panel toggle button | ✅ Done | Badge shows thread count |
| Thread panel visible when threads exist | ✅ Done | Collapsible on desktop |

### Phase 2: Thread Panel UX Improvements ✅ COMPLETE
**Priority**: Medium
**Completed**: 2026-03-01

| Task | Status | Notes |
|------|--------|-------|
| Persist panel state in localStorage | ✅ Done | Uses `coder:threadPanelOpen` key |
| Desktop toggle button | ✅ Done | Floating button when panel collapsed |
| Collapse button in ThreadView | ✅ Done | Chevron button in panel header |
| Show thread count badge on conversation | Deferred | Requires backend `thread_count` field |

### Phase 3: Thread-Message Integration
**Priority**: Low
**Status**: Not Started

| Task | Status | Notes |
|------|--------|-------|
| Show thread indicator on messages | Pending | Badge showing which thread a message belongs to |
| Threaded message visualization | Pending | Visual distinction in chat area |
| Click to open thread from message | Pending | Navigation enhancement |

### Phase 4: Backend Enhancements (Optional)
**Priority**: Low
**Status**: Not Started

| Task | Status | Notes |
|------|--------|-------|
| Add `thread_count` to Conversation response | Pending | Avoid extra API call for counts |
| Add batch thread loading endpoint | Pending | Load threads for multiple conversations |
| Add thread summary generation | Pending | AI-generated thread summaries |

---

## Problem Summary

After frontend-backend integration, the thread feature is not working correctly:
1. Threads are not displayed in the ThreadView component
2. The thread panel visibility/open/close state is disconnected
3. There's a data flow mismatch between how threads are stored and accessed

---

## Root Cause Analysis

### Issue 1: Thread Data Not Reaching ThreadView

**Location**: `ChatLayout.svelte` (line 206)

```svelte
<ThreadView
    threads={selectedConversation?.threads || []}  <!-- BUG: Always empty! -->
    ...
/>
```

**Problem**:
- `ChatLayout` expects threads to be inside `conversation.threads`
- But threads are stored separately in `chatStore.threads` (not embedded in conversations)
- Backend `Conversation` struct does NOT include a `threads` field
- This means ThreadView always receives an empty array

**Data Flow Mismatch**:
```
Backend: Conversation (no threads) → API → Frontend: Conversation
                                           ↓
Backend: Thread[] → API → chatStore.threads (separate array)
                                           ↓
                    ChatLayout uses: conversation.threads (empty!)
```

### Issue 2: Type Inconsistency

**Frontend Type** (`types/thread.ts:64-82`):
```typescript
export interface Conversation {
    id: string;
    title: string;
    // ...
    threads: Thread[];  // <-- Frontend expects this
    settings: ConversationSettings;
}
```

**Backend Type** (`nlp/chat.rs:222-234`):
```rust
pub struct Conversation {
    pub id: String,
    pub title: String,
    // ...
    // NO threads field!
    pub settings: ConversationSettings,
}
```

### Issue 3: Missing Thread Panel Toggle

The `ChatLayout` has internal state `threadPanelOpen` but:
- No way to close/expand the thread panel from outside
- No persistent state for panel visibility preference
- The toggle button only appears on mobile

### Issue 4: Missing Thread Indicator in Main View

When viewing a conversation (not a thread), there's no indication that threads exist.
- No badge showing thread count
- No way to see which messages belong to threads
- Thread panel is only visible when a thread is selected

---

## Proposed Solution

### Option A: Keep Separate Thread Storage (Recommended)

This aligns with backend architecture and avoids loading all threads for all conversations.

#### Changes Required:

**1. Update ChatLayout.svelte**
- Add `threads` as a separate prop
- Pass `currentThreads` from the parent component

```svelte
<!-- ChatLayout.svelte -->
<script lang="ts">
    interface Props {
        conversations: Conversation[];
        threads: Thread[];  // NEW: Separate threads prop
        // ... other props
    }
</script>

<ThreadView
    threads={threads}  <!-- Use threads prop instead of conversation.threads -->
    ...
/>
```

**2. Update +page.svelte**
- Subscribe to `currentThreads` store
- Pass threads to ChatLayout

```svelte
<script lang="ts">
    import { chatStore, currentThreads } from '$lib/stores';
    let threads = $state<Thread[]>([]);

    $effect(() => {
        const unsub = currentThreads.subscribe(t => threads = t);
        return unsub;
    });
</script>

<ChatLayout
    {conversations}
    {threads}  <!-- NEW: Pass threads -->
    ...
/>
```

**3. Remove `threads` from Frontend Conversation Type**
- Update `types/thread.ts` to not expect `threads` in Conversation
- Or make it optional with a comment explaining it's populated client-side

**4. Add Thread Visibility Toggle**
- Add a threads toggle button in the main header
- Allow users to show/hide thread panel independently of thread selection
- Persist preference in localStorage

### Option B: Embed Threads in Conversations

Load threads when loading conversations and embed them.

**Pros**: Simpler data flow, matches frontend expectation
**Cons**: Less efficient (loads all threads for all conversations), more complex API

---

## Implementation Plan

### Phase 1: Fix Thread Display (Critical)
**Priority**: High
**Estimated Effort**: 1-2 hours

| Task | Status | Notes |
|------|--------|-------|
| Add `threads` prop to ChatLayout | Pending | Separate from conversations |
| Update +page.svelte to pass threads | Pending | Subscribe to currentThreads |
| ThreadView receives correct threads | Pending | Verify with test data |
| Remove `threads` from Conversation type (or make optional) | Pending | Align with backend |

### Phase 2: Thread Panel UX Improvements
**Priority**: Medium
**Estimated Effort**: 2-3 hours

| Task | Status | Notes |
|------|--------|-------|
| Add thread panel toggle button | Pending | Always visible, not just on thread selection |
| Persist panel state in localStorage | Pending | Remember user preference |
| Show thread count badge on conversation | Pending | Visual indicator |
| Add "View Threads" action | Pending | Quick access to thread list |

### Phase 3: Thread-Message Integration
**Priority**: Low
**Estimated Effort**: 2-3 hours

| Task | Status | Notes |
|------|--------|-------|
| Show thread indicator on messages | Pending | Badge showing which thread a message belongs to |
| Threaded message visualization | Pending | Visual distinction in chat area |
| Click to open thread from message | Pending | Navigation enhancement |

### Phase 4: Backend Enhancements (Optional)
**Priority**: Low
**Estimated Effort**: 3-4 hours

| Task | Status | Notes |
|------|--------|-------|
| Add `thread_count` to Conversation response | Pending | Avoid extra API call for counts |
| Add batch thread loading endpoint | Pending | Load threads for multiple conversations |
| Add thread summary generation | Pending | AI-generated thread summaries |

---

## Files to Modify

### Phase 1 (Critical Fix)
1. `src-ui/src/lib/components/layout/ChatLayout.svelte` - Add threads prop
2. `src-ui/src/routes/+page.svelte` - Pass threads from store
3. `src-ui/src/lib/types/thread.ts` - Make threads optional in Conversation

### Phase 2 (UX Improvements)
4. `src-ui/src/lib/components/layout/ChatLayout.module.css` - Panel toggle styles
5. `src-ui/src/lib/components/chat/sidebar/ConversationItem.svelte` - Thread count badge

### Phase 3 (Integration)
6. `src-ui/src/lib/components/chat/ChatMessage.svelte` - Thread indicator
7. `src-ui/src/lib/components/chat/ChatArea.svelte` - Threaded message display

---

## Testing Checklist

- [ ] Threads display correctly when conversation is selected
- [ ] ThreadView shows open/resolved groups correctly
- [ ] Creating a thread updates the list immediately
- [ ] Selecting a thread loads its messages
- [ ] Resolving a thread moves it to resolved group
- [ ] Deleting a thread removes it from the list
- [ ] Thread panel toggle persists across sessions
- [ ] Thread count badge shows correct number
- [ ] Mobile responsive behavior works

---

## Questions to Clarify

1. **Thread panel behavior**: Should the thread panel be:
   - Always visible when a conversation is selected?
   - Only visible when user explicitly opens it?
   - Auto-open when threads exist?

2. **Thread creation**: Should we:
   - Auto-create a thread when starting a new conversation?
   - Require manual thread creation?
   - Create threads based on topic detection?

3. **Message threading**: Should messages in the main view:
   - Show which thread they belong to?
   - Be clickable to open the thread?
   - Be visually grouped by thread?

---

## Related Documents

- [Chat Interface Implementation Plan](./chat-interface-implementation-plan.md)
- [Backend Progress](./backend-progress.md)
- [Frontend Progress](./progress.md)
