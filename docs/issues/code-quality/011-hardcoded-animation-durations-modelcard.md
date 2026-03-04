# Hardcoded color values in ModelCard

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-ModelCard-review.md
- **Location**: src-ui/src/lib/components/config/ModelCard.module.css:166,187
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
Animation durations are hardcoded in the CSS rather than being driven by CSS variables from the design system.

## Impact
Hardcoded animation values prevent consistent timing across the application and make it difficult to adjust animation speeds for different contexts or accessibility needs. This inconsistency can affect the user experience and perceived performance.

## Suggested Fix
Replace hardcoded values with CSS variables from the design system.

```css
/* Before */
.model-card {
  animation: slideIn 0.3s ease-out;
}

/* After */
:root {
  --animation-duration-fast: 0.3s;
}

.model-card {
  animation: slideIn var(--animation-duration-fast) ease-out;
}
```

## Related Components
- ModelCard.svelte
- ModelCard.module.css
- design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-ModelCard-review.md