# Animation performance in MessageActions

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-MessageActions.svelte.md
- **Location**: src-ui/src/lib/components/chat/MessageActions.module.css:121-153
- **Priority**: Low
- **Category**: Performance
- **Status**: Open

## Description
The CSS animations could benefit from hardware acceleration for smoother performance.

## Impact
Current animation implementations may cause:
- Performance issues on low-end devices
- Unnecessary CPU/GPU usage
- Animation jank during user interactions
- Reduced user experience with slow animations
- Battery drain on mobile devices

## Suggested Fix
Add transform and will-change properties for animations.

```css
/* Current implementation may need optimization */
.message-action {
  transition: all 0.2s ease;
}

.message-action:hover {
  transform: scale(1.1);
}

/* Optimized animations */
.message-action {
  transition: transform 0.2s ease, opacity 0.2s ease;
  will-change: transform, opacity;
}

.message-action:hover {
  transform: scale(1.1);
}

.message-action:active {
  transform: scale(0.95);
}

/* Optimized fade animation */
.message-action.fade-in {
  animation: fadeIn 0.3s ease-out;
  will-change: opacity, transform;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Optimized slide animation */
.message-action.slide-in {
  animation: slideIn 0.3s ease-out;
  will-change: transform, opacity;
}

@keyframes slideIn {
  from {
    transform: translateX(-20px);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

/* Alternative: Use transform3d for better performance */
.message-action-3d {
  transform: translateZ(0);
  transition: transform 0.2s ease;
  will-change: transform;
}

.message-action-3d:hover {
  transform: scale3d(1.1, 1.1, 1);
}

/* Add reduced-motion support */
@media (prefers-reduced-motion: reduce) {
  .message-action,
  .message-action.fade-in,
  .message-action.slide-in {
    animation: none;
    transition: none;
    will-change: auto;
  }
}
```

## Related Components
- MessageActions.svelte
- MessageActions.module.css
- Chat message components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-MessageActions.svelte.md