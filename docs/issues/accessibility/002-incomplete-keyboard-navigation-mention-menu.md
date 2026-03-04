# [ISSUE-002] - Incomplete Keyboard Navigation in Mention Menu

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/mention-menu-review-2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/MentionMenu.svelte`
- **Line Numbers**: TBD

## Description
The mention menu component lacks proper keyboard navigation support. Users cannot navigate through menu items using arrow keys, select items with Enter, or close the menu with Escape. This makes the mention feature inaccessible to keyboard-only users.

## Impact
Accessibility - This fails WCAG guidelines by:
- Violating WCAG 2.1.1 (Keyboard)
- Preventing keyboard-only users from accessing mention functionality
- Creating a poor user experience for assistive technology users
- Breaking keyboard navigation flow

## Suggested Fix
Implement complete keyboard navigation:

```typescript
let selectedIndex = 0;

function handleKeydown(event: KeyboardEvent) {
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault();
      selectedIndex = (selectedIndex + 1) % filteredItems.length;
      break;
    case 'ArrowUp':
      event.preventDefault();
      selectedIndex = selectedIndex > 0 ? selectedIndex - 1 : filteredItems.length - 1;
      break;
    case 'Enter':
      event.preventDefault();
      if (filteredItems[selectedIndex]) {
        selectItem(filteredItems[selectedIndex]);
      }
      break;
    case 'Escape':
      event.preventDefault();
      open = false;
      break;
  }
}

// In template
<div
  role="listbox"
  tabindex="0"
  onkeydown={handleKeydown}
  aria-activedescendant={`mention-${selectedIndex}`}
>
  {#each filteredItems as (item, index)}
    <div
      role="option"
      id={`mention-${index}`}
      aria-selected={index === selectedIndex}
      class:selected={index === selectedIndex}
    >
      {item.display}
    </div>
  {/each}
</div>
```

## Related Issues
- None