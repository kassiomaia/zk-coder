# Hardcoded values in ThreadBreadcrumbs

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-code-reviewer-thread-breadcrumbs.md
- **Location**: src-ui/src/lib/components/chat/thread/ThreadBreadcrumbs.svelte:30-31
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
The `getVisibleItems` function has hardcoded values (startCount=1) that could be more flexible.

## Impact
Hardcoded values reduce the reusability of the component and make it difficult to customize behavior for different use cases or requirements.

## Suggested Fix
Make these configurable through props.

```javascript
// In ThreadBreadcrumbs.svelte
export let startCount = 1;
export let maxVisibleItems = 3;

// In the template
<ThreadBreadcrumbs {startCount} {maxVisibleItems} />
```

## Related Components
- src-ui/src/lib/components/chat/thread/ThreadBreadcrumbs.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-code-reviewer-thread-breadcrumbs.md