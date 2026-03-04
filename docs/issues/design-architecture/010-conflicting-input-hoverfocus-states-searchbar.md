# Conflicting Input Hover/Focus States in SearchBar

## Metadata
- **Source Review**: docs/reviews/code-reviewer/code-review-SearchBar.svelte-2026-03-04.md
- **Location**: src-ui/src/lib/components/chat/sidebar/SearchBar.module.css:49-59,74-83,90-92,170-178
- **Priority**: Medium
- **Category**: Design & Architecture
- **Status**: Open

## Description
The CSS file contains multiple conflicting rules for `.input:hover` and `.input:focus` states. There are multiple definitions for these states in different parts of the file, with potentially different properties and values. This creates a CSS cascade issue where which styles take effect depends on the order they appear in the file and the specificity of the selectors.

## Impact
- Unpredictable styling behavior
- Visual inconsistencies in the search input
- Difficult to debug styling issues
- Potential for CSS specificity conflicts
- Inconsistent user experience across different states

## Suggested Fix
Consolidate the hover and focus state rules into a single coherent set of styles.

```css
/* Consolidate all input states */
.input {
  /* Base styles */
  background: var(--color-background);
  border: 1px solid var(--color-border);
  padding: 8px 12px;
  border-radius: 6px;
  transition: all 0.2s ease;
}

.input:hover {
  background: var(--color-background-hover);
  border-color: var(--color-border-hover);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.input:focus {
  background: var(--color-background-focus);
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-light);
  outline: none;
}

/* Remove conflicting duplicate rules from other sections */
```

## Related Components
- src-ui/src/lib/components/chat/sidebar/SearchBar.svelte
- src-ui/src/lib/components/chat/sidebar/SearchBar.module.css

## References
- Original review: docs/reviews/code-reviewer/code-review-SearchBar.svelte-2026-03-04.md