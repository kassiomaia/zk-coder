# Icon markup as HTML strings in ConversationSidebar

## Metadata
- **Source Review**: docs/reviews/code-reviewer/code-reviewer-2026-03-04-conversation-sidebar.md
- **Location**: src-ui/src/lib/components/chat/sidebar/ConversationSidebar.svelte:103-108
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
Icons are embedded as HTML strings which is harder to maintain and type-check.

## Impact
Using HTML strings for icons makes the code harder to read, maintain, and type-check.

## Suggested Fix
Extract icons to separate Svelte components.

## Related Components
- ConversationSidebar.svelte

## References
- Original review: docs/reviews/code-reviewer/code-reviewer-2026-03-04-conversation-sidebar.md