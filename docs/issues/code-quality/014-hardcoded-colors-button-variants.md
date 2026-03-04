# Hardcoded colors in Button variants

## Status
✅ **RESOLVED** (2026-03-04)

## Metadata
- **Source Review**: docs/reviews/Code Reviewer/2026-03-04-153045-IconButton.svelte-review.md
- **Location**: src-ui/src/lib/components/shared/IconButton.module.css:238,266
- **Priority**: Medium
- **Category**: Code Quality

## Description
Warning and danger solid variants used hardcoded hex values for hover states instead of design system tokens.

## Resolution
Added dark color variants to the design system and updated IconButton to use them:

```css
/* Added to design-system.css */
--color-warning-dark: #d97706;
--color-error-dark: #dc2626;

/* Updated IconButton.module.css */
.warning.solid::before {
	background: var(--color-warning-dark);
}

.danger.solid::before {
	background: var(--color-error-dark);
}
```

## Files Changed
- `src-ui/src/lib/styles/design-system.css`
- `src-ui/src/lib/components/shared/IconButton.module.css`

## References
- Original review: docs/reviews/Code Reviewer/2026-03-04-153045-IconButton.svelte-review.md
