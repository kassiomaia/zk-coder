# Issue 006: Potential memory leak in ThreadPanel timeout

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md
- **Location**: ThreadPanel.svelte:72-74
- **Priority**: Medium
- **Category**: Memory Leaks
- **Status**: Resolved

## Description
The `triggerRestrictedFeedback` function uses a timeout but doesn't clear it when the component unmounts. This could potentially cause memory leaks.

## Impact
When a ThreadPanel component is destroyed while the timeout is still pending:
- The timeout will continue to exist in memory
- It will attempt to call functions on a destroyed component
- This can lead to memory accumulation over time
- May cause errors when the timeout executes after component destruction

## Proposed Solution
Clear the timeout in a cleanup function using Svelte 5's `$effect` with cleanup return function.

## Resolution
Fixed by adding:
1. A `feedbackTimer` variable to store the timer reference
2. An `$effect` with cleanup function that clears the timer on component destroy
3. Clearing any existing timer before setting a new one to prevent multiple timers

### Implementation
```typescript
// Timer for restricted feedback - cleaned up on component destroy
let feedbackTimer: ReturnType<typeof setTimeout> | null = null;

// Cleanup all timers on component destroy
$effect(() => {
  return () => {
    if (feedbackTimer) {
      clearTimeout(feedbackTimer);
      feedbackTimer = null;
    }
  };
});

function triggerRestrictedFeedback() {
  restrictedFeedback = true;
  // Clear any existing timer before setting a new one
  if (feedbackTimer) {
    clearTimeout(feedbackTimer);
  }
  feedbackTimer = setTimeout(() => {
    restrictedFeedback = false;
    feedbackTimer = null;
  }, 300);
}
```

## Related Components
- ThreadPanel.svelte
- ThreadItem.svelte (parent component)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md