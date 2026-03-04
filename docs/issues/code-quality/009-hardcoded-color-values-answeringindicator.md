# Hardcoded color values in AnsweringIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/AnsweringIndicator.module.css:39
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The accent color is hardcoded in the CSS using RGB values (99, 102, 241) instead of using the design system tokens.

## Impact
Using hardcoded RGB values breaks the design system consistency and makes it difficult to maintain theming and accessibility. It prevents global color changes from affecting this component and creates visual inconsistencies across the application.

## Suggested Fix
Replace hardcoded RGB values with CSS variables from the design system (e.g., `var(--color-primary-400)`).

```css
/* Before */
.accent {
  color: rgb(99, 102, 241);
}

/* After */
.accent {
  color: var(--color-primary-400);
}
```

## Related Components
- AnsweringIndicator.svelte
- AnsweringIndicator.module.css
- design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md