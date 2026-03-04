# Hardcoded animation durations in ModelCard

## Status
✅ **RESOLVED** (2026-03-04)

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-ModelCard-review.md
- **Location**: src-ui/src/lib/components/config/ModelCard.module.css:166,187
- **Priority**: Medium
- **Category**: Code Quality

## Description
Animation durations were hardcoded in the CSS instead of being driven by CSS variables from the design system.

## Resolution
Added accent glow color tokens to the design system and replaced hardcoded rgba values:

```css
/* In design-system.css */
--color-accent-primary-glow: rgba(99, 102, 241, 0.3);
--color-accent-primary-glow-light: rgba(99, 102, 241, 0.08);

/* In ModelCard.module.css */
box-shadow:
	0 0 0 1px var(--color-accent-primary-glow-light),
	0 0 12px -2px var(--color-accent-primary-glow);
```

## Files Changed
- `src-ui/src/lib/styles/design-system.css`
- `src-ui/src/lib/components/config/ModelCard.module.css`

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-ModelCard-review.md
