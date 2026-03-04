# Hardcoded animation durations in TypingIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/TypingIndicator-review.md
- **Location**: TypingIndicator.module.css:54,59-67,98
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
The animation delays and durations are hardcoded in the CSS, which makes it harder to customize or adjust based on user preferences.

## Impact
Hardcoded animation values reduce flexibility and make it difficult to adjust animation speeds for different contexts or accessibility needs. It also prevents users from customizing their experience based on personal preferences.

## Suggested Fix
Define animation timings as CSS variables for better maintainability.

```css
/* Before */
.dot {
  animation: typing 1.4s infinite;
}

.dot:nth-child(2) {
  animation-delay: 0.2s;
}

.dot:nth-child(3) {
  animation-delay: 0.4s;
}

/* After */
:root {
  --typing-duration: 1.4s;
  --typing-delay-step: 0.2s;
}

.dot {
  animation: typing var(--typing-duration) infinite;
}

.dot:nth-child(2) {
  animation-delay: var(--typing-delay-step);
}

.dot:nth-child(3) {
  animation-delay: calc(var(--typing-delay-step) * 2);
}
```

## Related Components
- TypingIndicator.svelte
- TypingIndicator.module.css
- design-system.css

## References
- Original review: docs/reviews/code-reviewer/TypingIndicator-review.md