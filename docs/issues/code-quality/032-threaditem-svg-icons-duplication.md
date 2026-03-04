# Duplicate SVG icons in ThreadItem

## Metadata
- **Source Review**: docs/reviews/code-reviewer/20260304_040239.md
- **Location**: ThreadItem.svelte:83-95,97-110,126-137,140-153
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The SVG icons for resolved and unresolved states are duplicated twice in the component (edit mode and normal view).

## Impact
The duplication violates the DRY principle and makes it difficult to maintain consistent styling across the component.

## Suggested Fix
Extract SVG icons into separate constants or components.

## Related Components
- ThreadItem.svelte

## References
- Original review: docs/reviews/code-reviewer/20260304_040239.md