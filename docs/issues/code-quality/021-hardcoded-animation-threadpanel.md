# Hardcoded animation durations in ThreadPanel

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md
- **Location**: ThreadPanel.svelte:132,134,163,172,317
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
Animation durations are hardcoded in multiple places (150ms, 200ms, 300ms). This makes it difficult to maintain consistent timing across the application.

## Impact
Hardcoded animation durations create inconsistencies across components and make global style updates difficult. It violates the DRY principle and requires coordinated changes across multiple locations.

## Suggested Fix
Define animation duration tokens in the design system and reference them from there for consistency.

```javascript
// In design-system.ts or design-constants.ts
export const ANIMATION_DURATIONS = {
  fast: '150ms',
  normal: '200ms',
  slow: '300ms',
} as const;

// In ThreadPanel.svelte
import { ANIMATION_DURATIONS } from '$lib/styles/design-system';

// Use constants instead of hardcoded values
transition: { duration: ANIMATION_DURATIONS.normal }
```

## Related Components
- src-ui/src/lib/components/chat/ThreadPanel.svelte
- src-ui/src/lib/styles/design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md