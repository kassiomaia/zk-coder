# Missing Esc key handler in ConfirmDialog

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-confirm-dialog-review.md
- **Location**: ConfirmDialog.svelte:57
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
The dialog should support closing with the Escape key for better UX.

## Impact
Users expect to be able to cancel dialogs using the Escape key, which is a standard UX pattern. Without this, users may be forced to use the mouse, which can be inconvenient.

## Suggested Fix
Verify if Modal handles Esc key. If not, implement it.

```typescript
// Example fix for ConfirmDialog.svelte
<script>
  import { onMount } from 'svelte';

  let modalElement;

  onMount(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onCancel();
      }
    };

    modalElement.addEventListener('keydown', handleKeyDown);

    return () => {
      modalElement.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

<!-- In template, ensure the dialog has proper focus -->
<div
  bind:this={modalElement}
  class="confirm-dialog"
  tabindex="0"
  role="dialog"
  aria-modal="true"
  aria-labelledby="dialog-title"
>
  <!-- Dialog content -->
</div>

// Alternative using svelte:action (if using Svelte 5)
<script>
  import { useKeydown } from 'svelte/actions';

  let dialog;

  useKeydown(dialog, {
    Escape: (event) => {
      event.preventDefault();
      onCancel();
    }
  });
</script>

<div bind:this={dialog} class="confirm-dialog" tabindex="0">
  <!-- Dialog content -->
</div>
```

## Related Components
- ConfirmDialog.svelte
- Modal container component
- Keyboard event handling system

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-confirm-dialog-review.md