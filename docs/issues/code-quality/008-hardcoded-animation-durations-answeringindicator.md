# Hardcoded animation durations in AnsweringIndicator

## Status
✅ **RESOLVED** (2026-03-04)

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/AnsweringIndicator.svelte:22-23
- **Priority**: Medium
- **Category**: Code Quality

## Description
The durations (PHASE_DURATION: 2200ms, SCRAMBLE_DURATION: 800ms) are hardcoded magic numbers without explanation or configuration options.

## Resolution
Added JSDoc comments explaining the animation configuration constants:

```typescript
/**
 * Animation configuration constants.
 * These control the timing of the phase transitions and text scrambling effects.
 */
const PHASE_DURATION = 2200; // Duration for each phase in milliseconds (includes scramble time)
const SCRAMBLE_DURATION = 800; // Duration for the letter scramble animation in milliseconds
```

## Files Changed
- `src-ui/src/lib/components/chat/AnsweringIndicator.svelte`

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-AnsweringIndicator.svelte.md
