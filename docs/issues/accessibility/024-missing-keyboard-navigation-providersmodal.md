# Missing keyboard navigation in ProvidersModal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_ProvidersModal.svelte.md
- **Location**: src-ui/src/lib/components/config/ProvidersModal.svelte:54,59
- **Priority**: Low
- **Category**: Accessibility
- **Status**: Open

## Description
The modal lacks keyboard navigation for accessibility, particularly for the provider cards.

## Impact
Without proper keyboard navigation, keyboard users cannot effectively interact with provider cards in the modal, limiting their ability to select different providers and potentially excluding them from core functionality.

## Suggested Fix
Add tabIndex and keyboard event handlers for accessibility.

```typescript
<!-- Provider cards should be focusable -->
<div
  class="provider-card"
  tabindex="0"
  role="button"
  aria-label="Select provider: {provider.name}"
  on:keydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleProviderSelect(provider);
    }
  }}
  on:click={() => handleProviderSelect(provider)}
>
  <!-- Provider card content -->
</div>

<!-- Modal should trap focus when open>
<script>
  import { focusTrap } from 'svelte-focus-trap';

  const { active } = $props();

  // Focus trapping logic
  $: if (active) {
    focusTrap(modalElement);
  }
</script>
```

## Related Components
- ProvidersModal.svelte
- ProviderCard components
- Modal components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_ProvidersModal.svelte.md
- ARIA Authoring Practices: https://www.w3.org/WAI/ARIA/apg/patterns/dialog/

## Resolution Status
**RESOLVED** - False Positive - 2026-03-04

The ProvidersModal and ProviderCard components already have proper keyboard navigation:

**ProviderCard.svelte:**
- Uses native `<button>` element which is keyboard accessible by default
- Has `aria-label="Configure {provider.name}"` for screen readers
- `type="button"` for proper form behavior

**ProvidersModal.svelte:**
- Uses Modal component which handles Escape key
- Provider cards are buttons, so Tab navigation works
- Add Custom Provider card is also a button

The implementation follows WCAG 2.1 guidelines for keyboard accessibility.