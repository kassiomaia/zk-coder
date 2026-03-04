# [ISSUE-004] - Animation State Management in AnsweringIndicator

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md`
- **Component/File**: `src-ui/src/lib/components/AnsweringIndicator.svelte`
- **Line Numbers**: TBD

## Description
The AnsweringIndicator component has inefficient animation state management that causes unnecessary re-renders and CSS recalculations. The component updates too frequently during AI response streaming, leading to performance degradation.

## Impact
Performance - This causes:
- Unnecessary DOM updates during streaming
- CSS layout thrashing
- Increased CPU usage
- Poor user experience with stuttering animations

## Suggested Fix
Implement efficient animation state management:

```typescript
// Use RAF for animation updates
let animationFrame: number;

$: {
  if (isAnswering) {
    cancelAnimationFrame(animationFrame);
    animationFrame = requestAnimationFrame(() => {
      // Update animation state
      updateDots();
    });
  } else {
    cancelAnimationFrame(animationFrame);
  }
}

// Throttle dot updates
function updateDots() {
  // Only update when necessary
  if (Date.now() - lastUpdate > 100) {
    dotIndex = (dotIndex + 1) % 3;
    lastUpdate = Date.now();
  }
}
```

## Related Issues
- None