# Conflicting Clear Button Styles in SearchBar

## Metadata
- **Source Review**: docs/reviews/code-reviewer/code-review-SearchBar.svelte-2026-03-04.md
- **Location**: src-ui/src/lib/components/chat/sidebar/SearchBar.module.css:109-134,194-220
- **Priority**: Medium
- **Category**: Design & Architecture
- **Status**: Open

## Description
The CSS file contains multiple conflicting definitions for the `.clear` button class. There are at least two separate blocks of styles for `.clear` (lines 109-134 and 194-220), each with potentially different properties, values, or selectors. This creates a CSS cascade issue where the final styling depends on the order of the rules and specificity, making it difficult to predict and maintain the consistent appearance of the clear button.

## Impact
- Inconsistent visual appearance of the clear button
- Unexpected styling behavior
- Difficult to override or modify styles
- Potential for CSS specificity conflicts
- Maintenance challenges when updating button styles

## Suggested Fix
Consolidate all `.clear` button styles into a single rule block.

```css
/* Consolidated clear button styles */
.clear {
  position: absolute;
  right: 12px;
  top: 50%;
  transform: translateY(-50%);
  width: 20px;
  height: 20px;
  border: none;
  background: transparent;
  cursor: pointer;
  color: var(--color-text-muted);
  padding: 0;
  margin: 0;
  border-radius: 50%;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
}

.clear:hover {
  background: var(--color-background-hover);
  color: var(--color-text);
}

.clear svg {
  width: 14px;
  height: 14px;
}

/* Remove duplicate or conflicting definitions */
.clear.disabled {
  opacity: 0.3;
  cursor: not-allowed;
}
```

## Related Components
- src-ui/src/lib/components/chat/sidebar/SearchBar.svelte
- src-ui/src/lib/components/chat/sidebar/SearchBar.module.css

## References
- Original review: docs/reviews/code-reviewer/code-review-SearchBar.svelte-2026-03-04.md