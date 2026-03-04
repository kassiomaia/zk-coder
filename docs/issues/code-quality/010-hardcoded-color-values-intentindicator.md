# Hardcoded color values in IntentIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-IntentIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/input/IntentIndicator.module.css:29,35
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The CSS contains hardcoded color values in rgba format that should use CSS variables for consistency and maintainability.

## Impact
Hardcoded color values prevent the application from adopting consistent theming and make it difficult to adjust colors for different themes or accessibility needs. These values also break the design system's color consistency across components.

## Suggested Fix
Replace hardcoded rgba values with appropriate CSS variables from the design system.

```css
/* Before */
.intent-indicator {
  background: rgba(99, 102, 241, 0.1);
  border: 1px solid rgba(99, 102, 241, 0.3);
}

/* After */
.intent-indicator {
  background: var(--color-primary-100);
  border: 1px solid var(--color-primary-200);
}
```

## Related Components
- IntentIndicator.svelte
- IntentIndicator.module.css
- design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-IntentIndicator.svelte.md