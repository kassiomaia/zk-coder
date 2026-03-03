# Message Editing - Product Requirements Document

## Problem Statement

Users cannot correct mistakes in their messages after sending them. This leads to:
- Wasted tokens on responses to typos or unclear prompts
- Inability to refine prompts for better AI responses
- Cluttered conversations with correction follow-ups
- Poor user experience compared to modern chat applications

## Success Metrics

- **Edit Usage**: > 25% of conversations contain at least one edited message by month 2
- **Regeneration Rate**: 40% of edits are followed by AI response regeneration
- **User Satisfaction**: < 5% of users report inability to edit as a pain point in feedback
- **Time to Edit**: Average time to edit and save a message < 10 seconds

## User Stories

1. **As a user**, I want to edit my message to fix a typo so that the AI understands my intent correctly.

2. **As a user**, I want to rephrase my question after seeing the AI's response so that I can get a better answer without starting a new thread.

3. **As a user**, I want to see an "edited" indicator on modified messages so that I know the conversation context has changed.

4. **As a user**, I want to regenerate the AI response after editing my message so that the response reflects my updated input.

5. **As a user**, I want to cancel an edit and restore my original message so that I can experiment without commitment.

## Functional Requirements

### Core Editing

- [ ] **Double-click to edit** user messages (not assistant messages)
- [ ] **Inline editing** within the message bubble
- [ ] **Save button** to commit changes (Enter key also saves)
- [ ] **Cancel button** to discard changes (Escape key also cancels)
- [ ] **"Edited" indicator** displayed on modified messages
- [ ] **Edit timestamp** shown on hover over edited indicator

### Response Regeneration

- [ ] **Prompt to regenerate** after editing a message that has responses
- [ ] **Regenerate button** on edited messages with AI responses
- [ ] **Clear downstream** option to delete all messages after the edited one
- [ ] **Keep downstream** option to preserve responses (with warning about context mismatch)

### Edit History (Phase 2)

- [ ] **View edit history** by clicking on "edited" indicator
- [ ] **Previous versions** displayed in a modal or dropdown
- [ ] **Restore previous version** option

## Non-Functional Requirements

### Performance

- **Edit save latency**: < 100ms for saving edits
- **UI responsiveness**: Edit mode appears in < 50ms on double-click
- **Database update**: Single UPDATE query, no full message reload required

### User Experience

- **Visual feedback**: Clear distinction between view and edit modes
- **Confirmation for destructive actions**: Warn before clearing downstream messages
- **Undo support**: Allow undo (Cmd/Ctrl+Z) within 5 seconds of saving
- **Touch support**: Long-press to edit on touch devices

### Technical

- **Database**: Add `edited_at` timestamp column to messages table
- **Immutability**: Original `created_at` preserved, only `edited_at` updated
- **API**: New `update_message` Tauri command

## Dependencies

### Internal

- `src-tauri/src/storage/chat_store.rs` - Add update method
- `src-tauri/src/commands/chat.rs` - Add update_message command
- `src-ui/src/lib/components/chat/ChatMessage.svelte` - Edit mode UI
- `src-ui/src/lib/stores/chat.ts` - Edit state management

### External

- None

## Technical Approach

### Backend Changes

```rust
// Migration: Add edited_at column
ALTER TABLE messages ADD COLUMN edited_at INTEGER DEFAULT NULL;

// New command
#[tauri::command]
pub async fn update_message(
    message_id: String,
    content: String,
) -> Result<Message, String>;

// Updated Message struct
pub struct Message {
    pub id: String,
    pub content: String,
    pub role: MessageRole,
    pub created_at: i64,
    pub edited_at: Option<i64>,  // New field
    // ... other fields
}
```

### Frontend Changes

```
Component modifications:
- ChatMessage.svelte:
  - Add editMode state
  - Double-click handler
  - Inline textarea for editing
  - Save/Cancel buttons
  - "Edited" badge with timestamp

Store additions:
- editingMessageId: string | null
- editContent: string
```

### Message Component States

```
┌─────────────────────────────────┐
│ [User Message - View Mode]      │
│ "What is the capital of Franc?" │  ← Double-click to edit
│                     edited 2m   │  ← If edited, show indicator
└─────────────────────────────────┘

┌─────────────────────────────────┐
│ [User Message - Edit Mode]      │
│ ┌─────────────────────────────┐ │
│ │ What is the capital of France│ │  ← Textarea with focus
│ └─────────────────────────────┘ │
│  [Cancel]  [Save & Regenerate]  │  ← Action buttons
└─────────────────────────────────┘
```

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Context confusion after edit | Medium | Medium | Clear visual indicator; warn about downstream |
| Accidental edits | Low | Low | Double-click (not single) to enter edit mode |
| Database migration issues | Low | Medium | Test migration with existing databases |
| Performance on long messages | Low | Low | Optimize for messages < 10,000 characters |

## Out of Scope

- Editing AI/assistant messages (security/trust concern)
- Collaborative editing (not a shared workspace)
- Rich text editing (Markdown only)
- Edit permissions/roles

## Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Database migration | 0.5 days | edited_at column, backward compatible |
| Backend command | 1 day | update_message Tauri command |
| Frontend edit mode | 2 days | Inline editing UI, save/cancel |
| Regeneration flow | 1 day | Prompt to regenerate, downstream handling |
| Polish & testing | 1 day | Edge cases, animations, accessibility |
| **Total** | **5.5 days** | **Feature complete** |

## Open Questions

1. Should we auto-save drafts of edits in progress?
2. What is the maximum time window for editing a message?
3. Should regeneration be automatic or require user action?
4. How should we handle edits to messages with file attachments?

## Acceptance Criteria

- [ ] User can double-click their message to enter edit mode
- [ ] Edit mode shows textarea with current content
- [ ] Save button commits changes and shows "edited" indicator
- [ ] Cancel button restores original content
- [ ] Escape key cancels edit
- [ ] Enter key saves edit
- [ ] Edited messages show timestamp on hover
- [ ] User is prompted to regenerate after editing
- [ ] Regeneration clears downstream messages (with warning)
- [ ] Works offline (local SQLite)
- [ ] Migration works on existing databases

---

**Document Status**: Draft
**Author**: Product Team
**Created**: 2026-03-02
**Last Updated**: 2026-03-02
