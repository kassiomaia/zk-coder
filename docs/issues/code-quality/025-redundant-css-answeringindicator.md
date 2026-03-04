# Redundant CSS classes in AnsweringIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/AnsweringIndicator.module.css:44-48,51-59
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
The CSS has several redundant class definitions (e.g., multiple `.panel` state declarations with identical properties).

## Impact
Redundant CSS code increases file size, makes maintenance more difficult, and can cause performance issues due to unnecessary style rules.

## Suggested Fix
Consolidate similar styles and use CSS cascade more effectively to reduce redundancy.

```css
/* Before - redundant */
.panel.panel-animate {
  opacity: 0;
}
.panel.panel-animate.panel-animate-active {
  opacity: 1;
}

/* After - consolidated */
.panel {
  opacity: 0;
  transition: opacity 0.3s ease;
}
.panel-active {
  opacity: 1;
}
```

## Related Components
- src-ui/src/lib/components/chat/AnsweringIndicator.module.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md