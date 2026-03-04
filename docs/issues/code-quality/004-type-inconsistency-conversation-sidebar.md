# [ISSUE-004] - Type Inconsistency in Conversation Sidebar

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/code-reviewer-conversation-sidebar.md`
- **Component/File**: `src-ui/src/lib/components/ConversationSidebar.svelte`
- **Line Numbers**: TBD

## Description
The ConversationSidebar component has type inconsistencies in its data structures. Properties use different naming conventions and type definitions, leading to potential runtime errors and maintenance issues.

## Impact
Code Quality - This affects:
- Type safety
- Runtime errors
- Difficult maintenance
- Poor developer experience

## Suggested Fix
Standardize type definitions:

```typescript
// Define consistent interface
interface Conversation {
  id: string;
  title: string;
  lastMessage: {
    id: string;
    content: string;
    timestamp: Date;
    isUser: boolean;
  };
  unread: boolean;
  pinned: boolean;
  metadata?: {
    threadType: 'chat' | 'coding' | 'planning';
    priority?: 'low' | 'medium' | 'high';
  };
}

// Use consistent naming
interface ConversationSidebarProps {
  conversations: Conversation[];
  selectedConversation: string | null;
  onConversationSelect: (id: string) => void;
}
```

## Related Issues
- None