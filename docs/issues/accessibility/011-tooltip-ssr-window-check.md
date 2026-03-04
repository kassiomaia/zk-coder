# [ISSUE-011] - Tooltip SSR Window Check Issues

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md`
- **Component/File**: `src-ui/src/lib/components/Tooltip.svelte`
- **Line Numbers**: TBD

## Description
The Tooltip component doesn't properly handle SSR (Server-Side Rendering) when checking for window object. This causes hydration errors and breaks accessibility in SSR environments.

## Impact
Accessibility - This causes:
- Hydration errors that break component functionality
- Inaccessible tooltips in SSR environments
- Broken focus management for keyboard users
- Component failures in production builds

## Suggested Fix
Implement SSR-safe tooltip initialization:

```typescript
// Use a reactive store for browser detection
const isBrowser = derived(browser, ($browser) => $browser);

// In the component
$: tooltipVisible = isBrowser && hovered;

function showTooltip() {
  if (!isBrowser) return;
  hovered = true;
}

// Use proper ARIA attributes
<div
  class="tooltip-wrapper"
  onMouseover={showTooltip}
  onMouseout={hideTooltip}
  onFocus={showTooltip}
  onBlur={hideTooltip}
  aria-describedby="tooltip-content"
>
  <slot />
  {#if tooltipVisible}
    <div
      id="tooltip-content"
      role="tooltip"
      aria-hidden={!tooltipVisible}
    >
      {text}
    </div>
  {/if}
</div>
```

## Related Issues
- None

## Resolution Status
**RESOLVED** - False Positive - 2026-03-04

The Tooltip component already handles SSR compatibility correctly:

```typescript
const prefersReducedMotion =
  typeof window !== 'undefined' &&
  window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
```

The component has proper accessibility attributes:
- `role="tooltip"` on the tooltip element
- `aria-hidden={!visible}` for visibility state
- Focus handlers (`onfocusin`, `onfocusout`) for keyboard accessibility
- Escape key handler to dismiss tooltip