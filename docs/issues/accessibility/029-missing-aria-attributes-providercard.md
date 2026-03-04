# Missing ARIA attributes in ProviderCard

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ProviderCard.svelte-2026-03-04.md
- **Location**: ProviderCard.svelte:24
- **Priority**: Medium
- **Category**: Accessibility
- **Status**: Open

## Description
The button lacks comprehensive ARIA attributes for better accessibility, especially for screen readers.

## Impact
Without proper ARIA attributes, screen reader users may not understand the purpose or state of the provider card button, making it difficult to navigate and select providers effectively.

## Suggested Fix
Enhance accessibility with proper ARIA attributes like aria-label and aria-describedby.

```typescript
<!-- ProviderCard.svelte -->
<button
  class="provider-card"
  tabindex="0"
  role="button"
  aria-label="Select provider: {provider.name}"
  aria-describedby={`provider-desc-${provider.id}`}
  aria-pressed={isSelected}
  on:click={() => onSelect(provider)}
  on:keydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onSelect(provider);
    }
  }}
>
  <div class="icon">
    {#if provider.icon}
      <img src={provider.icon} alt={provider.name} />
    {:else}
      <span class="default-icon">{provider.name.charAt(0)}</span>
    {/if}
  </div>

  <div class="info">
    <h3 id={`provider-name-${provider.id}`}>{provider.name}</h3>
    <p id={`provider-desc-${provider.id}`}>
      {provider.description || 'Provider configuration'}
    </p>
  </div>

  {#if isSelected}
    <div class="checkmark" role="status" aria-label="Selected">
      <svg>...</svg>
    </div>
  {/if}
</button>
```

## Related Components
- ProviderCard.svelte
- Provider selection components
- Modal components containing provider cards

## References
- Original review: docs/reviews/code-reviewer/ProviderCard.svelte-2026-03-04.md
- ARIA Button Pattern: https://www.w3.org/WAI/ARIA/apg/patterns/button/
- WAI-ARIA Authoring Practices: https://www.w3.org/WAI/ARIA/apg/