# Warning variant logic inconsistency in ConfirmDialog

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-confirm-dialog-review.md
- **Location**: ConfirmDialog.svelte:67
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
When variant is 'warning', the confirm button shows as 'primary', but warning actions typically need visual distinction.

## Impact
Warning actions may not be visually distinct enough, potentially causing users to accidentally proceed with destructive actions.

## Suggested Fix
Consider a dedicated 'warning' variant for the confirm button or document the current behavior clearly.

```typescript
// Example fix for ConfirmDialog.svelte
<script>
  // Button class logic with warning variant
  const getConfirmButtonClass = () => {
    if (variant === 'warning') {
      return 'button button--warning button--destructive';
    }
    if (variant === 'danger') {
      return 'button button--danger';
    }
    return 'button button--primary';
  };
</script>

<!-- In template -->
<button
  class={getConfirmButtonClass()}
  disabled={isLoading}
  on:click={onConfirm}
>
  {#if isLoading}
    <Spinner size="small" />
    <span>Processing...</span>
  {:else}
    {confirmText || 'Confirm'}
  {/if}
</button>

// CSS for warning variant
<style>
  .button--warning {
    background-color: var(--color-warning);
    color: white;
  }

  .button--warning.button--destructive {
    background-color: var(--color-error);
    border-color: var(--color-error);
  }
</style>
```

## Related Components
- ConfirmDialog.svelte
- Button component
- Design system (colors and variants)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-confirm-dialog-review.md