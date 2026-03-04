# Issue 006: Potential memory leak in ThreadPanel timeout

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md
- **Location**: ThreadPanel.svelte:72-74
- **Priority**: Medium
- **Category**: Memory Leaks
- **Status**: Open

## Description
The `triggerRestrictedFeedback` function uses a timeout but doesn't clear it when the component unmounts. This could potentially cause memory leaks.

## Impact
When a ThreadPanel component is destroyed while the timeout is still pending:
- The timeout will continue to exist in memory
- It will attempt to call functions on a destroyed component
- This can lead to memory accumulation over time
- May cause errors when the timeout executes after component destruction

## Suggested Fix
Clear the timeout in a cleanup function or use a more reactive approach with Svelte's lifecycle methods.

```typescript
// In ThreadPanel.svelte
let feedbackTimeout: NodeJS.Timeout | null = null;

function triggerRestrictedFeedback() {
  feedbackTimeout = setTimeout(() => {
    showRestrictedFeedback = false;
  }, 3000);
}

// Cleanup on component destroy
$: onDestroy(() => {
  if (feedbackTimeout) {
    clearTimeout(feedbackTimeout);
    feedbackTimeout = null;
  }
});
```

## Related Components
- ThreadPanel.svelte
- ThreadItem.svelte (parent component)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md