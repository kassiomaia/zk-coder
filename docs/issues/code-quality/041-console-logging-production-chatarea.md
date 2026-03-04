# Console logging in ChatArea

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md
- **Location**: src-ui/src/lib/components/chat/ChatArea.svelte:164,248,302
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
Debug console logs should be conditionally based on environment.

## Impact
Debug console logs in production code can expose sensitive information and impact performance.

## Suggested Fix
Wrap in environment check: `if (import.meta.env.DEV) { console.log(...) }`

## Related Components
- ChatArea.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md