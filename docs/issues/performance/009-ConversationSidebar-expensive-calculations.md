# Expensive calculations in ConversationSidebar

## Metadata
- **Source Review**: docs/reviews/code-reviewer/code-reviewer-2026-03-04-conversation-sidebar.md
- **Location**: src-ui/src/lib/components/chat/sidebar/ConversationSidebar.svelte:94-95, 125-154
- **Priority**: Medium
- **Category**: Performance
- **Status**: Open

## Description
The component performs expensive filtering operations on every render without memoization.

## Impact
Unoptimized filtering operations can cause:
- Performance degradation with large thread lists
- Unnecessary recalculations on every render
- Reduced UI responsiveness
- Increased CPU usage
- Poor user experience during scrolling or interactions

## Suggested Fix
Use Svelte's `$derived` or `$effect` for better optimization, or implement memoization for these calculations.

```javascript
// Current problematic implementation
const openThreads = threads.filter(t => !t.resolved);
const resolvedThreads = threads.filter(t => t.resolved);

// Optimized using $derived
const openThreads = $derived(
  threads.filter(t => !t.resolved)
);

const resolvedThreads = $derived(
  threads.filter(t => t.resolved)
);

// Alternative: Implement custom memoization
const memoizedFilter = (list, predicate) => {
  let cachedList = list;
  let cachedPredicate = predicate;
  let result = [];

  return $derived(() => {
    if (cachedList !== list || cachedPredicate !== predicate) {
      result = list.filter(predicate);
      cachedList = list;
      cachedPredicate = predicate;
    }
    return result;
  });
};

const openThreads = memoizedFilter(threads, t => !t.resolved);
```

## Related Components
- ConversationSidebar.svelte
- ThreadItem components
- Sidebar components

## References
- Original review: docs/reviews/code-reviewer/code-reviewer-2026-03-04-conversation-sidebar.md