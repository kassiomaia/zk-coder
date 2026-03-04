# [ISSUE-003] - Animation Delays in Thread View

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md`
- **Component/File**: `src-ui/src/lib/components/ThreadPanel.svelte`
- **Line Numbers**: TBD

## Description
The thread view component has hardcoded animation delays that are not optimized for different devices and network conditions. These delays cause perceived performance issues and make the UI feel sluggish.

## Impact
Performance - This affects:
- User perception of application responsiveness
- UI fluidity when switching between threads
- Apparent loading times
- User satisfaction with the interface

## Suggested Fix
Implement dynamic animation delays based on device capabilities:

```typescript
// Detect device capabilities
const isSlowDevice = navigator.hardwareConcurrency < 4;
const isSlowNetwork = navigator.connection?.effectiveType.includes('2g');

$: animationDelay = isSlowDevice || isSlowNetwork ? 300 : 150;

// Use CSS variables for animation timing
:global {
  --thread-transition-delay: calc(var(--base-delay) * 0.8);
}

// Apply dynamically
transition: all var(--thread-transition-delay) ease-in-out;
```

## Related Issues
- None