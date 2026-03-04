# Large SVG block in IntentIndicator

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-IntentIndicator.svelte.md
- **Location**: src-ui/src/lib/components/chat/input/IntentIndicator.svelte:131-212
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The large block of SVG icons (40+ lines of SVG code) makes the component hard to maintain.

## Impact
The large SVG block increases component complexity and makes it difficult to read and maintain the code.

## Suggested Fix
Create a dedicated icon component or utility function to handle SVG icon rendering.

## Related Components
- IntentIndicator.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-IntentIndicator.svelte.md