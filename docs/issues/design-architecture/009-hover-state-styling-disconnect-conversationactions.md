# Hover State Styling Disconnect in ConversationActions

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ ConversationActions.svelte.md
- **Location**: src-ui/src/lib/components/chat/sidebar/ConversationActions.svelte:85-112, ConversationActions.module.css:45-53
- **Priority**: Medium
- **Category**: Design & Architecture
- **Status**: Open

## Description
The CSS defines custom hover states that are not being applied because the buttons don't have the .action class. The ConversationActions component has CSS rules for `.action:hover` (lines 45-53) but the buttons in the component template (lines 85-112) don't have the `.action` class, causing these hover states to be ignored. This creates a disconnect between the intended styling and the actual implementation.

## Impact
- Visual feedback on hover is inconsistent or missing
- Users may not perceive buttons as interactive
- Component styling doesn't match design specifications
- Confusing user experience when hovering over action buttons

## Suggested Fix
Remove the custom hover state rules from the CSS module or add the .action class to the IconButton component.

Option 1: Update CSS to match actual structure
```css
/* Remove this if not needed */
.action:hover {
  /* ... existing styles ... */
}

/* Update to target actual button class */
IconButton:hover {
  background: var(--color-background-hover);
  transform: scale(1.05);
}
```

Option 2: Add the class to buttons
```svelte
<!-- In ConversationActions.svelte -->
<IconButton class="action" on:click={handleEdit}>
  <EditIcon />
</IconButton>
```

## Related Components
- src-ui/src/lib/components/chat/sidebar/ConversationActions.svelte
- src-ui/src/lib/components/chat/sidebar/ConversationActions.module.css
- src-ui/src/lib/components/ui/IconButton.svelte

## References
- Original review: docs/reviews/code-reviewer/ ConversationActions.svelte.md