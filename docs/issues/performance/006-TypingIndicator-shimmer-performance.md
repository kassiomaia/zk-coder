# Shimmer animation performance in TypingIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/TypingIndicator-review.md
- **Location**: TypingIndicator.module.css:88-98
- **Priority**: Medium
- **Category**: Performance
- **Status**: Open

## Description
The shimmer effect uses `background-clip: text` which can be expensive for performance, especially on lower-end devices.

## Impact
The current shimmer animation implementation uses `background-clip: text` which:
- Triggers expensive compositing operations
- Can cause performance issues on low-end devices
- May lead to dropped frames during animation
- Creates unnecessary repaints

## Suggested Fix
Consider using a simpler animation or add `will-change: background-position` for better performance.

```css
/* Optimized shimmer animation */
.shimmer {
  background: linear-gradient(
    90deg,
    var(--color-primary-light) 0%,
    var(--color-primary) 50%,
    var(--color-primary-light) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite;
  will-change: background-position; /* Add for better performance */
}

/* Alternative: Use opacity-based animation */
.shimmer-alternative {
  background: var(--color-primary-light);
  animation: shimmer-pulse 1.5s infinite;
  will-change: opacity;
}

@keyframes shimmer-pulse {
  0%, 100% { opacity: 0.3; }
  50% { opacity: 0.8; }
}
```

## Related Components
- TypingIndicator.svelte
- TypingIndicator.module.css

## References
- Original review: docs/reviews/code-reviewer/TypingIndicator-review.md