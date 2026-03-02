# Auto-scroll Feature

## Overview

Auto-scroll functionality ensures message content flows smoothly during chat conversations in the Coder AI Code Agent application.

## Behavior

### New Threads
When a user opens a thread for the first time, the chat starts at the bottom (instant, no animation).

### User Scroll Detection
- **Scroll Position Memory**: When switching between threads, the scroll position is remembered (session-only, not persisted to storage)
- **User Scrolls Up**: Auto-scroll is immediately disabled, allowing user to read previous messages
- **Scroll to Bottom Button**: Appears when user scrolls up, with pulsing animation during streaming

### Auto-Scroll Re-enable
Auto-scroll only re-enables when user **actively** goes to the bottom:
1. User scrolls to bottom manually
2. User clicks "Scroll to Bottom" button
3. User sends a new message

**No timers** - user must actively return to bottom.

### Pulsing Button
When the assistant is streaming and user is scrolled up:
- The "Scroll to Bottom" button pulses smoothly
- Indicates active response being generated
- Stops pulsing when streaming completes

### OS Notifications
When the assistant finishes a message while the user is scrolled up (not near the bottom), a system notification is sent.

## Technical Details

- **Smooth scrolling**: 300ms animation with easeOutQuad easing
- **Scroll position memory**: Session-only Map-based storage by thread ID
- **Near bottom threshold**: 100px from the bottom
- **Initial scroll**: Instant (no animation) to prevent visual flash

## API Reference

### Scroll Manager (`scrollManager.ts`)
- `isNearBottom(element)` - Check if user is scrolled near the bottom
- `scrollToBottom(element, smooth, threadId)` - Scroll to bottom
- `saveScrollPosition(threadId, element)` - Save position for thread
- `restoreScrollPosition(threadId, element)` - Restore saved position
- `enableAutoScroll(threadId)` - Enable auto-scroll for thread
- `disableAutoScroll(threadId)` - Disable auto-scroll for thread
- `isAutoScrollEnabled(threadId)` - Check if auto-scroll is enabled

### Notifications (`notifications.ts`)
- `initNotifications()` - Initialize notification permissions
- `notifyAssistantComplete(title, body)` - Send notification when assistant finishes

### Scroll to Bottom Button
- Floating button in bottom-right corner
- Pulses during streaming (CSS animation)
- Appears when user is not near bottom
- Accessible via keyboard
