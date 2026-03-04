# Hardcoded values in NewThreadButton

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-NewThreadButton.svelte.md
- **Location**: NewThreadButton.svelte:30-31
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
The SVG width and height are hardcoded to 16px, which may cause layout issues in different contexts or when the button size changes.

## Impact
Hardcoded SVG dimensions prevent the icon from properly scaling with the button size, potentially causing layout inconsistencies. This makes the button less flexible and can lead to visual alignment issues in different contexts.

## Suggested Fix
Consider using CSS to control SVG size or make it relative to the button's font size.

```typescript
<!-- Before -->
<svg width="16" height="16" viewBox="0 0 24 24" fill="none">

<!-- After -->
<svg class="icon" viewBox="0 0 24 24" fill="none">

/* CSS */
:root {
  --icon-size: 1em;
}

.icon {
  width: var(--icon-size);
  height: var(--icon-size);
}

/* Alternative: Make it relative to font */
.icon {
  width: 1em;
  height: 1em;
}
```

## Related Components
- NewThreadButton.svelte
- NewThreadButton.module.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-NewThreadButton.svelte.md