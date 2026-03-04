# [ISSUE-002] - Complex Effect Blocks in ChatArea Component

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md`
- **Component/File**: `src-ui/src/lib/components/ChatArea.svelte`
- **Line Numbers**: TBD

## Description
The ChatArea component contains complex effect blocks that perform multiple expensive operations during each message update. These effects are not optimized and cause performance bottlenecks during active conversations.

## Impact
Performance - This causes:
- UI lag when messages are added
- Reduced scrolling performance
- High CPU usage during message streaming
- Poor user experience with large message histories

## Suggested Fix
Optimize effect blocks by separating concerns and using debouncing:

```typescript
// Separate concerns into different effects
$: debouncedScroll = debounce(() => {
  scrollController.scrollToBottom();
}, 50);

$: {
  // Only scroll when new messages are added
  if (messages.length > previousMessagesLength) {
    debouncedScroll();
  }
}

// Separate effect for DOM updates
$: {
  // Batch DOM updates
  const elements = messageElements;
  if (elements.length > 0) {
    requestAnimationFrame(() => {
      // Perform DOM updates
    });
  }
}
```

## Related Issues
- None