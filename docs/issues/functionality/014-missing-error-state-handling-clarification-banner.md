# Missing error state handling in ClarificationBanner

## Metadata
- **Source Review**: docs/reviews/code-reviewer/clarification-banner-review-2026-03-04.md
- **Location**: ClarificationBanner.svelte:44,48,50
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
There's no error state handling when the onResolve callback fails.

## Impact
If the resolution callback fails, users will be left with no feedback about what happened, potentially leading to confusion about whether their action was successful.

## Suggested Fix
Add error state management to display feedback if resolution fails.

```typescript
// Example fix for ClarificationBanner.svelte
<script>
  let resolutionError = null;
  let isResolving = false;

  const handleOptionSelect = (option: AmbiguityOption) => {
    resolveAmbiguity(option, ambiguity.id);
  };

  const handleCustomSubmit = () => {
    if (!customInput.trim()) {
      resolutionError = 'Please enter a valid response';
      return;
    }

    resolveAmbiguity(
      { type: 'custom', value: customInput.trim() },
      ambiguity.id
    );
  };

  const resolveAmbiguity = async (option: AmbiguityOption, id: string) => {
    isResolving = true;
    resolutionError = null;

    try {
      await onResolve(option, id);
      // Success - banner will close or update
    } catch (error) {
      console.error('Failed to resolve ambiguity:', error);
      resolutionError = error.message || 'Failed to process your response. Please try again.';

      // Show error in banner
      showNotification({
        type: 'error',
        message: resolutionError
      });
    } finally {
      isResolving = false;
    }
  };
</script>

<!-- In template -->
{#if resolutionError}
  <div class="error-message" role="alert">
    <Icon name="error" />
    <span>{resolutionError}</span>
    <button
      class="retry-button"
      on:click={() => resolutionError = null}
      disabled={isResolving}
    >
      Try Again
    </button>
  </div>
{/if}

<!-- Loading state -->
{#if isResolving}
  <div class="loading-state">
    <Spinner size="small" />
    <span>Processing...</span>
  </div>
{/if}
```

## Related Components
- ClarificationBanner.svelte
- Notification system
- Spinner component

## References
- Original review: docs/reviews/code-reviewer/clarification-banner-review-2026-03-04.md