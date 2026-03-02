# Auto-scroll Feature

## Overview

Auto-scroll functionality ensures message content flows smoothly during chat conversations in the Coder AI Code Agent application.

## Behavior

### Auto-Scroll Zone
The "auto-scroll zone" is an imaginary area within **30px of the bottom**. When the user's scroll position is in this zone:
- Auto-scroll is active during streaming
- Content automatically scrolls down smoothly
- Uses debounced scrolling (150ms) to prevent jitter

### Outside the Zone
When user scrolls **more than 30px from bottom**:
- Auto-scroll is disabled
- "Scroll to Bottom" button appears
- Button pulses during streaming to indicate activity
- User can read previous messages without interruption

### Scroll Button Visibility
The scroll-to-bottom button appears when user is **more than 100px from bottom** (larger visibility zone than auto-scroll zone).

### Ways to Return to Bottom
1. **Click scroll button** - Smooth scroll to bottom
2. **Manually scroll to bottom** - Enter auto-scroll zone, auto-scroll engages
3. **Send a message** - Enforced scroll to bottom (always scrolls, regardless of position)

### New Threads
When opening a thread for the first time:
- Scroll starts at bottom (instant, no animation)
- No visual "flash" of seeing top before scrolling

### OS Notifications
When assistant finishes responding while user is scrolled up, a system notification is sent.

## Technical Details

| Parameter | Value | Purpose |
|-----------|-------|---------|
| Auto-scroll zone | 30px | Threshold for active auto-scroll during streaming |
| Button visibility | 100px | Threshold for showing scroll button |
| Debounce delay | 150ms | Prevents jitter during streaming |
| Scroll animation | 300ms | Smooth scroll duration |
| Easing | easeOutQuad | Natural scroll feel |

## User Flow

```
User at bottom (in zone)
    │
    ├── Streaming starts → Debounced auto-scroll keeps user at bottom
    │
    ├── User scrolls up (< 30px) → Still in zone, auto-scroll continues
    │
    └── User scrolls up (> 30px) → Out of zone
            │
            ├── Auto-scroll disabled
            ├── Button appears (if > 100px)
            ├── Button pulses if streaming
            │
            └── User returns to bottom via:
                    ├── Manual scroll
                    ├── Click button
                    └── Send message (enforced scroll)
```

## API Reference

### Scroll Manager (`scrollManager.ts`)
- `isNearBottom(element)` - Check if in auto-scroll zone (30px)
- `scrollToBottom(element, smooth, threadId)` - Scroll to bottom
- `debouncedScrollToBottom(threadId, element)` - Debounced scroll for streaming
- `enableAutoScroll(threadId)` / `disableAutoScroll(threadId)` - Toggle auto-scroll

### CSS Classes
- `.scrollToBottom` - Scroll button base styles
- `.pulsing` - Pulsing animation during streaming (2s ease-in-out infinite)
