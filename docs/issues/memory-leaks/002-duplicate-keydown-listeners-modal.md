# [ISSUE-002] - Duplicate Keydown Listeners in Modal

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/Modal.svelte_2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/Modal.svelte`
- **Line Numbers**: TBD

## Description
The Modal component adds multiple keydown event listeners without properly checking for existing listeners. This results in duplicate listeners being added, causing event handler execution multiple times and potential memory leaks.

## Impact
Memory Leaks - This causes:
- Duplicate event handler execution
- Unnecessary memory usage
- Unexpected behavior when handling keyboard events
- Performance degradation

## Suggested Fix
Implement single event listener management:

```typescript
let keydownHandler: (event: KeyboardEvent) => void;

function createKeydownHandler() {
  return (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      onClose();
    }
  };
}

$: {
  if (isOpen) {
    // Remove existing handler if any
    if (keydownHandler) {
      document.removeEventListener('keydown', keydownHandler);
    }

    // Create and add new handler
    keydownHandler = createKeydownHandler();
    document.addEventListener('keydown', keydownHandler);
  }
}

onDestroy(() => {
  if (keydownHandler) {
    document.removeEventListener('keydown', keydownHandler);
  }
});
```

## Related Issues
- None