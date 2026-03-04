# Hardcoded theme color values

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_layout-review.md
- **Location**: src-ui/src/routes/+layout.svelte:50
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
The theme color in the meta tag is hardcoded to specific hex values rather than using CSS variables from the design system.

## Impact
Hardcoded theme colors prevent dynamic theming and make it difficult to implement light/dark mode or custom color schemes. It also violates the design system's consistency.

## Suggested Fix
Use CSS variables or import them from the design system.

```javascript
// In +layout.svelte
import { themeColors } from '$lib/styles/design-system';

// Use in template
<meta name="theme-color" content={themeColors.primary}>
```

Or use CSS variables in global styles:

```css
/* In global.css */
:root {
  --theme-color: var(--color-primary);
}

/* In +layout.svelte */
<meta name="theme-color" style="color: var(--theme-color);">
```

## Related Components
- src-ui/src/routes/+layout.svelte
- src-ui/src/lib/styles/design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_layout-review.md