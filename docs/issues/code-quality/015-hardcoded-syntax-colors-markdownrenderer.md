# Hardcoded syntax highlighting colors in MarkdownRenderer

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_MarkdownRenderer.svelte.md
- **Location**: src-ui/src/lib/components/markdown/MarkdownRenderer.module.css:184-220
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The syntax highlighting colors are hardcoded with specific hex values instead of using CSS variables from the design system.

## Impact
Hardcoded syntax highlighting colors prevent consistent theming and make it difficult to adjust colors for different themes or accessibility needs. These colors also don't adapt to user preferences or accessibility requirements like high contrast mode.

## Suggested Fix
Define syntax highlighting colors in the design system and reference them via CSS variables.

```css
/* Before */
.code-block pre {
  color: #e5e7eb;
  background-color: #1f2937;
}

.token.comment { color: #6b7280; }
.token.keyword { color: #8b5cf6; }
.token.string { color: #10b981; }

/* After */
/* Define in design-system.css */
:root {
  --syntax-bg: #1f2937;
  --syntax-text: #e5e7eb;
  --syntax-comment: #6b7280;
  --syntax-keyword: #8b5cf6;
  --syntax-string: #10b981;
}

/* In component */
.code-block pre {
  color: var(--syntax-text);
  background-color: var(--syntax-bg);
}

.token.comment { color: var(--syntax-comment); }
.token.keyword { color: var(--syntax-keyword); }
.token.string { color: var(--syntax-string); }
```

## Related Components
- MarkdownRenderer.svelte
- MarkdownRenderer.module.css
- design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_MarkdownRenderer.svelte.md