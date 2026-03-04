# Accessibility improvements for command palette

## Metadata
- **Source Review**: docs/reviews/claude-opus-4-6/2026-03-04-14-30-00-code-review-chat-input.md
- **Location**: ChatInput.svelte:404-405
- **Priority**: Low
- **Category**: Accessibility
- **Status**: Open

## Description
The command palette lacks proper ARIA attributes for keyboard navigation and screen readers when active. Users who rely on keyboard navigation or screen readers may have difficulty accessing the available commands efficiently.

## Impact
Users with visual impairments or those who prefer keyboard navigation will face barriers in accessing the command palette functionality, making the application less accessible to this user group.

## Suggested Fix
Add proper ARIA attributes for keyboard navigation and screen readers when command palette is active. Consider implementing roving tabindex.

```html
<!-- Fix example -->
<div
  class="command-palette"
  role="listbox"
  aria-label="Command palette"
  aria-expanded="true"
  aria-activedescendant="{activeItemId}"
>
  {#each commands as command (command.id)}
    <div
      class="command-item"
      role="option"
      tabindex={command.id === activeItemId ? 0 : -1}
      data-id={command.id}
      aria-selected={command.id === activeItemId}
    >
      {command.label}
    </div>
  {/each}
</div>
```

## Related Components
- ChatInput.svelte
- CommandPalette.svelte

## References
- Original review: docs/reviews/claude-opus-4-6/2026-03-04-14-30-00-code-review-chat-input.md
- WAI-ARIA Listbox Pattern: https://www.w3.org/WAI/ARIA/apg/patterns/listbox/