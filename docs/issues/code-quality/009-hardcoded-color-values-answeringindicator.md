# Hardcoded color values in AnsweringIndicator

## Status
✅ **RESOLVED** (2026-03-04)

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/AnsweringIndicator.module.css:39
- **Priority**: Medium
- **Category**: Code Quality

## Description
The accent color was hardcoded in the CSS using RGB values (99, 102, 241) instead of using the design system tokens.

## Resolution
Replaced hardcoded RGB values with design system CSS variables:

```css
/* Before */
--accent-color: 99, 102, 241; /* Base Indigo RGB */

/* After */
/* Removed custom accent-color variable */
/* Icon color now uses: */
color: var(--color-accent-primary);
```

The component now uses the design system's `--color-accent-primary` variable which automatically adapts to light/dark mode.

## Files Changed
- `src-ui/src/lib/components/chat/AnsweringIndicator.module.css`
- `src-ui/src/lib/components/chat/AnsweringIndicator.svelte`

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
