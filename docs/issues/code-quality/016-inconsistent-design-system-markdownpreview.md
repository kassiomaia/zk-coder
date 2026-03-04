# Inconsistent design system usage in MarkdownPreview

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-MarkdownPreview.md
- **Location**: src-ui/src/lib/components/chat/input/MarkdownPreview.module.css:81
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The component uses some CSS variables that don't exist in the design system (color-accent-rose), and the styling approach doesn't fully leverage the design system's capabilities.

## Impact
Using undefined CSS variables creates maintenance issues and potential runtime errors. It also indicates that the component isn't fully utilizing the design system, leading to inconsistencies in the UI and potential visual clashes.

## Suggested Fix
Replace color-accent-rose with appropriate design system tokens or add it to the design system if intentional.

```css
/* Before */
.markdown-preview {
  border-color: var(--color-accent-rose);
}

/* After */
/* Option 1: Use existing token */
.markdown-preview {
  border-color: var(--color-red-200);
}

/* Option 2: Add to design system if needed */
/* In design-system.css */
:root {
  --color-accent-rose: #fecaca;
}

/* In component */
.markdown-preview {
  border-color: var(--color-accent-rose);
}
```

## Related Components
- MarkdownPreview.svelte
- MarkdownPreview.module.css
- design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-MarkdownPreview.md