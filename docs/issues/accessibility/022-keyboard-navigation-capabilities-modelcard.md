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