# Scroll System Architecture

> **Document Status**: Reference Documentation
> **Last Updated**: 2026-03-02
> **Purpose**: Preserve the current working implementation for future reference

## Overview

The scroll system is a sophisticated multi-layer architecture that handles auto-scrolling during LLM streaming while respecting user scroll intentions. It prevents the "fighting" problem where auto-scroll conflicts with user scrolling.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ChatArea.svelte                              │
│  (Consumer - uses the unified API)                                   │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      scrollController.ts                             │
│  (Facade - unified API for components)                               │
│                                                                      │
│  Public API:                                                         │
│  - registerThreadElement() / unregisterThread()                      │
│  - handleStreamingStart() / handleStreamingEnd()                     │
│  - handleContentUpdate()                                             │
│  - handleUserScroll() / handleUserClickScrollDown()                  │
│  - scrollToBottom() / enableAutoScroll() / disableAutoScroll()       │
└──────────┬──────────────────┬──────────────────┬─────────────────────┘
           │                  │                  │
           ▼                  ▼                  ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│ scrollStateMachine│ │ liveScrollAnimation│ │   domBatcher    │
│                  │ │                  │ │                  │
│ States:          │ │ Features:        │ │ Features:        │
│ - idle           │ │ - Chasing anim   │ │ - Batched reads  │
│ - streaming      │ │ - Priority-aware │ │ - Batched writes │
│ - user_paused    │ │ - Preemptible    │ │ - RAF-based      │
│ - resuming       │ │                  │ │                  │
└──────────────────┘ └──────────────────┘ └──────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    scrollPriorityQueue.ts                            │
│                                                                      │
│  Priority Order (highest to lowest):                                 │
│  1. user - Manual scroll, button click, send message                 │
│  2. programmatic - Auto-scroll triggered by app                      │
│  3. background - Streaming content updates                           │
│                                                                      │
│  Higher priority operations preempt lower priority ones mid-animation│
└─────────────────────────────────────────────────────────────────────┘
```

## Core Files

| File | Lines | Purpose |
|------|-------|---------|
| `scrollController.ts` | ~715 | Unified facade coordinating all subsystems |
| `scrollStateMachine.ts` | ~414 | Explicit state management with transitions |
| `liveScrollAnimation.ts` | ~429 | Chasing animation for streaming content |
| `scrollPriorityQueue.ts` | ~435 | Priority-based operation scheduling |
| `domBatcher.ts` | ~354 | Batched DOM reads/writes to prevent thrashing |
| `scrollManager.ts` | ~340 | Legacy position storage (session-only) |

## State Machine

### States

```
┌─────────┐     STREAM_START     ┌───────────┐
│  idle   │ ──────────────────► │ streaming │
└─────────┘                     └───────────┘
     ▲                                │
     │                          USER_SCROLL_UP
     │                                │
     │                                ▼
     │    STREAM_END           ┌─────────────┐
     └──────────────────────── │ user_paused │
                                └─────────────┘
                                      │
                           USER_SCROLL_TO_BOTTOM
                           USER_CLICK_SCROLL_DOWN
                                      │
                                      ▼
                                ┌──────────┐
                                │ resuming │
                                └──────────┘
                                      │
                           STREAM_START (back to streaming)
```

### State Descriptions

| State | Description | Auto-Scroll |
|-------|-------------|-------------|
| `idle` | No streaming, user can scroll freely | Enabled |
| `streaming` | Content is being streamed | Enabled |
| `user_paused` | User scrolled up during streaming | Disabled |
| `resuming` | User scrolled back to bottom | Enabled |

### Events

| Event | Trigger |
|-------|---------|
| `STREAM_START` | LLM starts streaming content |
| `STREAM_END` | LLM finishes streaming |
| `USER_SCROLL_UP` | User scrolls upward (away from bottom) |
| `USER_SCROLL_TO_BOTTOM` | User scrolls to bottom manually |
| `USER_CLICK_SCROLL_DOWN` | User clicks scroll-to-bottom button |
| `USER_SEND_MESSAGE` | User sends a new message |
| `THREAD_CHANGE` | Active thread changes |

## Key Behaviors

### 1. Auto-Scroll During Streaming

When content is being streamed:
- If user is at bottom: Auto-scroll follows content smoothly
- If user scrolls up: Auto-scroll pauses immediately
- If user scrolls back to bottom: Auto-scroll resumes

### 2. Scroll Thumb Detection

The system detects when user is dragging the scroll thumb:
```typescript
// Mouse event listeners detect scrollbar area clicks
const handleMouseDown = (e: MouseEvent) => {
    const rect = element.getBoundingClientRect();
    const scrollbarWidth = rect.width - element.clientWidth;
    const isOnScrollbar = e.clientX >= rect.right - scrollbarWidth;
    if (isOnScrollbar) {
        context.isDraggingThumb = true;
    }
};
```

### 3. Chasing Animation

For background priority (streaming), the animation "chases" the live bottom:
```typescript
if (state.priority === 'background') {
    // Update target to current bottom (chasing)
    state.targetScrollTop = calculateTargetBottom(element);
}
```

### 4. Priority Preemption

Higher priority operations cancel lower priority ones:
- User click → cancels background animation immediately
- User send message → cancels any running animation, scrolls instantly

### 5. Programmatic Scroll Ignoring

To prevent scroll events from programmatic scrolls being treated as user scrolls:
```typescript
function withIgnoredScrollEvents(threadId: string, fn: () => void): void {
    context.ignoreScrollEvents = true;
    try {
        fn();
    } finally {
        setTimeout(() => {
            context.ignoreScrollEvents = false;
            context.lastScrollTop = context.element.scrollTop;
        }, 0);
    }
}
```

## Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `AT_BOTTOM_THRESHOLD` | 50px | Distance considered "at bottom" |
| `SCROLL_UP_THRESHOLD` | 10px | Minimum scroll to detect upward movement |
| `USER_SCROLL_COOLDOWN` | 150ms | Cooldown after user scroll |
| `USER_SCROLL_UP_COOLDOWN` | 500ms | Extended cooldown when scrolling up |
| `DEFAULT_DURATION` | 300ms | Animation duration |
| `SCROLL_BUTTON_THRESHOLD` | 100px | When to show scroll-to-bottom button |

## ChatArea Integration

The `ChatArea.svelte` component integrates with the scroll system:

```typescript
// Register thread on mount
$effect(() => {
    if (ref && currentThreadId) {
        registerThreadElement(currentThreadId, ref);
        initializeThreadScroll(currentThreadId);
    }
});

// Handle streaming state changes
$effect(() => {
    if (!wasStreaming && streaming) {
        handleStreamingStart(currentThreadId);
    }
    if (wasStreaming && !streaming) {
        handleStreamingEnd(currentThreadId);
    }
});

// Handle content updates
$effect(() => {
    void messages.length;
    void $streamingContent.length;
    handleContentUpdate(currentThreadId);
});
```

## Testing Checklist

When verifying scroll behavior works correctly:

### Streaming Auto-Scroll
- [ ] New content during streaming scrolls to bottom automatically
- [ ] Smooth animation, not jarring jumps
- [ ] Works consistently across multiple messages

### User Pause/Resume
- [ ] Scrolling up during streaming stops auto-scroll
- [ ] No "fighting" between user and auto-scroll
- [ ] Scrolling back to bottom resumes auto-scroll
- [ ] Click "scroll to bottom" button resumes auto-scroll

### Thread Switching
- [ ] Scroll position preserved when switching threads
- [ ] Position restored when switching back
- [ ] No visual glitches during switch

### Edge Cases
- [ ] Sending message always scrolls to bottom
- [ ] Stream completion leaves view at bottom
- [ ] Very long messages scroll correctly
- [ ] Rapid content updates don't cause jank

## What NOT to Change

These are critical behaviors that must be preserved:

1. **State Machine Transitions**: The explicit state transitions ensure predictable behavior
2. **Scroll Thumb Detection**: Essential for detecting user intent
3. **Priority Preemption**: User actions must always win over background operations
4. **Chasing Animation**: The live target update during streaming is key to smooth UX
5. **Programmatic Scroll Ignoring**: Prevents infinite loops and false user detection

## Future Enhancements (Safe)

These can be done without touching scroll logic:

- CSS-only scrollbar styling (colors, width, hover states)
- Keyboard shortcuts for scroll actions
- Haptic feedback on scroll boundaries (if supported)

## Future Enhancements (Risky)

These would require careful testing:

- Scroll position persistence across sessions
- Virtual scrolling for very long conversations
- Scroll position sharing across devices
