# [ISSUE-003] - Hardcoded Animation Delay in Inline Thread List

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-InlineThreadList.svelte.md`
- **Component/File**: `src-ui/src/lib/components/InlineThreadList.svelte`
- **Line Numbers**: TBD

## Description
The InlineThreadList component has hardcoded animation delays that are not configurable through design system tokens. This makes the animations difficult to adjust for different contexts or accessibility needs.

## Impact
Code Quality - This affects:
- Difficult maintenance
- Inconsistent animation timing
- Hard to adapt for accessibility
- Reduced code reusability

## Suggested Fix
Use design system tokens for animation delays:

```css
/* In design system */
:root {
  --animation-duration-fast: 150ms;
  --animation-duration-base: 300ms;
  --animation-delay-stagger: 50ms;
  --easing-base: ease-in-out;
}

/* In component */
.thread-item {
  animation: slideIn var(--animation-duration-base) var(--easing-base)
             calc(var(--animation-delay-stagger) * var(--index));
}

/* Or use CSS variables in style attribute */
<div
  class="thread-item"
  style={{
    animationDelay: `${index * 50}ms`
  }}
>
```

## Related Issues
- None