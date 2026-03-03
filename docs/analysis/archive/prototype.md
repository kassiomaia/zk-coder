# Archive Feature Prototype

## Overview
Implement an archive feature that allows users to archive conversations and threads from the sidebar and thread list.

## Requirements

### Functional Requirements
1. **Archive Button**: Add a small archive button to both ConversationItem and ThreadItem components
2. **Visual Feedback**: Show archive action in the action menu (similar to delete/rename)
3. **Data Persistence**: Archive functionality should be handled by the backend
4. **UI Updates**: Update UI immediately after archiving to remove the item from the list

### Non-Functional Requirements
1. **Accessibility**: Archive button must be keyboard accessible and have proper ARIA labels
2. **Consistency**: Archive functionality should follow the same patterns as delete/rename
3. **Performance**: Should not impact list rendering performance

## Implementation Plan

### Component Changes

#### 1. ConversationItem.svelte
- Add archive handler to props interface
- Add archive button to action menu when `onarchive` is provided
- Include archive icon in action menu
- Update action menu to show archive button alongside delete/rename

#### 2. ThreadItem.svelte
- Add archive handler to props interface
- Add archive button to action menu when `onarchive` is provided
- Include archive icon in action menu
- Place archive in appropriate position in action menu

### Data Structure
The archive functionality should:
- Move conversations/threads to an "archived" state in the backend
- Remove them from active view but keep them in the database
- Allow for potential unarchiving in the future

### UI/UX Considerations
1. **Icon**: Use a standard archive icon (box with down arrow)
2. **Tooltip**: "Archive conversation/thread" on hover
3. **Confirmation**: No confirmation needed for archive (same as delete)
4. **Animation**: Fade out animation when archiving (if time permits)

### Backend Integration
- Create new API command for archiving conversations: `archive_conversation`
- Create new API command for archiving threads: `archive_thread`
- Commands should return success/failure status

## Files to Modify
```
src-ui/src/lib/components/chat/sidebar/ConversationItem.svelte
src-ui/src/lib/components/chat/thread/ThreadItem.svelte
src-ui/src/lib/components/chat/sidebar/ConversationActions.svelte (may need update)
```

## Questions for Implementation
1. Should archived items be moved to a separate "Archived" section in the sidebar?
2. Should there be an option to unarchive items?
3. Should archive be reversible with an undo action?
4. Should archived items be included in search results?