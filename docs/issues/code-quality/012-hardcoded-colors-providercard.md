# Hardcoded color values in ProviderCard

## Status
✅ **RESOLVED** (2026-03-04)

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ProviderCard.svelte-2026-03-04.md
- **Location**: src-ui/src/lib/components/config/ProviderCard.module.css:31,32,36,37
- **Priority**: Medium
- **Category**: Code Quality

## Description
The CSS module used hardcoded RGB values for the configured state instead of CSS variables from the design system.

## Resolution
Replaced hardcoded RGB values with design system CSS variables:

```css
/* Before */
.card.configured {
	border-color: rgba(16, 185, 129, 0.2);
	background: rgba(16, 185, 129, 0.03);
}

.card.configured:hover {
	border-color: rgba(16, 185, 129, 0.3);
	background: rgba(16, 185, 129, 0.05);
}

/* After */
.card.configured {
	border-color: var(--color-success-border);
	background: var(--color-success-subtle);
}

.card.configured:hover {
	border-color: var(--color-success);
	background: var(--color-success-light);
}
```

## Files Changed
- `src-ui/src/lib/styles/design-system.css`
- `src-ui/src/lib/components/config/ProviderCard.module.css`

## References
- Original review: docs/reviews/code-reviewer/ProviderCard.svelte-2026-03-04.md
