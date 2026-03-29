# Thread Locking

**Status**: ACTIVE
**Last reviewed**: 2026-03-29
**Purpose**: Specification for thread resolved/locked state behavior across backend and frontend

---

## Overview

Threads have two states: **unsolved** (open) and **resolved** (locked). When a thread is resolved, it enters a locked state that prevents further modifications.

## Thread States

1. **Unsolved (Open)**: The default state where threads can accept new messages and be actively modified.
2. **Resolved (Locked)**: A thread marked as complete that cannot accept new messages.

## Backend Behavior

The Rust backend enforces thread locking at the API level:

- **`send_message` command**: Validates that the target thread is not resolved before accepting any new message. Returns a `ThreadResolved` error if attempting to send to a resolved thread.
- **`create_thread_with_message` command**: Creates new threads (always in unsolved state).
- **`resolve_thread` command**: Toggles a thread's resolved status between unsolved and resolved.

The error message format when attempting to send to a resolved thread:
```
Thread '{thread_title}' is resolved and cannot accept new messages. Reopen the thread to continue the conversation.
```

## Frontend Behavior

The Svelte frontend reflects thread resolved state visually and functionally:

1. **ThreadPanel**: Displays a prominent warning banner when a thread is resolved, styled with warning colors from the design system.
2. **ChatInput**: Disables the text input when the thread is resolved, with visual feedback (warning background, reduced opacity).
3. **ChatArea**: Passes through the resolved state to the input component.

## User Flow

1. User can mark any thread as "resolved" via the resolve/reopen button in the ThreadItem.
2. Once resolved:
   - The thread appears in the "Resolved" section of the sidebar
   - A warning banner appears at the bottom of the thread view
   - The chat input is disabled with warning styling
3. User can "reopen" a resolved thread to continue the conversation:
   - Thread moves back to the "Open" section
   - Banner and input restrictions are removed
   - New messages can be sent again

## Design Tokens Used

For the resolved state UI, use these design system tokens:
- Background: `var(--color-warning-light)`
- Border: `var(--color-warning)`
- Text: `var(--color-warning)`
- Disabled opacity: `var(--opacity-disabled)`
