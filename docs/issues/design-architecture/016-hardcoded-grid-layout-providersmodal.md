# Hard-coded Grid Layout not Responsive in ProvidersModal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_ProvidersModal.svelte.md
- **Location**: src-ui/src/lib/components/config/ProvidersModal.module.css:30
- **Priority**: Medium
- **Category**: Design & Architecture
- **Status**: Open

## Description
The grid uses a fixed 2-column layout which may not be optimal for all screen sizes. The current implementation hardcodes `grid-template-columns: repeat(2, 1fr)`, which doesn't adapt to different viewport sizes or content density. This can create poor user experiences on very large screens (wasted space) or very small screens (forced scrolling or cramped layouts).

## Impact
- Poor mobile/desktop responsiveness
- Inconsistent spacing across different screen sizes
- Potential layout issues on extreme screen sizes
- Wasted screen real estate on large displays
- Reduced accessibility on smaller screens

## Suggested Fix
Use CSS grid auto-fit or media queries for better responsiveness.

Option 1: CSS Grid with auto-fit
```css
.providers-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 16px;
  padding: 16px;
}

/* Or with minmax for better control */
.providers-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 300px), 1fr));
  gap: 16px;
}
```

Option 2: Media queries for different breakpoints
```css
.providers-grid {
  display: grid;
  gap: 16px;
  grid-template-columns: 1fr;
  padding: 16px;
}

@media (min-width: 640px) {
  .providers-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (min-width: 1024px) {
  .providers-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}

@media (min-width: 1280px) {
  .providers-grid {
    grid-template-columns: repeat(4, 1fr);
  }
}
```

## Related Components
- src-ui/src/lib/components/config/ProvidersModal.svelte
- src-ui/src/lib/components/config/ProvidersModal.module.css
- src-ui/src/lib/styles/design-system.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_ProvidersModal.svelte.md