# Hardcoded SVG paths in ListItemActions

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ListItemActions.svelte_2026-03-04.md
- **Location**: ListItemActions.svelte:49-69
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
SVG icon paths are hardcoded directly in the code, making them difficult to maintain and update.

## Impact
Hardcoded SVG paths make it difficult to update icons consistently across the application.

## Suggested Fix
Move icon paths to a separate constants file or use an icon library.

## Related Components
- ListItemActions.svelte

## References
- Original review: docs/reviews/code-reviewer/ListItemActions.svelte_2026-03-04.md