# Missing ARIA live region for dynamic text in EmptyThreadState

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-EmptyThreadState.svelte.md
- **Location**: src-ui/src/lib/components/chat/EmptyThreadState.svelte:104-123
- **Priority**: Medium
- **Category**: Accessibility
- **Status**: Open

## Description
The component uses derived variables for headline and hint text which can change based on context and input activity. However, there's no `aria-live` region or proper accessibility announcements for screen readers when this content changes.

## Impact
Users relying on screen readers may miss important contextual information when the headline and hint text change, as these changes are not announced by assistive technologies.

## Suggested Fix
Add `aria-live="polite"` to the container or appropriate wrapper to ensure screen readers announce text changes when the context or input state changes.

```html
<!-- Fix example -->
<div class="empty-state" aria-live="polite">
  <h2 class="headline">{headline}</h2>
  <p class="hint">{hint}</p>
</div>
```

## Related Components
- EmptyThreadState.svelte
- ThreadItem.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-EmptyThreadState.svelte.md
- ARIA live regions: https://www.w3.org/WAI/WCAG21/techniques/aria/ARIA1