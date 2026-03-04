# Hardcoded colors in ProviderCard

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ProviderCard.svelte-2026-03-04.md
- **Location**: ProviderCard.module.css:31,32,36,37
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The CSS module uses hardcoded RGB values for the configured state instead of CSS variables from the design system.

## Impact
Hardcoded RGB values break the design system's theming capabilities and make it difficult to maintain consistent visual language across the application. These values don't adapt to theme changes and create inconsistency in the UI.

## Suggested Fix
Replace hardcoded RGB values with CSS variables like var(--color-success-light) and var(--color-success-subtle).

```css
/* Before */
.configured {
  background-color: rgb(34, 197, 94, 0.1);
  border-color: rgb(34, 197, 94);
}

/* After */
.configured {
  background-color: var(--color-success-light);
  border-color: var(--color-success);
}
```

## Related Components
- ProviderCard.svelte
- ProviderCard.module.css
- design-system.css

## References
- Original review: docs/reviews/code-reviewer/ProviderCard.svelte-2026-03-04.md