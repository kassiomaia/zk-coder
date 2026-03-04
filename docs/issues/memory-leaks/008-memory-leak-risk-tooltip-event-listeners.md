# Issue 008: Memory leak risk in Tooltip event listeners

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md
- **Location**: src-ui/src/lib/components/shared/Tooltip.svelte:282-295
- **Priority**: Medium
- **Category**: Memory Leaks
- **Status**: Open

## Description
The scroll and resize event listeners are added in an $effect but there's a potential race condition when the component unmounts quickly after becoming visible.

## Impact
When the Tooltip component unmounts quickly after becoming visible:
- The $effect cleanup might not run in time
- Event listeners (scroll, resize) remain attached to the window
- Each tooltip creation adds more listeners without proper cleanup
- This leads to memory accumulation
- Can cause performance degradation and unexpected behavior

## Suggested Fix
Ensure cleanup runs synchronously by using a mounted flag.

```typescript
// In Tooltip.svelte
let isMounted = false;
let scrollCleanup: (() => void) | null = null;
let resizeCleanup: (() => void) | null = null;

$: onMount(() => {
  isMounted = true;

  // Add scroll listener
  const handleScroll = () => {
    // Handle scroll logic
  };
  window.addEventListener('scroll', handleScroll);

  // Add resize listener
  const handleResize = () => {
    // Handle resize logic
  };
  window.addEventListener('resize', handleResize);

  // Store cleanup functions
  scrollCleanup = () => window.removeEventListener('scroll', handleScroll);
  resizeCleanup = () => window.removeEventListener('resize', handleResize);
});

$: onDestroy(() => {
  isMounted = false;
  // Ensure cleanup runs synchronously
  if (scrollCleanup) {
    scrollCleanup();
    scrollCleanup = null;
  }
  if (resizeCleanup) {
    resizeCleanup();
    resizeCleanup = null;
  }
});
```

## Related Components
- Tooltip.svelte
- MessageActions.svelte
- ChatMessage.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md