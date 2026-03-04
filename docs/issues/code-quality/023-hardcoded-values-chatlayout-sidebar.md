# Hardcoded values in ChatLayout sidebar

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ChatLayout.svelte_2026-03-04.md
- **Location**: src-ui/src/lib/components/layout/ChatLayout.module.css:207-208
- **Priority**: Low
- **Category**: Code Quality
- **Status**: Open

## Description
The sidebar width is hardcoded to 85% with a max-width of 300px in mobile layout. These values should be configurable through CSS variables.

## Impact
Hardcoded CSS values make it difficult to adapt the layout for different screen sizes and design requirements. It also violates the design system's consistency principles.

## Suggested Fix
Replace hardcoded values with CSS variables like `--sidebar-width-mobile` and `--sidebar-max-width-mobile`.

```css
/* In design-system.css */
:root {
  --sidebar-width-mobile: 85%;
  --sidebar-max-width-mobile: 300px;
}

/* In ChatLayout.module.css */
@media (max-width: 768px) {
  .sidebar {
    width: var(--sidebar-width-mobile);
    max-width: var(--sidebar-max-width-mobile);
  }
}
```

## Related Components
- src-ui/src/lib/components/layout/ChatLayout.module.css
- src-ui/src/lib/styles/design-system.css

## References
- Original review: docs/reviews/code-reviewer/ChatLayout.svelte_2026-03-04.md