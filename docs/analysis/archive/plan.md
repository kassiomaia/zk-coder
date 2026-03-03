# Archive Feature Implementation Plan

## Overview
Implement archive functionality for conversations and threads with a consistent UI pattern following existing delete/rename behavior.

## Implementation Steps

### Phase 1: Backend API Commands (Rust)

1. **Add Archive Commands to `src/src/commands.rs`**
   - Create `archive_conversation` command
   - Create `archive_thread` command
   - Follow existing patterns for `delete_conversation` and `delete_thread`

2. **Update Backend Models**
   - Add `archived_at` field to Conversation and Thread models
   - Create database migration for new field

### Phase 2: Frontend Components

#### 2.1 Update ConversationActions.svelte
```typescript
interface Props {
    ondelete?: () => void;
    onrename?: () => void;
    onarchive?: () => void;  // Add archive handler
}

let { ondelete, onrename, onarchive }: Props = $props();
```

Add archive button to the action menu between rename and delete:
```svelte
{#if onarchive}
    <button
        class={styles.action}
        onclick={(e) => {
            e.stopPropagation();
            onarchive();
        }}
        aria-label="Archive conversation"
        title="Archive"
        type="button"
    >
        <!-- Archive icon -->
        <svg>
            <path d="M21 9v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V9Z"/>
            <polyline points="9 11 12 14 15 11"/>
            <line x1="3" y1="5" x2="21" y2="5"/>
        </svg>
    </button>
{/if}
```

#### 2.2 Update ConversationItem.svelte
```typescript
interface Props {
    conversation: Conversation;
    selected?: boolean;
    onclick: () => void;
    ondoubleclick?: () => void;
    ondelete?: () => void;
    onrename?: (newTitle: string) => void;
    onarchive?: () => void;  // Add archive handler
}

let { conversation, selected = false, onclick, ondoubleclick, ondelete, onrename, onarchive }: Props = $props();
```

Pass archive handler to ConversationActions:
```svelte
<ConversationActions
    ondelete={ondelete}
    onrename={onrename ? startEdit : undefined}
    onarchive={onarchive}
/>
```

#### 2.3 Update ThreadItem.svelte
```typescript
interface Props {
    thread: Thread;
    selected?: boolean;
    onclick: () => void;
    onresolve?: () => void;
    ondelete?: () => void;
    onarchive?: () => void;  // Add archive handler
}

let { thread, selected = false, onclick, onresolve, ondelete, onarchive }: Props = $props();
```

Add archive button to actions:
```svelte
{#if showActions && (onresolve || ondelete || onarchive)}
    <div class={styles.actions}>
        <!-- Resolve/Reopen button -->

        <!-- Archive button -->
        {#if onarchive}
            <button
                class={styles.action}
                onclick={(e) => {
                    e.stopPropagation();
                    onarchive();
                }}
                aria-label="Archive thread"
                title="Archive"
                type="button"
            >
                <!-- Same archive icon -->
            </button>
        {/if}

        <!-- Delete button -->
    </div>
{/if}
```

### Phase 3: Parent Components Integration

#### 3.1 Sidebar Integration
Update the parent component that renders ConversationItem to pass archive handler:
```typescript
function handleArchiveConversation(conversationId: string) {
    // Call backend API to archive
    // Update UI state
}
```

#### 3.2 Thread Panel Integration
Update ThreadPanel or parent component to pass archive handler:
```typescript
function handleArchiveThread(threadId: string) {
    // Call backend API to archive
    // Update UI state
}
```

### Phase 4: Styling

#### 4.1 Update ConversationActions.module.css
Add archive button styling:
```css
.action:hover {
    background: var(--color-surface-hover);
    color: var(--color-text-primary);
}

.action.delete:hover {
    background: var(--color-error-light);
    color: var(--color-error);
}

.action.archive:hover {
    background: var(--color-warning-light);
    color: var(--color-warning);
}
```

#### 4.2 Update ThreadItem.module.css
Add same archive styles.

## Design Tokens
- Archive color: `var(--color-warning)` and `var(--color-warning-light)`
- Icon: Simple box with down arrow pattern
- Position: Between rename and delete in ConversationActions
- Position: After resolve and before delete in ThreadItem

## Testing Strategy
1. Unit tests for archive API commands
2. Integration tests for archive functionality
3. UI tests for visual feedback
4. Accessibility tests for keyboard navigation

## Questions for User
1. Should archived items be immediately removed from the UI or moved to an "Archived" section?
2. Should there be an undo feature for accidental archiving?
3. Should archived items be searchable/filterable?
4. Should there be a confirmation dialog for archive?

## Timeline
- Backend implementation: 1-2 days
- Frontend components: 1 day
- Styling: 0.5 days
- Testing: 1 day
- Total: 3.5-4.5 days