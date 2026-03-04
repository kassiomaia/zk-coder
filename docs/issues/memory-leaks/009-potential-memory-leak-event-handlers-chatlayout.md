# Issue 009: Potential memory leak with event handlers in ChatLayout

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ChatLayout.svelte_2026-03-04.md
- **Location**: src-ui/src/lib/components/layout/ChatLayout.svelte:222-241
- **Priority**: Low
- **Category**: Memory Leaks
- **Status**: Open

## Description
The component passes many event handlers to child components, but there's no cleanup verification. If child components store references to these handlers, they might prevent garbage collection.

## Impact
When ChatLayout passes event handlers to child components:
- Child components might store references to these handlers
- If the child components don't clean up properly, the handlers prevent garbage collection
- This can lead to memory leaks especially with frequent component recreation
- The entire component tree might be kept in memory longer than expected

## Suggested Fix
Consider using weak references or ensure child components properly clean up event listeners.

```typescript
// Option 1: Use weak references for event handlers
import { createWeakRef } from './utils/weakRef';

const handleClick = createWeakRef((event: MouseEvent) => {
  // Handle click
});

// Option 2: Use event delegation instead of individual handlers
// Move all event handling to a single parent container
<div
  class="chat-layout"
  on:click={handleChatLayoutClick}
  on:keydown={handleChatLayoutKeydown}
>
  <!-- Child components without individual handlers -->
</div>

function handleChatLayoutClick(event: MouseEvent) {
  // Determine which child was clicked based on event target
}

// Option 3: Ensure child components clean up
$: onDestroy(() => {
  // Clean up any references to child components
  cleanupChildComponents();
});
```

## Related Components
- ChatLayout.svelte
- ChatArea.svelte
- ConversationSidebar.svelte
- ThreadPanel.svelte

## References
- Original review: docs/reviews/code-reviewer/ChatLayout.svelte_2026-03-04.md