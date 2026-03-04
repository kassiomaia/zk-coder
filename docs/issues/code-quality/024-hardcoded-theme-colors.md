# Hardcoded theme color values

## Status
✅ **RESOLVED** (2026-03-04)

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_layout-review.md
- **Location**: src-ui/src/routes/+layout.svelte:50
- **Priority**: Low
- **Category**: Code Quality

## Description
The theme color in the meta tag was hardcoded to specific hex values rather than using CSS variables from the design system.

## Resolution
Created a `THEME_COLORS` constant that references design system color values:

```typescript
/**
 * Theme colors from the design system.
 * These match the CSS variables in design-system.css.
 */
const THEME_COLORS = {
	dark: '#09090b', // --color-neutral-950
	light: '#fafafa' // --color-neutral-50
} as const;
```

Updated the meta tag to use the constant:

```svelte
<meta name="theme-color" content={THEME_COLORS[theme]} />
```

## Files Changed
- `src-ui/src/routes/+layout.svelte`

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_layout-review.md
