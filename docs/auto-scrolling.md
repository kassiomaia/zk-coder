# Auto-scroll Feature

## Overview

Auto-scroll functionality ensures message content flows smoothly during chat conversations in the Coder AI Code Agent application.

## Behavior
### New Threads
When a user opens a thread for the first time, the chat automatically scrolls to the bottom to show the latest messages.
- **Scroll Position Memory**: When switching between threads, the scroll position is remembered ( restored (only in memory, not persisted to storage)
- **User Scroll Detection**: If the user scrolls up while the assistant is responding, auto-scroll is temporarily disabled
- **Auto-Scroll Resume**: After a brief pause (500ms), auto-scroll re-engages to allow the user to continue following the conversation
- **OS Notification**: When the assistant finishes a message while the user is scrolled up (not near the bottom) AND the app is not focused, the system notification triggers with sound (if available)
- **Send a new message**: When user sends a new message, auto-scroll is re-enabled, and the view scrolls to the latest response

- **Scroll to Bottom Button**: A floating button appears when the user scrolls up, allowing quick navigation back to the bottom

## User Flow
- **Smooth scrolling**: Animated using requestAnimationFrame for precise timing control ( 300-400ms duration)
- **Scroll position memory**: Session-only Map-based storage, uses the thread ID as key
- **Auto-scroll enabled/disabled**: Boolean flag per thread, managed with the scroll manager utility
- **Near bottom threshold**: 100px from the bottom
- **Auto-scroll resume delay**: 500ms delay before user inactivity before auto-scroll resumes
- **User preferences**:
  - **Scroll position memory**: Session-only Map (cleared on app closes)
  - **Smooth animation**: Uses `easeOutQuad` easing function for natural feel
  - **Animation timing**: 300-400ms (can be adjusted in scrollManager.ts)
  - **Scroll to Bottom Button**: Appears when user scrolls up, disappears when scrolled to the bottom

  - **OS Notification**: Trigger when:
    - Assistant finishes responding while user is scrolled up (not near bottom)
    - App is in the background
    - OR (2) When user was scrolled up and assistant finishes, OR (3) the app is in background
    - User is scrolling up to view the response

    - **Resume delay**: 500ms timeout before auto-scroll resume
    - **Notification Sound**: If available on the OS, notification system
    - **Animation**:
    - `easeOutQuad` function for smooth scroll animation (uses `requestAnimationFrame` for frame-by-frame animation.
    - 300-400ms duration
    - **Scroll Manager** (`scrollManager.ts`):
    - `isNearBottom(element)` - Check if user is scrolled near the bottom
    - `getScrollProgress(element)` - Get scroll progress as percentage (0-1)
    - `restoreScrollPosition(threadId, element)` - Restore saved position or scroll to bottom
    - `hasSavedPosition(threadId)` - Check if a thread has a saved scroll position
    - `clearScrollPosition(threadId)` - Clear saved position for a thread
    - `clearAllScrollPositions()` - Clear all saved scroll positions
- **Notifications** (`notifications.ts`)
    - `initNotifications()` - Initialize notification permissions (should be called early in app lifecycle)
    - `canSendNotifications()` - Check if notifications are available and permitted
    - `notifyAssistantComplete(title, body)` - Send a notification when assistant finishes responding
    - `sendCustomNotification(title, body)` - Send a custom notification with custom title and body
- **Scroll to Bottom Button Component** (`ChatArea.svelte`, `ChatArea.module.css`)
    - Floating button that appears at the bottom-right corner of the chat area
    - Allows clicking to button to manually scroll to bottom
    - Smooth animated scroll
    - Hidden when user is near the bottom
    - `showScrollButton` state controls visibility

    - Styling follows the design system
    - **Accessibility**: Accessible via keyboard (Enter focuses button, Tab key) or Escape key to scrolling)

    - Aria-label for screen readers
    - **Performance**: Uses native smooth scrolling with fallback instant scroll
    - **User Flow**:
    - Auto-scroll to ensures conversation flow remains live
        - As messages are sent/received, the chat automatically scrolls down to keep the latest messages always visible
    - **Auto-scroll Toggle**: If the user scrolls up while the assistant is responding, the auto-scroll is temporarily disabled to        The user can read previous messages without interruption
        - **Resume after inactivity**: After 500ms of user inactivity, auto-scroll resumes and tracks the bottom again
        - **Scroll to Bottom Button**: A floating button that appears when the user scrolls up, allowing them to quickly return to the conversation flow
        - **OS Notifications**: When the assistant finishes responding while the user is scrolled up (not near bottom), or/or app is in the background, an OS notification with sound to alert the user

        - **Re-enabling Auto-Scroll**: User can re-enable auto-scroll by sending a new message
            - This also scrolls the chat to the bottom, making the latest response visible
        - **Integration Points**:
    - ChatArea component ( `ChatArea.svelte`) receives `threadId` prop
    - Uses the scroll manager and notification utility
    - ThreadPanel component passes `threadId` to ChatArea

    - ChatLayout component passes `selectedThreadId` to ChatArea instances

- **CSS Module**: `ChatArea.module.css` contains styling for the scroll container and scroll-to-bottom button

- **Design Tokens**: Uses colors, spacing, typography from the design system (`design-system.css`)