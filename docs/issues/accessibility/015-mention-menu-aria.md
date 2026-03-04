# Missing ARIA attributes for dynamic content in MentionMenu

## Metadata
- **Source Review**: docs/reviews/code-reviewer/mention-menu-review-2026-03-04.md
- **Location**: src-ui/src/lib/components/chat/input/MentionMenu.svelte:92
- **Priority**: Medium
- **Category**: Accessibility
- **Status**: Open

## Description
Missing ARIA attributes for dynamic content updates and search result count. The MentionMenu component doesn't properly announce when search results change or how many results are available to screen reader users.

## Impact
Screen reader users cannot determine how many mention options are available or when the search results have been updated, making it difficult to navigate through the mention options effectively.

## Suggested Fix
Add aria-activedescendant and provide count of search results to screen readers.

```html
<!-- Fix example -->
<div
  class="mention-menu"
  role="listbox"
  aria-label="Mentions"
  aria-expanded={visible}
  aria-activedescendant={activeItem?.id}
>
  <div class="results-count" aria-hidden="true">
    {results.length} results
  </div>
  {#each results as result (result.id)}
    <div
      class="mention-item"
      role="option"
      tabindex={result.id === activeItem?.id ? 0 : -1}
      aria-selected={result.id === activeItem?.id}
      data-id={result.id}
    >
      {result.display}
    </div>
  {/each}
</div>
```

## Related Components
- MentionMenu.svelte
- ChatInput.svelte

## References
- Original review: docs/reviews/code-reviewer/mention-menu-review-2026-03-04.md
- WAI-ARIA Listbox Pattern: https://www.w3.org/WAI/ARIA/apg/patterns/listbox/