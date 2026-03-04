# Magic numbers in ChatArea CSS

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md
- **Location**: src-ui/src/lib/components/chat/ChatArea.module.css:44,353,363,419
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
Values like `width: 1400px` should be defined as CSS variables for consistency and maintainability.

## Impact
Hardcoded pixel values make it difficult to implement responsive design and maintain consistent spacing across the application. Changes require multiple file updates and can lead to inconsistent UI.

## Suggested Fix
Add to design system with variable like `--chat-max-width: 1400px`.

```css
/* In design-system.css */
:root {
  --chat-max-width: 1400px;
  --chat-padding: 24px;
}

/* In ChatArea.module.css */
.chat-area {
  max-width: var(--chat-max-width);
  padding: var(--chat-padding);
}
```

## Related Components
- src-ui/src/lib/components/chat/ChatArea.module.css
- src-ui/src/lib/styles/design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md