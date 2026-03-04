# Inconsistent Responsive Breakpoints in ChatArea

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md
- **Location**: src-ui/src/lib/components/chat/ChatArea.module.css:724-764
- **Priority**: Low
- **Category**: Design & Architecture
- **Status**: Open

## Description
Media queries use pixel values that don't align with the design system's breakpoint tokens. The ChatArea component uses hardcoded pixel values like `768px`, `992px`, and `1200px` in its media queries, while the design system likely defines named breakpoints (like `--breakpoint-sm`, `--breakpoint-md`, etc.). This inconsistency makes it difficult to maintain responsive design across the application and can lead to layout issues when the design system breakpoints change.

## Impact
- Inconsistent responsive behavior across components
- Difficult to maintain responsive design
- Risk of layout breaking if design system changes
- Inconsistent user experience across different screen sizes
- Violates DRY principle by duplicating breakpoint values

## Suggested Fix
Use design system breakpoints defined as CSS variables.

```css
/* In design-system.css */
:root {
  /* Responsive breakpoints */
  --breakpoint-xs: 0px;
  --breakpoint-sm: 640px;
  --breakpoint-md: 768px;
  --breakpoint-lg: 1024px;
  --breakpoint-xl: 1280px;
  --breakpoint-2xl: 1536px;
}

/* In ChatArea.module.css */
/* Use CSS variables instead of hardcoded values */
@media (max-width: var(--breakpoint-md)) {
  .chat-area {
    flex-direction: column;
    padding: 12px;
  }

  .messages-container {
    max-width: 100%;
    margin-right: 0;
    margin-bottom: 16px;
  }
}

@media (max-width: var(--breakpoint-sm)) {
  .chat-area {
    padding: 8px;
  }

  .message-bubble {
    max-width: 85%;
    font-size: 14px;
  }
}

/* Or use clamp() for more fluid layouts */
.chat-area {
  padding: clamp(8px, 2vw, 16px);
}
```

## Related Components
- src-ui/src/lib/components/chat/ChatArea.svelte
- src-ui/src/lib/components/chat/ChatArea.module.css
- src-ui/src/lib/styles/design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md