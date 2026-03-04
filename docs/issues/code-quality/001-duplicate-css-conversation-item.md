# [ISSUE-001] - Duplicate CSS in ConversationItem Component

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04_ConversationItem.svelte.md`
- **Component/File**: `src-ui/src/lib/components/ConversationItem.svelte`
- **Line Numbers**: TBD

## Description
The ConversationItem component has duplicate CSS rules that could be consolidated into shared utility classes or design system tokens. This increases maintenance burden and leads to inconsistencies.

## Impact
Code Quality - This affects:
- Maintenance difficulty
- CSS bloat
- Inconsistent styling
- Reduced developer productivity

## Suggested Fix
Extract common styles into shared classes:

```css
/* In shared utilities or design system */
.message-avatar {
  @apply w-10 h-10 rounded-full flex-shrink-0;
}

.message-content {
  @apply flex-1 min-w-0;
}

.message-timestamp {
  @apply text-xs text-gray-500 ml-auto;
}
```

```typescript
// In component
<div class="conversation-item">
  <div class="message-avatar">
    <Avatar src={user.avatar} />
  </div>
  <div class="message-content">
    <MessageContent message={message} />
  </div>
  <span class="message-timestamp">
    {formatTimestamp(message.timestamp)}
  </span>
</div>
```

## Related Issues
- None