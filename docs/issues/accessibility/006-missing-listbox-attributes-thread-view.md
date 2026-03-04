# [ISSUE-006] - Missing Listbox Attributes in Thread View

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-threadpanel-review.md`
- **Component/File**: `src-ui/src/lib/components/ThreadView.svelte`
- **Line Numbers**: TBD

## Description
The thread view component doesn't properly implement listbox ARIA attributes for the thread list. Screen reader users cannot properly navigate and interact with the list of threads.

## Impact
Accessibility - This fails WCAG guidelines by:
- Not properly marking the thread list as a listbox
- Not providing proper navigation cues for screen reader users
- Making it difficult for assistive technology users to interact with threads

## Suggested Fix
Implement proper listbox ARIA attributes:

```typescript
<div
  role="listbox"
  aria-label="Conversation threads"
  aria-activedescendant={selectedThreadId ? `thread-${selectedThreadId}` : undefined}
>
  {#each threads as thread}
    <div
      role="option"
      id={`thread-${thread.id}`}
      tabindex={thread.id === selectedThreadId ? 0 : -1}
      aria-selected={thread.id === selectedThreadId}
      class:selected={thread.id === selectedThreadId}
    >
      {thread.title}
    </div>
  {/each}
</div>
```

## Related Issues
- None

## Resolution Status
**RESOLVED** - False Positive - 2026-03-04

The ThreadView and ThreadItem components already have proper listbox ARIA attributes:

**ThreadView:**
- `role="listbox"` on `<ul>` elements
- `aria-label="Open threads"` and `aria-label="Resolved threads"` on lists

**ThreadItem:**
- `role="option"` on `<li>` elements
- `aria-selected={selected}` for selection state
- `aria-label="Select thread: {thread.title}"` on the inner button

The implementation follows WCAG 2.1 guidelines for listbox patterns. The use of buttons inside list items provides additional keyboard accessibility.