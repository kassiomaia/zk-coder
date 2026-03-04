# Missing ARIA labels for actions in ListItemActions

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ListItemActions.svelte_2026-03-04.md
- **Location**: ListItemActions.svelte:149-183
- **Priority**: Low
- **Category**: Accessibility
- **Status**: Open

## Description
Missing `aria-label` for menu items and keyboard navigation support. The action menu items in ListItemActions don't have proper accessibility attributes, making them difficult to navigate and understand for screen reader users.

## Impact
Screen reader users cannot identify what each action button does and cannot navigate through them using only the keyboard, significantly reducing the accessibility of the interface.

## Suggested Fix
Add proper aria attributes for menu items and implement keyboard navigation (Escape, Enter, Arrow keys).

```html
<!-- Fix example -->
<div class="list-item-actions" role="menu">
  <button
    class="action-button"
    role="menuitem"
    aria-label="More actions"
    aria-expanded={isMenuOpen}
    aria-haspopup="true"
    on:click={toggleMenu}
  >
    <svg>...</svg>
  </button>

  {#if isMenuOpen}
    <div class="action-menu" role="menu">
      <button
        role="menuitem"
        aria-label="Edit thread"
        tabindex="0"
        on:click={handleEdit}
        on:keydown={(e) => handleKeydown(e, 'edit')}
      >
        Edit
      </button>
      <button
        role="menuitem"
        aria-label="Delete thread"
        tabindex="0"
        on:click={handleDelete}
        on:keydown={(e) => handleKeydown(e, 'delete')}
      >
        Delete
      </button>
    </div>
  {/if}
</div>
```

```javascript
// Keyboard navigation handler
function handleKeydown(event, action) {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    executeAction(action);
  } else if (event.key === 'Escape') {
    event.preventDefault();
    isMenuOpen = false;
  }
}
```

## Related Components
- ListItemActions.svelte
- ThreadItem.svelte

## References
- Original review: docs/reviews/code-reviewer/ListItemActions.svelte_2026-03-04.md
- WAI-ARIA Menu Pattern: https://www.w3.org/WAI/ARIA/apg/patterns/menu/