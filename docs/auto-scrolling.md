# Auto-scroll Feature

## Overview

Auto-scroll functionality ensures message content flows smoothly during chat conversations in the Coder AI Code Agent application.

## Behavior

### New Threads
When a user opens a thread for the first time, the chat automatically scrolls to the bottom to show the latest messages.

### User Scroll Detection
- **Scroll Position Memory**: When switching between threads, the scroll position is remembered (restored only in memory, not persisted to storage)
- **User Scrolls Up**: Auto-scroll is immediately disabled, allowing user to read previous messages
- **Scroll to Bottom Button**: A floating button appears when user scrolls up, allowing quick navigation back to bottom
- **User Scrolls to Bottom**: Auto-scroll is immediately re-enabled

### Auto-Scroll Resume (After User Scrolls Up)
When user releases scroll and stops for 1 second:
- **IF streaming**: Auto-scroll re-engages AND scrolls to bottom (follow the conversation)
- **IF NOT streaming**: Auto-scroll is re-enabled but scroll position is preserved (user stays where they left off)

### Ways to Re-enable Auto-Scroll
1. User clicks "Scroll to Bottom" button
2. User manually scrolls to bottom
3. User sends a new message
4. 1 second after releasing scroll (if streaming)

### OS Notifications
When the assistant finishes a message while the user is scrolled up (not near the bottom) AND the app is not focused, the system notification triggers with sound (if available).

## Technical Details

- **Smooth scrolling**: Animated using requestAnimationFrame with easeOutQuad easing (300ms duration)
- **Scroll position memory**: Session-only Map-based storage, uses the thread ID as key
- **Auto-scroll enabled/disabled**: Boolean flag per thread, managed with the scroll manager utility
- **Near bottom threshold**: 100px from the bottom
- **Resume delay**: 1000ms (1 second) after user stops scrolling

## API Reference

### Scroll Manager (`scrollManager.ts`)
- `isNearBottom(element)` - Check if user is scrolled near the bottom
- `getScrollProgress(element)` - Get scroll progress as percentage (0-1)
- `restoreScrollPosition(threadId, element)` - Restore saved position or scroll to bottom
- `hasSavedPosition(threadId)` - Check if a thread has a saved scroll position
- `clearScrollPosition(threadId)` - Clear saved position for a thread
- `clearAllScrollPositions()` - Clear all saved scroll positions
- `scrollToBottom(element, smooth, threadId)` - Scroll to bottom with optional animation tracking

### Notifications (`notifications.ts`)
- `initNotifications()` - Initialize notification permissions
- `canSendNotifications()` - Check if notifications are available and permitted
- `notifyAssistantComplete(title, body)` - Send notification when assistant finishes responding
- `sendCustomNotification(title, body)` - Send custom notification

### Scroll to Bottom Button
- Floating button in bottom-right corner of chat area
- Appears when user is not near bottom
- Accessible via keyboard
- Smooth animated scroll on click
