# Hardcoded animation durations in AnsweringIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/AnsweringIndicator.svelte:22-23
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The durations (PHASE_DURATION: 2200ms, SCRAMBLE_DURATION: 800ms) are hardcoded magic numbers without explanation or configuration options.

## Impact
Hardcoded values reduce maintainability and flexibility. Without documentation, future developers won't understand the reasoning behind these specific values. It also makes it difficult to adjust animations based on user preferences or accessibility requirements.

## Suggested Fix
Define these as constants with proper JSDoc comments explaining their purpose. Consider making them configurable via props.

```typescript
// Animation durations with explanation
const PHASE_DURATION = 2200; // Duration for initial phase in milliseconds
const SCRAMBLE_DURATION = 800; // Duration for scrambling animation in milliseconds

// Optional: Make configurable via props
// const props = defineProps<{
//   phaseDuration?: number;
//   scrambleDuration?: number;
// }>();
```

## Related Components
- AnsweringIndicator.svelte
- AnsweringIndicator.module.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md