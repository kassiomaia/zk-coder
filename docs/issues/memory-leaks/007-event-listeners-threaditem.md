# Issue 007: Event listeners in ThreadItem

## Metadata
- **Source Review**: docs/reviews/code-reviewer/20260304_040239.md
- **Location**: ThreadItem.svelte:70-71
- **Priority**: Medium
- **Category**: Memory Leaks
- **Status**: Open

## Description
The component uses mouseenter/mouseleave event listeners that could potentially cause memory leaks if the component is frequently recreated.

## Impact
When mouseenter/mouseleave event listeners are added directly to elements without proper cleanup:
- Event listeners persist even after component destruction
- Each recreation of the component adds more listeners to the DOM
- This leads to memory accumulation
- Can cause performance degradation over time
- May trigger unexpected behavior when events fire on destroyed components

## Suggested Fix
Use Svelte's built-in event handling or consider using on: directives for better cleanup.

```typescript
// Option 1: Use Svelte's on: directives
<div on:mouseenter={handleMouseEnter} on:mouseleave={handleMouseLeave}>
  <!-- component content -->
</div>

// Option 2: If using addEventListener, store reference and cleanup
let element: HTMLDivElement;

$: onDestroy(() => {
  if (element) {
    element.removeEventListener('mouseenter', handleMouseEnter);
    element.removeEventListener('mouseleave', handleMouseLeave);
  }
});
```

## Related Components
- ThreadItem.svelte
- ConversationSidebar.svelte
- ThreadPanel.svelte

## References
- Original review: docs/reviews/code-reviewer/20260304_040239.md