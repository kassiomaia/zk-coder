# Focus management in Popover

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-Popover-Review.md
- **Location**: src-ui/src/lib/components/shared/Popover.svelte:62-87
- **Priority**: Low
- **Category**: Accessibility
- **Status**: Open

## Description
No focus management for accessibility when the popover opens. Should focus on the first or last item depending on keyboard navigation direction.

## Impact
Without proper focus management, keyboard users may have difficulty navigating popover content and may not know which element is currently focused, leading to a poor user experience.

## Suggested Fix
Add focus management in the open state and handle arrow keys for navigation.

```typescript
<!-- Popover.svelte with focus management -->
<script>
  import { afterUpdate } from 'svelte';

  let popover: HTMLElement;
  let content: HTMLElement;
  const { open = false } = $props();

  // Track keyboard navigation direction
  let lastAction: 'click' | 'keydown' = 'click';
  let focusedIndex = -1;

  function handleKeyDown(e: KeyboardEvent) {
    const items = content?.querySelectorAll('[tabindex="0"]') || [];

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      lastAction = 'keydown';
      focusedIndex = (focusedIndex + 1) % items.length;
      items[focusedIndex]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      lastAction = 'keydown';
      focusedIndex = (focusedIndex - 1 + items.length) % items.length;
      items[focusedIndex]?.focus();
    } else if (e.key === 'Escape') {
      // Close popover
    }
  }

  $: if (open) {
    // Focus first element by default, or last if keyboard navigation
    const items = content?.querySelectorAll('[tabindex="0"]') || [];
    if (items.length > 0) {
      focusedIndex = lastAction === 'keydown' ? items.length - 1 : 0;
      items[focusedIndex]?.focus();
    }
  }
</script>

<div class="popover" bind:this={popover}>
  <div class="trigger" on:click={() => { lastAction = 'click'; open = !open; }}>
    <slot name="trigger" />
  </div>

  {#if open}
    <div
      class="content"
      bind:this={content}
      on:keydown={handleKeyDown}
      role="menu"
    >
      <slot />
    </div>
  {/if}
</div>
```

## Related Components
- Popover.svelte
- Menu components
- Keyboard navigation utilities

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-Popover-Review.md
- WAI-ARIA Authoring Practices: https://www.w3.org/WAI/ARIA/apg/patterns/menu/
- Keyboard Navigation: https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/

## Resolution Status
**DEFERRED** - Enhancement Request - 2026-03-04

The current Popover implementation:
- Has `role="menu"` and `role="menuitem"` for ARIA semantics
- Handles Escape key to close
- Uses native `<button>` elements which are keyboard accessible

For improved accessibility, focus management could be added. However:
1. Items are buttons and can be navigated with Tab
2. The menu closes on click or Escape
3. The implementation follows basic ARIA menu patterns

Future enhancement: Implement arrow key navigation and automatic focus on open.