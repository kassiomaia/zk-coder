# Duplicated SVG separator code in ThreadBreadcrumbs

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-code-reviewer-thread-breadcrumbs.md
- **Location**: src-ui/src/lib/components/chat/thread/ThreadBreadcrumbs.svelte:56-69,76-89,104-117
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The separator SVG is duplicated multiple times in the template, violating DRY principles.

## Impact
The duplicated SVG code makes the component harder to maintain and increases the risk of inconsistencies when modifications are needed.

## Suggested Fix
Extract the SVG into a separate component or create a helper function.

## Related Components
- ThreadBreadcrumbs.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-code-reviewer-thread-breadcrumbs.md