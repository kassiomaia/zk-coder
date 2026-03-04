# Animation performance in Modal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/Modal.svelte_2026-03-04.md
- **Location**: src-ui/src/lib/components/shared/Modal.module.css:75-114
- **Priority**: Low
- **Category**: Performance
- **Status**: Open

## Description
The modal uses multiple complex animations that might cause performance issues on lower-end devices.

## Impact
Complex modal animations can cause:
- Performance issues on low-end devices
- Unnecessary GPU/CPU usage
- Battery drain on mobile devices
- Potential animation jank
- Reduced accessibility for users with motion sensitivity

## Suggested Fix
Optimize animations by using transform and opacity only, and consider adding prefers-reduced-motion handling for the content reveal animations.

```css
/* Optimized modal animations */
.modal-overlay {
  opacity: 0;
  transition: opacity 0.3s ease-out;
  will-change: opacity;
}

.modal-overlay.show {
  opacity: 1;
}

.modal-content {
  transform: scale(0.9) translateY(20px);
  opacity: 0;
  transition: transform 0.3s ease-out, opacity 0.3s ease-out;
  will-change: transform, opacity;
}

.modal-overlay.show .modal-content {
  transform: scale(1) translateY(0);
  opacity: 1;
}

/* Add reduced-motion support */
@media (prefers-reduced-motion: reduce) {
  .modal-overlay,
  .modal-content {
    transition: none;
    animation: none;
  }

  .modal-overlay.show {
    opacity: 1;
  }

  .modal-overlay.show .modal-content {
    transform: scale(1) translateY(0);
    opacity: 1;
  }
}

/* Alternative: Use more performant properties only */
.modal-content-simple {
  animation: modalFadeIn 0.3s ease-out;
}

@keyframes modalFadeIn {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

## Related Components
- Modal.svelte
- Modal.module.css
- Overlay components

## References
- Original review: docs/reviews/code-reviewer/Modal.svelte_2026-03-04.md