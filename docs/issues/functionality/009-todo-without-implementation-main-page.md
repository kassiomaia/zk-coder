# [ISSUE-009] - TODO Comments Without Implementation in Main Page

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04_main_page_review.md`
- **Component/File**: `src-ui/src/lib/pages/MainPage.svelte`
- **Line Numbers**: TBD

## Description
The MainPage component contains TODO comments that have been left without implementation. These incomplete features create functionality gaps and poor user experience.

## Impact
Functionality - This affects:
- Missing features
- Incomplete user experience
- Developer confusion
- Broken functionality expectations

## Suggested Fix
Implement or remove TODO items:

```typescript
// Bad - TODO without implementation
// TODO: Add search functionality
// <input type="text" placeholder="Search..." />

// Good - Implemented functionality
<input
  type="text"
  placeholder="Search conversations..."
  on:input={handleSearch}
  bind:value={searchQuery}
/>

function handleSearch(event: Event) {
  const query = (event.target as HTMLInputElement).value;
  filteredConversations = conversations.filter(conv =>
    conv.title.toLowerCase().includes(query.toLowerCase())
  );
}

// Or remove if not needed
// TODO: Implement dark mode (not planned for v1)
```

## Related Issues
- None