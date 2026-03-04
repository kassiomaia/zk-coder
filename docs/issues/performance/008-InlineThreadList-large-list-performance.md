# Large thread list performance in InlineThreadList

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-InlineThreadList.svelte.md
- **Location**: InlineThreadList.module.css:33
- **Priority**: Medium
- **Category**: Performance
- **Status**: Open

## Description
The component uses a max-height of 600px for the expanded container, which may cause performance issues with very long thread lists. The current implementation may render many DOM elements at once.

## Impact
Rendering large thread lists can cause:
- DOM bloating with many elements
- Poor scrolling performance
- Increased memory usage
- Slow initial render times
- UI lag when expanding/collapsing

## Suggested Fix
Consider virtualization for large thread lists or implement lazy loading with pagination.

```javascript
// Virtual scrolling approach using svelte-virtual-list
import VirtualList from 'svelte-virtual-list';

function renderThreads(threads) {
  return new VirtualList({
    items: threads,
    itemHeight: 60, // Approximate height of each thread item
    containerHeight: 600,
    renderItem: ({ item, index }) => {
      return `<ThreadItem data-index="${index}" />`;
    }
  });
}

// Alternative: Lazy loading with pagination
let visibleThreads = $derived(threads.slice(0, PAGE_SIZE));
let isLoadingMore = false;

function loadMoreThreads() {
  if (!isLoadingMore) {
    isLoadingMore = true;
    setTimeout(() => {
      visibleThreads = threads.slice(0, visibleThreads.length + PAGE_SIZE);
      isLoadingMore = false;
    }, 300);
  }
}
```

## Related Components
- InlineThreadList.svelte
- InlineThreadList.module.css
- ThreadItem components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-InlineThreadList.svelte.md