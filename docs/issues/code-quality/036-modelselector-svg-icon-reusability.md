# SVG icon reusability in ModelSelector

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ModelSelector.svelte.md
- **Location**: ModelSelector.svelte:134-146,148-162,192-204
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
Multiple SVG icons are duplicated in the component, reducing code reusability.

## Impact
The duplicated SVG icons increase code size and make maintenance more difficult.

## Suggested Fix
Extract SVG icons into separate components or utility functions to avoid duplication.

## Related Components
- ModelSelector.svelte

## References
- Original review: docs/reviews/code-reviewer/ModelSelector.svelte.md