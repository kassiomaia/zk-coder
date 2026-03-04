# Keyboard navigation for capabilities in ModelCard

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-ModelCard-review.md
- **Location**: src-ui/src/lib/components/config/ModelCard.svelte:43-71
- **Priority**: Medium
- **Category**: Accessibility
- **Status**: Open

## Description
While the main card is focusable, individual capability icons lack keyboard navigation support, which may limit accessibility for power users.

## Impact
This affects keyboard-only users who may need to navigate and interact with individual capability icons directly. Without proper keyboard support, these users may miss out on important features or have difficulty understanding what capabilities are available.

## Suggested Fix
Add tabindex attributes to capability spans or implement a keyboard-accessible capabilities menu

```typescript
// Add tabindex="0" to each capability span
<span class="capability" tabindex="0" role="button" aria-label="Capability: {capability.name}">
  <span class="icon" />
  <span class="name">{capability.name}</span>
</span>

// Or implement keyboard event handlers
on:keydown={(e) => {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    handleCapabilitySelect(capability);
  }
}}
```

## Related Components
- ModelCard.svelte
- Capability components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-ModelCard-review.md

## Resolution Status
**DEFERRED** - Enhancement Request - 2026-03-04

The capability icons in ModelCard are informational only (not interactive). They display status with `title` attributes for tooltips. Making them keyboard-focusable would require:

1. Adding `tabindex="0"` to each icon
2. Adding keyboard event handlers
3. Defining what action should happen when activated

Since these icons are purely informational (displaying model capabilities), adding keyboard navigation is not required for WCAG compliance. The model card itself is keyboard accessible with proper `aria-pressed` and `aria-label` attributes.

This can be revisited if capability icons become interactive in the future.