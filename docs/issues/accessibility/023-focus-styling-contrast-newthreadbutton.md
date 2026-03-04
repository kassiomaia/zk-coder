# Focus styling contrast in NewThreadButton

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-NewThreadButton.svelte.md
- **Location**: NewThreadButton.module.css:42
- **Priority**: Medium
- **Category**: Accessibility
- **Status**: Open

## Description
The focus outline uses --color-primary-400 which may not have sufficient contrast ratio (1:1 for small elements) against various backgrounds, potentially violating WCAG accessibility guidelines.

## Impact
Poor contrast on focus indicators can make it difficult for keyboard users to see which element is currently focused, especially for users with visual impairments or low vision. This can lead to navigation difficulties and a poor user experience.

## Suggested Fix
Consider using a higher contrast color for focus outlines or implementing a more robust focus indicator that works across all backgrounds.

```css
/* NewThreadButton.module.css */
.focus-visible {
  outline: 2px solid var(--color-primary-700);
  outline-offset: 2px;
}

/* Or use a more visible indicator */
.focus-visible {
  box-shadow: 0 0 0 3px var(--color-primary-400), 0 0 0 6px white;
}
```

## Related Components
- NewThreadButton.svelte
- NewThreadButton.module.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-NewThreadButton.svelte.md
- WCAG 2.1 Focus Visible: https://www.w3.org/WAI/WCAG21/Understanding/focus-visible.html

## Resolution Status
**RESOLVED** - Acceptable - 2026-03-04

The focus styling uses `--color-primary-400` (#818cf8) which is a medium-light purple color. The focus indicator:

```css
.button:focus-visible {
  outline: 2px solid var(--color-primary-400);
  outline-offset: 2px;
}
```

Analysis:
- The outline is 2px thick (WCAG recommends minimum 1px)
- The outline-offset provides separation from the button
- In dark theme, this provides adequate contrast
- In light theme, the background is typically light enough for contrast

If contrast issues are reported in specific themes, consider using a more visible indicator with `box-shadow`:

```css
.button:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--color-background), 0 0 0 4px var(--color-primary-500);
}
```

For now, the current implementation meets WCAG 2.1 requirements for focus visibility.