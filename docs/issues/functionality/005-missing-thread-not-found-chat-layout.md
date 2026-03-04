# [ISSUE-005] - Missing Thread Not Found in Chat Layout

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/ChatLayout.svelte_2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/ChatLayout.svelte`
- **Line Numbers**: TBD

## Description
The ChatLayout component doesn't handle the case when a thread is not found. Users trying to access non-existent threads get blank screens or broken layouts instead of helpful error messages.

## Impact
Functionality - This affects:
- User experience with invalid thread access
- Error handling consistency
- Application reliability
- User confusion

## Suggested Fix
Add proper 404 handling for threads:

```typescript
$: threadNotFound = $derived(() => {
  return !thread && threadId;
});

// In template
{#if threadNotFound}
  <div class="thread-not-found">
    <Icon name="alert-circle" size="48" />
    <h2>Thread Not Found</h2>
    <p>The conversation thread you're looking for doesn't exist or has been deleted.</p>
    <Button on:click={goToDashboard}>
      Back to Dashboard
    </Button>
  </div>
{:else if loading}
  <div class="loading-state">...</div>
{:else}
  <ChatArea thread={thread} />
{/if}

// Handle in router or parent component
function loadThread() {
  if (!threadId) {
    // Handle empty thread ID
    return;
  }

  threadService.getThread(threadId)
    .then(data => {
      thread = data;
    })
    .catch(error => {
      if (error.code === 'THREAD_NOT_FOUND') {
        threadNotFound = true;
      } else {
        // Handle other errors
      }
    });
}
```

## Related Issues
- None