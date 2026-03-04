# [ISSUE-001] - Tight Coupling Between ScrollController and ChatArea

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-ChatArea-Review.md`
- **Component/File**: `src-ui/src/lib/components/ChatArea.svelte`, `src-ui/src/lib/utils/scrollController.ts`
- **Line Numbers**: TBD

## Description
The ChatArea component has tight coupling with the ScrollController utility. This creates a dependency that makes it difficult to test, maintain, and reuse components independently.

## Impact
Design Architecture - This affects:
- Component reusability
- Testability
- Maintainability
- Code organization

## Suggested Fix
Implement loose coupling with event-based communication:

```typescript
// Create a scroll event bus
import { createEventDispatcher } from 'svelte';

const scrollEvents = createEventDispatcher();

// In ScrollController
export function scrollToBottom() {
  scrollEvents('scroll-to-bottom', { behavior: 'smooth' });
}

// In ChatArea
scrollEvents.addEventListener('scroll-to-bottom', ({ behavior }) => {
  const element = chatContainerRef;
  element.scrollTo({
    top: element.scrollHeight,
    behavior: behavior || 'auto'
  });
});

// Or use a provider pattern
<ScrollContextProvider value={scrollContext}>
  <ChatArea />
</ScrollContextProvider>
```

## Related Issues
- None