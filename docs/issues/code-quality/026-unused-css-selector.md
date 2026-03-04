# Unused CSS selector in AnsweringIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/AnsweringIndicator.module.css:129
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
The `.highlightSweep` selector is referenced in the reduced motion media query but doesn't exist in the component.

## Impact
Dead CSS code increases file size and can cause confusion during maintenance. It may also lead to unintended side effects if the selector is mistakenly referenced elsewhere.

## Suggested Fix
Remove the non-existent selector reference.

```css
/* Remove this line */
@media (prefers-reduced-motion: reduce) {
  .highlightSweep {
    animation: none;
  }
}
```

## Related Components
- src-ui/src/lib/components/chat/AnsweringIndicator.module.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md