# Inefficient DOM updates in state.js

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-agent-coding-input-state-review.md
- **Location**: docs/prototypes/agent-coding-input/js/state.js:228-232
- **Priority**: Low
- **Category**: Performance
- **Status**: Open

## Description
The handleScroll() method updates palette position on every scroll event, which could be performance intensive.

## Impact
Scroll event handling without optimization can cause:
- Poor scrolling performance
- Excessive DOM updates
- Unnecessary reflows/repaints
- Janky UI behavior
- Reduced battery life on mobile devices

## Suggested Fix
Use requestAnimationFrame or debouncing to optimize scroll event handling.

```javascript
// Current implementation (problematic)
const handleScroll = () => {
  const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
  setPalettePosition({ top: scrollTop + 20 });
};

// Optimized with requestAnimationFrame
let rafId;
const handleScrollOptimized = () => {
  if (!rafId) {
    rafId = requestAnimationFrame(() => {
      const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
      setPalettePosition({ top: scrollTop + 20 });
      rafId = null;
    });
  }
};

// Alternative: Use debouncing
let scrollTimeout;
const handleScrollDebounced = () => {
  clearTimeout(scrollTimeout);
  scrollTimeout = setTimeout(() => {
    const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
    setPalettePosition({ top: scrollTop + 20 });
  }, 16); // ~60fps
};

// Best: Combine both approaches
let ticking = false;
const handleScrollOptimized = () => {
  if (!ticking) {
    requestAnimationFrame(() => {
      const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
      setPalettePosition({ top: scrollTop + 20 });
      ticking = false;
    });
    ticking = true;
  }
};
```

## Related Components
- CommandPalette.svelte
- state.js (prototype)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-agent-coding-input-state-review.md