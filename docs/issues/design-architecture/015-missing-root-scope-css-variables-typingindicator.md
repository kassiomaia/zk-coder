# Missing :root Scope for CSS Variables in TypingIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/TypingIndicator-review.md
- **Location**: TypingIndicator.module.css:20,52,86,87, design-system.css:395,424,442,460
- **Priority**: Medium
- **Category**: Design & Architecture
- **Status**: Open

## Description
CSS variables are used but defined within scoped theme selectors. The TypingIndicator component uses CSS variables (like `--color-primary`, `--color-secondary`) in its styles but these variables are defined within theme-specific selectors rather than in the global `:root` scope. This can cause issues if the theme is not properly applied before the component is rendered, leading to undefined variable errors and incorrect styling.

## Impact
- Styling can break if theme is not properly loaded
- Fallback values may be used instead of intended theme colors
- CSS cascade issues with variable inheritance
- Potential FOUC (Flash of Unstyled Content)
- Inconsistent theming across components

## Suggested Fix
Ensure all CSS variables used are defined in the global `:root` scope.

```css
/* In design-system.css or global.css */
:root {
  /* Primary colors */
  --color-primary: #0066ff;
  --color-primary-dark: #0052cc;
  --color-primary-light: #4d94ff;

  /* Secondary colors */
  --color-secondary: #6c757d;
  --color-secondary-dark: #5a6268;
  --color-secondary-light: #8a92a6;

  /* Component-specific variables */
  --typing-indicator-width: 200px;
  --typing-indicator-height: 40px;
}

/* In TypingIndicator.module.css */
.typing-indicator {
  width: var(--typing-indicator-width);
  height: var(--typing-indicator-height);
  background: var(--color-background);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 8px 16px;
  display: flex;
  align-items: center;
}

.dots {
  display: flex;
  gap: 4px;
}

.dot {
  width: 8px;
  height: 8px;
  background: var(--color-primary);
  border-radius: 50%;
  animation: pulse 1.4s infinite;
}
```

## Related Components
- src-ui/src/lib/components/chat/TypingIndicator.svelte
- src-ui/src/lib/components/chat/TypingIndicator.module.css
- src-ui/src/lib/styles/design-system.css

## References
- Original review: docs/reviews/code-reviewer/TypingIndicator-review.md