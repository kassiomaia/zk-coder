# Mobile-Specific Concerns in ThreadView

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ThreadView-review.md
- **Location**: ThreadView.module.css:224
- **Priority**: Low
- **Category**: Design & Architecture
- **Status**: Open

## Description
The mobile CSS (max-width: 768px) assumes a specific breakpoint that might not align with the application's responsive design system. The ThreadView component hardcodes a mobile breakpoint of `768px`, which may not match the design system's actual mobile breakpoint definitions. This creates a disconnect between the component's responsive behavior and the overall application design system.

## Impact
- Inconsistent mobile experience across components
- Potential layout issues at actual design system breakpoints
- Hard to maintain responsive design consistency
- May cause UI to break at unexpected screen sizes
- Violates the principle of using design system tokens

## Suggested Fix
Use a named breakpoint constant from the design system instead of hard-coding the value.

```css
/* In design-system.css */
:root {
  /* Responsive breakpoints */
  --breakpoint-xs: 0px;
  --breakpoint-sm: 640px;
  --breakpoint-md: 768px;
  --breakpoint-lg: 1024px;
  --breakpoint-xl: 1280px;
}

/* In ThreadView.module.css */
/* Use CSS variable instead of hardcoded value */
@media (max-width: var(--breakpoint-md)) {
  .thread-view {
    padding: 12px;
  }

  .thread-header {
    flex-direction: column;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .thread-actions {
    width: 100%;
    justify-content: flex-end;
    margin-top: 8px;
  }
}

/* Or use clamp() for more fluid layouts */
.thread-view {
  padding: clamp(8px, 3vw, 16px);
}
```

## Related Components
- src-ui/src/lib/components/chat/ThreadView.svelte
- src-ui/src/lib/components/chat/ThreadView.module.css
- src-ui/src/lib/styles/design-system.css

## References
- Original review: docs/reviews/code-reviewer/ThreadView-review.md