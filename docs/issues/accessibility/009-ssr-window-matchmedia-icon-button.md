# [ISSUE-009] - SSR Window MatchMedia Error in IconButton

## Priority
High

## Source
- **Review File**: `docs/reviews/Code Reviewer/2026-03-04-153045-IconButton.svelte-review.md`
- **Component/File**: `src-ui/src/lib/components/IconButton.svelte`
- **Line Numbers**: TBD

## Description
The IconButton component uses window.matchMedia without checking for SSR (Server-Side Rendering) environment. This causes hydration mismatches and accessibility issues during server rendering.

## Impact
Accessibility - This causes:
- Hydration errors that break accessibility
- Failing accessibility audits
- Inconsistent DOM structures between server and client
- Broken button functionality in some environments

## Suggested Fix
Handle SSR compatibility for media queries:

```typescript
// Use a reactive store for SSR-safe media queries
const prefersReducedMotion = derived(
  browser,
  ($browser) => $browser ? window.matchMedia('(prefers-reduced-motion: reduce)').matches : false
);

// In the component
$: iconVariant = prefersReducedMotion ? 'static' : 'animated';

// Also check for browser availability
function handleClick() {
  if (!browser) return;
  // Handle click
}
```

## Related Issues
- None