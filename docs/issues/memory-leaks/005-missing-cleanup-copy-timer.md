# Issue 005: Missing cleanup for copy timer in MessageActions

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-MessageActions.svelte.md
- **Location**: src-ui/src/lib/components/chat/MessageActions.svelte:52-67
- **Priority**: Medium
- **Category**: Memory Leaks
- **Status**: Open

## Description
The timeout for resetting the copied state should be cleaned up when the component is destroyed. Potential memory leak if the component is destroyed while the copy timer is active.

## Impact
When the MessageActions component is destroyed while a copy timeout is active, the timeout will continue to exist in memory and attempt to update a non-existent component's state. This can lead to:
- Memory accumulation over time if components are frequently recreated
- Potential errors when the timeout fires and tries to access destroyed DOM elements
- Unpredictable behavior in the application's copy functionality

## Suggested Fix
Store the timer ID and cleanup on component destroy.

```typescript
// In MessageActions.svelte
let copiedTimer: NodeJS.Timeout | null = null;

function copyToClipboard(text: string) {
  // ... existing code ...

  // Store timer ID
  copiedTimer = setTimeout(() => {
    setCopied(false);
  }, 2000);
}

// Cleanup on component destroy
$: onDestroy(() => {
  if (copiedTimer) {
    clearTimeout(copiedTimer);
    copiedTimer = null;
  }
});
```

## Related Components
- MessageActions.svelte
- ChatMessage.svelte (parent component)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-MessageActions.svelte.md