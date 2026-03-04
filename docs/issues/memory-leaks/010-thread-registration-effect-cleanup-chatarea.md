# Issue 010: Thread registration effect cleanup in ChatArea

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md
- **Location**: src-ui/src/lib/components/chat/ChatArea.svelte:244-288
- **Priority**: Low
- **Category**: Memory Leaks
- **Status**: Open

## Description
Thread registration effect should include cleanup for unmounting.

## Impact
When the ChatArea component unmounts:
- The thread registration effect doesn't clean up registered threads
- Scroll controller references might persist in memory
- Unregistered threads might cause memory leaks
- Scroll state might not be properly cleaned up

## Suggested Fix
Add cleanup to effect that unregisters threads and cleans up scroll.

```typescript
// In ChatArea.svelte
$: onMount(() => {
  // Register threads with thread manager
  const threads = registeredThreads.get().map(thread => ({
    id: thread.id,
    messages: thread.messages,
    register: () => registerThread(thread),
    unregister: () => unregisterThread(thread),
    onMessage: () => handleThreadMessage(thread),
    onScroll: (position: number) => handleThreadScroll(thread.id, position),
    onResize: () => handleThreadResize(thread.id),
  }));

  registerThreads(threads);

  return () => {
    // Cleanup: unregister all threads
    unregisterThreads(threads);

    // Clean up scroll controller references
    threads.forEach(thread => {
      scrollController.unregisterThread(thread.id);
    });
  };
});

// Alternative with explicit $effect
$effect(() => {
  const threads = registeredThreads.get();

  // Register threads
  threads.forEach(thread => {
    registerThread(thread);
  });

  // Cleanup function
  return () => {
    // Unregister all threads
    threads.forEach(thread => {
      unregisterThread(thread);
      scrollController.unregisterThread(thread.id);
    });
  };
});
```

## Related Components
- ChatArea.svelte
- ThreadPanel.svelte
- ScrollController
- ThreadManager

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md