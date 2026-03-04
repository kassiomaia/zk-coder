# Loading state accessibility in IconButton

## Metadata
- **Source Review**: docs/reviews/Code Reviewer/2026-03-04-153045-IconButton.svelte-review.md
- **Location**: src-ui/src/lib/components/shared/IconButton.svelte:118
- **Priority**: Medium
- **Category**: Accessibility
- **Status**: Open

## Description
When `loading` is true, the button has `aria-busy="true"` but the spinner might not be programmatically announced to screen readers in all browsers.

## Impact
Screen reader users may not be aware that the button is in a loading state, leading to confusion about whether their action was successful or if the interface is frozen.

## Suggested Fix
Add `aria-live="polite"` to the button when loading, and ensure the spinner has appropriate ARIA attributes.

```typescript
<!-- IconButton.svelte -->
<button
  class:loading={loading}
  aria-busy={loading ? 'true' : undefined}
  aria-live={loading ? 'polite' : undefined}
  aria-label={loading ? `Loading ${label}` : label}
>
  {#if loading}
    <span class="spinner" aria-hidden="true" />
  {:else}
    <span class="icon" aria-hidden="true" />
  {/if}
  <span class="label">{loading ? 'Loading...' : label}</span>
</button>

/* Add spinner animations with appropriate contrast */
.spinner {
  animation: spin 1s linear infinite;
  /* Ensure spinner is visible */
  border-color: var(--color-primary-600);
  border-top-color: transparent;
}
```

## Related Components
- IconButton.svelte
- Loading state components

## References
- Original review: docs/reviews/Code Reviewer/2026-03-04-153045-IconButton.svelte-review.md
- ARIA busy state: https://www.w3.org/WAI/ARIA/apg/patterns/button/#button-type-loading