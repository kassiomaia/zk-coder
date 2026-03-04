# Hardcoded colors in Button variants

## Metadata
- **Source Review**: docs/reviews/Code Reviewer/2026-03-04-153045-IconButton.svelte-review.md
- **Location**: src-ui/src/lib/components/shared/IconButton.module.css:238-266
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
Warning and danger variants use hardcoded color values instead of CSS variables, reducing flexibility and breaking the design system consistency.

## Impact
Hardcoded button variant colors prevent consistent theming and make it difficult to adjust colors for different contexts or accessibility needs. This creates inconsistencies in the UI and breaks the unified design language.

## Suggested Fix
Define these colors in CSS variables in the design system and reference them.

```css
/* Before */
.warning {
  color: #f59e0b;
  background-color: #fbbf24;
}

.danger {
  color: #ef4444;
  background-color: #fca5a5;
}

/* After */
/* Define in design-system.css */
:root {
  --color-warning-500: #f59e0b;
  --color-warning-200: #fbbf24;
  --color-danger-500: #ef4444;
  --color-danger-200: #fca5a5;
}

/* In component */
.warning {
  color: var(--color-warning-500);
  background-color: var(--color-warning-200);
}

.danger {
  color: var(--color-danger-500);
  background-color: var(--color-danger-200);
}
```

## Related Components
- IconButton.svelte
- IconButton.module.css
- design-system.css

## References
- Original review: docs/reviews/Code Reviewer/2026-03-04-153045-IconButton.svelte-review.md