# Auto-scroll Feature

## Overview

Auto-scroll functionality ensures message content flows smoothly during chat conversations in the Coder AI Code Agent application.

## Behavior

### Auto-Scroll Zone
The "auto-scroll zone" is an imaginary area within **30px of the bottom**. When the user's scroll position is in this zone:
- Auto-scroll is active during streaming
- Content automatically scrolls down smoothly
- Uses debounced scrolling (150ms) to prevent jitter

### Streaming Lock Mechanism
Once streaming starts and user is in the zone, auto-scroll is **locked** for that streaming session:
- Even if content growth pushes scroll thumb "out of zone", auto-scroll continues
- Only **manual user scroll UP** can release the lock
- This prevents accidental disengagement during long responses

### Releasing the Lock
Auto-scroll lock is released when:
1. **User manually scrolls UP** (drags scroll thumb upward)
2. **Streaming completes**
3. **User switches threads**

### Outside the Zone
When user scrolls up (outside zone):
- Auto-scroll is disabled
- "Scroll to Bottom" button appears
- Button pulses during streaming to indicate activity

### Scroll Button Visibility
The scroll-to-bottom button appears when user is **more than 100px from bottom**.

### Ways to Return to Bottom
1. **Click scroll button** - Smooth scroll, locks if streaming
2. **Manually scroll to bottom** - Enter zone, auto-scroll engages
3. **Send a message** - Enforced scroll to bottom (150ms delay for message visibility)

### New Threads
When opening a thread for the first time:
- Scroll starts at bottom (instant, no animation)
- No visual "flash" of seeing top before scrolling

## Technical Details

| Parameter | Value | Purpose |
|-----------|-------|---------|
| Auto-scroll zone | 30px | Threshold for engaging auto-scroll |
| Button visibility | 100px | Threshold for showing scroll button |
| User scroll detection | 5px | Threshold to detect manual scroll UP |
| Debounce delay | 150ms | Prevents jitter during streaming |
| Scroll animation | 300ms | Smooth scroll duration |
| Send message delay | 150ms | Wait for message to render before scroll |

## User Flow

```
Streaming starts + user in zone (≤30px)
    │
    ├── Lock engaged → Auto-scroll continues
    │
    ├── Content grows → Thumb moves but LOCK keeps scrolling
    │
    └── User manually scrolls UP (>5px)
            │
            ├── Lock released
            ├── Auto-scroll disabled
            ├── Button appears + pulses
            │
            └── User returns via:
                    ├── Click button (locks again if streaming)
                    ├── Scroll to bottom
                    └── Send message
```
