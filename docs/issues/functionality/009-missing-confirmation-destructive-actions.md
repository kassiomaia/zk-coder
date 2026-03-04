# Missing confirmation for destructive actions

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ ConversationActions.svelte.md
- **Location**: src-ui/src/lib/components/chat/sidebar/ConversationActions.svelte:84-113
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
The delete button has no confirmation mechanism before executing the delete action.

## Impact
Users might accidentally delete important conversations due to misclicks, with no way to undo the action once completed.

## Suggested Fix
Add a confirmation dialog or use the browser's confirm() API before executing the delete action.

```typescript
// Example fix for ConversationActions.svelte
const handleDelete = async () => {
  if (confirm('Are you sure you want to delete this conversation? This action cannot be undone.')) {
    try {
      await deleteConversation(threadId);
      // Navigate away or show success message
    } catch (error) {
      console.error('Failed to delete conversation:', error);
      // Show error message
    }
  }
};

// Or using a more sophisticated ConfirmDialog component:
const handleDelete = () => {
  confirmDialog.show({
    title: 'Delete Conversation',
    message: 'Are you sure you want to delete this conversation? This action cannot be undone.',
    variant: 'warning',
    onConfirm: async () => {
      try {
        await deleteConversation(threadId);
        // Navigate away or show success message
      } catch (error) {
        console.error('Failed to delete conversation:', error);
        // Show error message
      }
    },
    onCancel: () => {
      // User cancelled
    }
  });
};
```

## Related Components
- ConversationActions.svelte
- ConfirmDialog component (if available)
- deleteConversation API function

## References
- Original review: docs/reviews/code-reviewer/ ConversationActions.svelte.md