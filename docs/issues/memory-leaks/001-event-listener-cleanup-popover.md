# [ISSUE-001] - Event Listener Cleanup in Popover Component

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04_Popover-Review.md`
- **Component/File**: `src-ui/src/lib/components/Popover.svelte`
- **Line Numbers**: TBD

## Description
The Popover component adds event listeners but doesn't properly clean them up when the component is destroyed or when the popover is closed. This can lead to memory leaks and potential zombie event listeners.

## Impact
Memory Leaks - This causes:
- Accumulating event listeners over time
- Increased memory usage
- Potential memory exhaustion in long-running sessions
- Performance degradation

## Suggested Fix
Implement proper event listener cleanup:

```typescript
let clickOutsideHandler: () => void;

$: {
  if (open) {
    clickOutsideHandler = () => {
      if (!popoverRef?.contains(event.target as Node)) {
        open = false;
      }
    };
    document.addEventListener('click', clickOutsideHandler);
  } else {
    if (clickOutsideHandler) {
      document.removeEventListener('click', clickOutsideHandler);
      clickOutsideHandler = null;
    }
  }
}

// On component destroy
onDestroy(() => {
  if (clickOutsideHandler) {
    document.removeEventListener('click', clickOutsideHandler);
  }
});
```

## Related Issues
- None