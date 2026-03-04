# Hardcoded colors in ProviderIcon

## Metadata
- **Source Review**: docs/reviews/code-reviewer/code-review-providericon-2026-03-04.md
- **Location**: ProviderIcon.svelte:16-20,22
- **Priority**: Medium
- **Category**: Code Quality
- **Status**: Open

## Description
The colorMap uses hardcoded rgba values instead of CSS variables from the design system, making it inconsistent with the application's design language.

## Impact
Hardcoded colors in the provider icons prevent consistent theming and make it difficult to adjust colors for different themes or accessibility requirements. This creates visual inconsistencies and breaks the unified design system.

## Suggested Fix
Use CSS variables from the design system for colors. Define provider-specific colors as tokens in the design system.

```typescript
// Before
const colorMap = {
  'openai': 'rgba(0, 0, 0, 0.9)',
  'anthropic': 'rgba(0, 123, 255, 0.9)',
  // ...
};

// After
// Define in design-system.css
:root {
  --provider-color-openai: var(--color-gray-900);
  --provider-color-anthropic: var(--color-blue-500);
}

// In component
const colorMap = {
  'openai': 'var(--provider-color-openai)',
  'anthropic': 'var(--provider-color-anthropic)',
  // ...
};
```

## Related Components
- ProviderIcon.svelte
- design-system.css

## References
- Original review: docs/reviews/code-reviewer/code-review-providericon-2026-03-04.md