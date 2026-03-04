# Glow animation performance in Tooltip

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md
- **Location**: src-ui/src/lib/components/shared/Tooltip.module.css:171-189
- **Priority**: Low
- **Category**: Performance
- **Status**: Open

## Description
The glow animation could cause performance issues on low-end devices due to continuous box-shadow changes.

## Impact
Box-shadow animations can cause:
- Performance issues on low-end devices
- GPU/CPU intensive operations
- Battery drain on mobile devices
- Potential animation jank
- Reduced accessibility for users with motion sensitivity

## Suggested Fix
Use transform-based animations instead of box-shadow for better performance, or make the glow optional via a prop.

```css
/* Current problematic implementation (box-shadow animation) */
.tooltip-glow {
  animation: glow 2s ease-in-out infinite;
}

@keyframes glow {
  0%, 100% {
    box-shadow: 0 0 5px var(--color-primary);
  }
  50% {
    box-shadow: 0 0 20px var(--color-primary),
                0 0 30px var(--color-primary);
  }
}

/* Optimized using transform and opacity */
.tooltip-glow {
  animation: glow-pulse 2s ease-in-out infinite;
  will-change: opacity;
}

@keyframes glow-pulse {
  0%, 100% {
    opacity: 0.6;
    transform: scale(1);
  }
  50% {
    opacity: 1;
    transform: scale(1.05);
  }
}

/* Alternative: Use text-shadow for glowing text */
.tooltip-glow-text {
  text-shadow: 0 0 10px var(--color-primary);
  animation: text-glow 2s ease-in-out infinite;
  will-change: text-shadow;
}

@keyframes text-glow {
  0%, 100% {
    text-shadow: 0 0 10px var(--color-primary);
  }
  50% {
    text-shadow: 0 0 20px var(--color-primary),
                 0 0 30px var(--color-primary);
  }
}

/* Make glow optional via prop */
.tooltip[data-glow="true"] {
  animation: glow-pulse 2s ease-in-out infinite;
}

/* Add reduced-motion support */
@media (prefers-reduced-motion: reduce) {
  .tooltip-glow,
  .tooltip-glow-text {
    animation: none;
    text-shadow: none;
  }
}
```

## Related Components
- Tooltip.svelte
- Tooltip.module.css
- Shared UI components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md