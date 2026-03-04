# Animation performance in Button

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-Button.svelte.md
- **Location**: src-ui/src/lib/components/shared/Button.module.css:225-237
- **Priority**: Low
- **Category**: Performance
- **Status**: Open

## Description
The expressive animations use multiple transform properties which might cause performance issues on lower-end devices, especially with the danger shake animation.

## Impact
Complex button animations can cause:
- Performance issues on low-end devices
- Unnecessary GPU/CPU usage
- Battery drain on mobile devices
- Potential animation jank during interactions
- Reduced accessibility for users with motion sensitivity

## Suggested Fix
Consider using CSS `will-change` property for animated elements and ensure transform animations are hardware-accelerated.

```css
/* Optimized button animations */
.btn {
  transition: all 0.2s ease-out;
  will-change: transform, opacity;
}

.btn:hover {
  transform: translateY(-2px);
}

.btn:active {
  transform: translateY(0);
}

/* Optimized danger shake animation */
.btn.danger:hover {
  animation: dangerShake 0.3s ease-in-out;
  animation-iteration-count: 2;
}

@keyframes dangerShake {
  0%, 100% {
    transform: translateX(0);
  }
  25% {
    transform: translateX(-5px);
  }
  75% {
    transform: translateX(5px);
  }
}

/* Alternative: Use simpler transform animations */
.btn-expressive {
  position: relative;
  overflow: hidden;
}

.btn-expressive::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 0;
  height: 0;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.5);
  transform: translate(-50%, -50%);
  transition: width 0.6s, height 0.6s;
  will-change: width, height;
}

.btn-expressive:hover::after {
  width: 300px;
  height: 300px;
}

/* Add reduced-motion support */
@media (prefers-reduced-motion: reduce) {
  .btn,
  .btn.danger {
    animation: none;
    transition: none;
  }
}
```

## Related Components
- Button.svelte
- Button.module.css
- Shared UI components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-Button.svelte.md