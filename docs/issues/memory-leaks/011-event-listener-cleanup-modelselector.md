# Issue 011: Event listener cleanup in ModelSelector

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ModelSelector.svelte.md
- **Location**: ModelSelector.svelte:96-99,111-115
- **Priority**: Medium
- **Category**: Memory Leaks
- **Status**: Open

## Description
The event listeners added in $effect hooks are properly cleaned up, but the cleanup should be more robust to handle edge cases.

## Impact
While the cleanup is present, it might not handle all edge cases:
- Component destruction during async operations
- Multiple rapid unmounts/remounts
- Errors in cleanup functions
- Missing cleanup in error scenarios

This could lead to:
- Incomplete cleanup leaving event listeners in memory
- Memory leaks in production environments with complex interactions
- Potential errors when cleanup fails

## Suggested Fix
Add error handling in cleanup functions to ensure event listeners are always removed.

```typescript
// In ModelSelector.svelte
$: onMount(() => {
  let scrollCleanup: (() => void) | null = null;
  let resizeCleanup: (() => void) | null = null;

  try {
    // Add scroll event listener
    const handleScroll = (e: Event) => {
      // Handle scroll logic
    };
    window.addEventListener('scroll', handleScroll, { passive: true });
    scrollCleanup = () => {
      try {
        window.removeEventListener('scroll', handleScroll);
      } catch (error) {
        console.error('Error removing scroll listener:', error);
      }
    };

    // Add resize event listener
    const handleResize = (e: Event) => {
      // Handle resize logic
    };
    window.addEventListener('resize', handleResize);
    resizeCleanup = () => {
      try {
        window.removeEventListener('resize', handleResize);
      } catch (error) {
        console.error('Error removing resize listener:', error);
      }
    };
  } catch (error) {
    console.error('Error adding event listeners:', error);
  }

  return () => {
    // Ensure cleanup runs even if setup failed
    if (scrollCleanup) scrollCleanup();
    if (resizeCleanup) resizeCleanup();
  };
});

// Alternative with defensive cleanup
$: onDestroy(() => {
  // Multiple cleanup attempts with error handling
  cleanupEventListeners();
});

function cleanupEventListeners() {
  try {
    // Cleanup scroll listeners
    const scrollElements = document.querySelectorAll('.model-selector-scroll');
    scrollElements.forEach(el => {
      el.removeEventListener('scroll', handleScroll);
    });
  } catch (error) {
    console.error('Error cleaning up scroll listeners:', error);
  }

  try {
    // Cleanup resize listeners
    window.removeEventListener('resize', handleResize);
  } catch (error) {
    console.error('Error cleaning up resize listeners:', error);
  }
}
```

## Related Components
- ModelSelector.svelte
- ChatArea.svelte
- ConversationSidebar.svelte

## References
- Original review: docs/reviews/code-reviewer/ModelSelector.svelte.md