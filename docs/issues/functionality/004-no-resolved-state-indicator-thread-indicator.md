# [ISSUE-004] - No Resolved State Indicator in Thread Indicator

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-code-reviewer-thread-indicator.md`
- **Component/File**: `src-ui/src/lib/components/ThreadIndicator.svelte`
- **Line Numbers**: TBD

## Description
The ThreadIndicator component doesn't show the resolved/locked state of threads. Users cannot visually distinguish between open and resolved threads, making it difficult to understand thread status.

## Impact
Functionality - This affects:
- User understanding of thread status
- Navigation clarity
- Thread management capability
- User experience with thread lifecycle

## Suggested Fix
Add resolved state indicator:

```typescript
interface ThreadIndicatorProps {
  thread: Thread;
  size?: 'sm' | 'md' | 'lg';
}

// In template
<div class="thread-indicator {thread.resolved ? 'resolved' : ''}">
  <div class="thread-status">
    {#if thread.resolved}
      <Icon name="lock" class="resolved-icon" />
      <span class="sr-only">Thread resolved</span>
    {:else}
      <Icon name="unlock" class="open-icon" />
      <span class="sr-only">Thread open</span>
    {/if}
  </div>
  <div class="thread-title">
    {thread.title}
  </div>
  {#if thread.unread}
    <div class="unread-indicator"></div>
  {/if}
</div>

// Add styles for resolved state
.thread-indicator.resolved {
  opacity: 0.7;
  border-color: var(--color-warning);
}
```

## Related Issues
- None