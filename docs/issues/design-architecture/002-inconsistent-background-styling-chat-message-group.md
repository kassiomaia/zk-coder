# [ISSUE-002] - Inconsistent Background Styling in ChatMessageGroup

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04_ConversationItem.svelte.md`
- **Component/File**: `src-ui/src/lib/components/ChatMessageGroup.svelte`
- **Line Numbers**: TBD

## Description
The ChatMessageGroup component has inconsistent background styling across different message types and contexts. This creates visual inconsistencies and breaks the design system's color palette.

## Impact
Design Architecture - This affects:
- Visual consistency
- Design system adherence
- User experience
- Brand perception

## Suggested Fix
Standardize background styling with design tokens:

```css
/* In design system */
:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f8f9fa;
  --bg-message-user: #007bff;
  --bg-message-ai: #f1f3f4;
  --bg-code: #2d2d2d;
  --bg-highlight: #fff3cd;
}

/* In component */
.message-group {
  background: var(--bg-primary);
  border-radius: var(--radius-md);
  padding: var(--spacing-md);
}

.message-group.user {
  background: var(--bg-message-user);
}

.message-group.ai {
  background: var(--bg-message-ai);
}

.message-group code {
  background: var(--bg-code);
}

/* Use semantic classes */
.message-container {
  @apply bg-white rounded-lg shadow-sm;
}

.user-message {
  @apply bg-blue-500 text-white;
}

.ai-message {
  @apply bg-gray-100;
}
```

## Related Issues
- None