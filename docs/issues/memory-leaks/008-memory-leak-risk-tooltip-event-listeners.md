# Issue 008: Memory leak risk in Tooltip event listeners

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md
- **Location**: src-ui/src/lib/components/shared/Tooltip.svelte:282-295
- **Priority**: Medium
- **Category**: Memory Leaks
- **Status**: Resolved

## Description
The scroll and resize event listeners are added in an $effect but there's a potential race condition when the component unmounts quickly after becoming visible.

## Impact
When the Tooltip component unmounts quickly after becoming visible:
- The $effect cleanup might not run in time
- Event listeners (scroll, resize) remain attached to the window
- Each tooltip creation adds more listeners without proper cleanup
- This leads to memory accumulation
- Can cause performance degradation and unexpected behavior

## Resolution Status
**RESOLVED - FALSE POSITIVE**

After code review, the Tooltip component already has proper cleanup implemented (lines 282-295):

```typescript
$effect(() => {
  if (visible) {
    const update = () => {
      requestAnimationFrame(calculatePosition);
    };
    window.addEventListener('scroll', update, { capture: true, passive: true });
    window.addEventListener('resize', update, { passive: true });

    return () => {
      window.removeEventListener('scroll', update, { capture: true });
      window.removeEventListener('resize', update);
    };
  }
});
```

The `$effect` properly returns a cleanup function that removes the event listeners. Svelte 5's `$effect` ensures cleanup runs when the component is destroyed or when `visible` becomes false.

No changes were needed.

## Related Components
- Tooltip.svelte
- MessageActions.svelte
- ChatMessage.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md