# Inconsistent Dark Mode Implementation in AnsweringIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/AnsweringIndicator.module.css:112-126
- **Priority**: Medium
- **Category**: Design & Architecture
- **Status**: Open

## Description
The dark mode styles only adjust the accent color but don't ensure proper contrast ratios for the text and icons. The current dark mode implementation only changes the primary color but doesn't adjust other color properties (text color, background, icon colors) to maintain proper accessibility and readability in dark themes. This can result in poor contrast ratios and an inconsistent dark mode experience.

## Impact
- Poor accessibility in dark mode
- Inconsistent visual appearance between light and dark themes
- Eye strain for users with dark mode enabled
- Violation of WCAG accessibility guidelines
- Inconsistent with the design system's dark mode specifications

## Suggested Fix
Ensure all color properties respect the dark theme by using appropriate design system tokens.

```css
/* Dark mode styles using design system tokens */
@media (prefers-color-scheme: dark) {
  .answering-indicator {
    background: var(--color-background-dark);
    color: var(--color-text-dark);
    border-color: var(--color-border-dark);
  }

  .answering-indicator .icon {
    color: var(--color-text-dark-secondary);
  }

  .answering-indicator .pulse {
    background: var(--color-primary-dark);
    opacity: 0.8;
  }

  .answering-indicator .typewriter {
    border-color: var(--color-border-dark);
  }
}

/* Or use CSS variables with fallbacks */
.answering-indicator {
  background: var(--color-background, white);
  color: var(--color-text, #333);
  border-color: var(--color-border, #ddd);
}

@media (prefers-color-scheme: dark) {
  :root {
    --color-background: #1a1a1a;
    --color-text: #e0e0e0;
    --color-border: #444;
    --color-primary: #0066ff;
  }
}
```

## Related Components
- src-ui/src/lib/components/chat/AnsweringIndicator.svelte
- src-ui/src/lib/components/chat/AnsweringIndicator.module.css
- src-ui/src/lib/styles/design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md