# Inconsistent icon usage in ConversationItem

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_ConversationItem.svelte.md
- **Location**: src-ui/src/lib/components/chat/sidebar/ConversationItem.svelte:94-106,127-139
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The chat icon is duplicated in both the edit mode snippet and in the button element. This duplication violates the DRY principle.

## Impact
Having the same SVG code in multiple places makes it harder to maintain and can lead to inconsistencies when updates are needed.

## Suggested Fix
Extract the icon into a separate component or use a template to reuse the same icon SVG.

## Related Components
- ConversationItem.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_ConversationItem.svelte.md