# Hardcoded color values in IntentIndicator

## Status
✅ **RESOLVED** (2026-03-04)

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-IntentIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/input/IntentIndicator.module.css:29,35
- **Priority**: Medium
- **Category**: Code Quality

## Description
The CSS contains hardcoded color values in rgba format that should use CSS variables for consistency and maintainability.

## Resolution
Added semantic border color tokens to the design system and updated IntentIndicator to use them:

```css
/* In design-system.css */
--color-success-border: rgba(16, 185, 129, 0.2);
--color-warning-border: rgba(245, 158, 11, 0.2);
--color-error-border: rgba(239, 68, 68, 0.2);
--color-info-border: rgba(14, 165, 233, 0.2);
```

Updated component to use new tokens:

```css
/* Before */
border-color: rgba(16, 185, 129, 0.2);

/* After */
border-color: var(--color-success-border);
```

## Files Changed
- `src-ui/src/lib/styles/design-system.css`
- `src-ui/src/lib/components/chat/input/IntentIndicator.module.css`

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-IntentIndicator.svelte.md
