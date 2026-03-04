# Redundant opacity transitions in ModelCard

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-ModelCard-review.md
- **Location**: src-ui/src/lib/components/config/ModelCard.module.css:170-181
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
Multiple opacity transitions are defined separately in the checkIcon animation when they could be consolidated.

## Impact
Redundant CSS transitions increase code size and make maintenance more difficult. It can also cause performance issues by having multiple similar animation rules.

## Suggested Fix
Consolidate opacity transitions in the animation keyframes.

```css
/* Before - redundant transitions */
@keyframes checkIcon {
  0% {
    opacity: 0;
    transform: scale(0.8);
  }
  50% {
    opacity: 1;
    transform: scale(1.1);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}

/* After - consolidated */
@keyframes checkIcon {
  0% {
    opacity: 0;
    transform: scale(0.8);
  }
  50% {
    opacity: 1;
    transform: scale(1.1);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}
```

## Related Components
- src-ui/src/lib/components/config/ModelCard.module.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-ModelCard-review.md