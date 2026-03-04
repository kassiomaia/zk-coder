# Issue 012: Popover cleanup in ConversationSidebar

## Metadata
- **Source Review**: docs/reviews/code-reviewer/code-reviewer-2026-03-04-conversation-sidebar.md
- **Location**: src-ui/src/lib/components/chat/sidebar/ConversationSidebar.svelte:216-223
- **Priority**: Low
- **Category**: Memory Leaks
- **Status**: Open

## Description
Popover might not clean up event listeners properly.

## Impact
If the Popover component in ConversationSidebar doesn't clean up event listeners:
- Event listeners (click, mousemove, etc.) might persist after component unmount
- This can lead to memory leaks
- Might cause unexpected behavior when listeners fire after component destruction
- Performance degradation over time with frequent sidebar interactions

## Suggested Fix
Add explicit cleanup in Svelte's `onDestroy` lifecycle function.

```typescript
// In ConversationSidebar.svelte
let popover: PopoverComponent | null = null;

// Create popover with explicit cleanup
function createPopover() {
  popover = new PopoverComponent({
    target: popoverContainer,
    props: {
      // popover props
    }
  });

  // Store cleanup function
  return () => {
    if (popover) {
      popover.$destroy();
      popover = null;
    }
  };
}

// Use with $effect for automatic cleanup
$effect(() => {
  let cleanup: (() => void) | null = null;

  if (showPopover) {
    cleanup = createPopover();
  }

  return () => {
    if (cleanup) {
      cleanup();
      cleanup = null;
    }
  };
});

// Alternative: Use onDestroy for explicit cleanup
$: onDestroy(() => {
  if (popover) {
    try {
      // Clean up any event listeners
      popover.removeEventListener('click', handlePopoverClick);
      popover.removeEventListener('keydown', handlePopoverKeydown);

      // Destroy the popover instance
      popover.$destroy();
      popover = null;
    } catch (error) {
      console.error('Error cleaning up popover:', error);
    }
  }
});
```

## Related Components
- ConversationSidebar.svelte
- Popover.svelte
- ThreadItem.svelte
- ThreadPanel.svelte

## References
- Original review: docs/reviews/code-reviewer/code-reviewer-2026-03-04-conversation-sidebar.md