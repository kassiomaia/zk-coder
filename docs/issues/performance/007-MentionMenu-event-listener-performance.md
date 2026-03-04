# Event listener performance in MentionMenu

## Metadata
- **Source Review**: docs/reviews/code-reviewer/mention-menu-review-2026-03-04.md
- **Location**: src-ui/src/lib/components/chat/input/MentionMenu.svelte:70-73
- **Priority**: Medium
- **Category**: Performance
- **Status**: Open

## Description
The keyboard event listener is attached to the document level, which could cause performance issues with multiple instances.

## Impact
Document-level event listeners can cause:
- Performance bottlenecks with multiple MentionMenu instances
- Unnecessary event propagation for keyboard events
- Potential memory leaks if not properly cleaned up
- Reduced responsiveness in complex UI scenarios

## Suggested Fix
Use event delegation or attach listeners to a specific container element instead of the document.

```javascript
// Current problematic implementation
onMount(() => {
  document.addEventListener('keydown', handleKeyDown);
});

// Better: Use event delegation
onMount(() => {
  // Attach to nearest container
  const container = mentionMenuRef?.parentElement;
  if (container) {
    container.addEventListener('keydown', handleKeyDown);
  }
});

// Alternative: Use Svelte's event forwarding
<div on:keydown|preventDefault={handleKeyDown}>
  <!-- Mention menu content -->
</div>
```

## Related Components
- MentionMenu.svelte
- Chat input components

## References
- Original review: docs/reviews/code-reviewer/mention-menu-review-2026-03-04.md