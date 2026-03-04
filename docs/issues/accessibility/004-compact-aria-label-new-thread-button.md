# [ISSUE-004] - Compact ARIA Label for New Thread Button

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-NewThreadButton.svelte.md`
- **Component/File**: `src-ui/src/lib/components/NewThreadButton.svelte`
- **Line Numbers**: TBD

## Description
The NewThreadButton component has a compact design that may not provide sufficient context to screen reader users. The current implementation likely doesn't include a descriptive ARIA label explaining what the button does.

## Impact
Accessibility - This fails WCAG guidelines by:
- Providing insufficient context for screen reader users
- Making it unclear what action the button performs
- Reducing the usability for visually impaired users

## Suggested Fix
Add a descriptive ARIA label that explains the button's action:

```typescript
<button
  class="new-thread-button"
  aria-label="Create new conversation thread"
  aria-describedby="new-thread-help"
  on:click={handleClick}
>
  <Icon name="plus" />
  <span id="new-thread-help" class="sr-only">
    Starts a new conversation thread with the AI assistant
  </span>
</button>
```

## Related Issues
- None