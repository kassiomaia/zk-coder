# Loading state inconsistency in ConfirmDialog

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-confirm-dialog-review.md
- **Location**: ConfirmDialog.svelte:67
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
The loading state prevents both buttons from being clicked, but the visual feedback could be improved.

## Impact
Users may not clearly understand which action is being processed, leading to confusion about the dialog's current state.

## Suggested Fix
Consider adding loading spinner to cancel button as well, or provide clearer visual feedback for disabled state.

```typescript
// Example fix for ConfirmDialog.svelte
<script>
  let isConfirming = false;
  let isCancelling = false;

  const handleConfirm = async () => {
    isConfirming = true;
    try {
      await onConfirm();
    } finally {
      isConfirming = false;
    }
  };

  const handleCancel = async () => {
    isCancelling = true;
    try {
      await onCancel();
    } finally {
      isCancelling = false;
    }
  };
</script>

<!-- In template -->
<div class="confirm-dialog">
  <div class="content">
    {message}
  </div>

  <div class="actions">
    <button
      class="button button--secondary"
      disabled={isCancelling}
      on:click={handleCancel}
    >
      {#if isCancelling}
        <Spinner size="small" />
        <span>Canceling...</span>
      {:else}
        {cancelText || 'Cancel'}
      {/if}
    </button>

    <button
      class="button button--primary"
      disabled={isConfirming}
      on:click={handleConfirm}
    >
      {#if isConfirming}
        <Spinner size="small" />
        <span>Confirming...</span>
      {:else}
        {confirmText || 'Confirm'}
      {/if}
    </button>
  </div>
</div>

// CSS for better loading state
<style>
  .button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    position: relative;
  }

  .button:disabled .spinner {
    margin-right: 8px;
  }
</style>
```

## Related Components
- ConfirmDialog.svelte
- Spinner component
- Button component

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-confirm-dialog-review.md