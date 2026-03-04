# [ISSUE-002] - Conflicting Icon Styles in Search Bar

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/code-review-SearchBar.svelte-2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/SearchBar.svelte`
- **Line Numbers**: TBD

## Description
The SearchBar component has conflicting icon styles from different sources. Icons have inconsistent sizing, colors, and spacing, creating a visual and code quality issue.

## Impact
Code Quality - This affects:
- Visual inconsistencies
- Difficult maintenance
- Confusing code structure
- Reduced user experience

## Suggested Fix
Create a unified icon styling system:

```typescript
// Define icon variants in design system
const iconVariants = {
  search: {
    size: '20px',
    color: 'currentColor',
    weight: 'regular'
  },
  clear: {
    size: '16px',
    color: 'var(--color-gray-500)',
    weight: 'bold'
  }
};

// Use consistent icon component
<Icon
  name="search"
  class="search-icon"
  style={{
    width: iconVariants.search.size,
    height: iconVariants.search.size,
    color: iconVariants.search.color
  }}
/>

// Or use CSS classes
.search-icon {
  width: var(--icon-size-md);
  height: var(--icon-size-md);
  color: var(--color-text-secondary);
}
```

## Related Issues
- None