# Icon rendering could be extracted in CommandPalette

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_10-30-45_CommandPalette-Review.md
- **Location**: CommandPalette.svelte:117-194
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The inline SVG icons make the component quite long (40+ lines of SVG code).

## Impact
The lengthy SVG code makes the component harder to read and maintain.

## Suggested Fix
Create an `Icon.svelte` component that takes an icon name as prop and renders the appropriate SVG.

## Related Components
- CommandPalette.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_10-30-45_CommandPalette-Review.md