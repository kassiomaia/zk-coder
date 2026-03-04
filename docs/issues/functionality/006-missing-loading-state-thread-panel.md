# Missing loading state in ThreadPanel

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md
- **Location**: ThreadPanel.svelte:332
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
The component has loading states but lacks loading spinners for the banner animation when reopening.

## Impact
Users may not realize that reopening a thread is an asynchronous operation, leading to confusion when the UI doesn't respond immediately.

## Suggested Fix
Add a loading spinner to the "Unlock" button when `isReopening` is true.

```typescript
// Example fix for ThreadPanel.svelte
const handleReopen = async () => {
  if (!isReopening && thread.isResolved) {
    isReopening = true;
    try {
      await reopenThread(thread.id);
      // Success handling
    } catch (error) {
      console.error('Failed to reopen thread:', error);
      // Show error message
    } finally {
      isReopening = false;
    }
  }
};

// In template:
<button
  on:click={handleReopen}
  disabled={isReopening}
  class:resolved={thread.isResolved}
>
  {#if isReopening}
    <Spinner size="small" />
    <span>Unlocking...</span>
  {:else}
    {#if thread.isResolved}
      <span>Unlock</span>
    {:else}
      <span>Lock</span>
    {/if}
  {/if}
</button>
```

## Related Components
- ThreadPanel.svelte
- Spinner component
- Thread service (reopenThread function)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md