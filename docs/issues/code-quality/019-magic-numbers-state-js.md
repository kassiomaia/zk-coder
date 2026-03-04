# Magic numbers in state.js

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_agent-coding-input-state-js-review.md
- **Location**: docs/prototypes/agent-coding-input/js/state.js:344,450
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
Several magic numbers are used without explanation (e.g., scrollPadding: 8, setTimeout delays).

## Impact
Magic numbers reduce code maintainability by making it difficult to understand the purpose of values and requiring changes in multiple places when adjustments are needed.

## Suggested Fix
Define these as constants at the top of the file for better maintainability.

```javascript
// Define constants at the top of state.js
const SCROLL_PADDING = 8;
const ANIMATION_DELAY = 150;
const DEFAULT_TIMEOUT_DELAY = 1000;

// Then use constants instead of magic numbers
// Example: scrollPadding: SCROLL_PADDING instead of scrollPadding: 8
// Example: setTimeout(() => { ... }, DEFAULT_TIMEOUT_DELAY)
```

## Related Components
- docs/prototypes/agent-coding-input/js/state.js

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_agent-coding-input-state-js-review.md