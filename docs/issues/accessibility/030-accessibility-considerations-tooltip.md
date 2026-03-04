# Accessibility considerations for Tooltip

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md
- **Location**: src-ui/src/lib/components/shared/Tooltip.svelte:328-330
- **Priority**: Low
- **Category**: Accessibility
- **Status**: Open

## Description
While basic ARIA attributes are present, the component could benefit from additional accessibility features like live regions for dynamic content.

## Impact
Tooltip components often contain important contextual information that may not be properly communicated to screen reader users, especially when the content changes dynamically.

## Suggested Fix
Consider adding aria-live="polite" for dynamic content and proper keyboard navigation support.

```typescript
<!-- Tooltip.svelte enhanced for accessibility -->
<script>
  import { createId } from 'svelte/elements';

  let tooltip: HTMLElement;
  const { content, visible = false, position = 'top' } = $props();
  const tooltipId = createId();
</script>

<div class="tooltip-container">
  <slot />

  {#if visible}
    <div
      class="tooltip"
      bind:this={tooltip}
      id={tooltipId}
      role="tooltip"
      aria-hidden={!visible}
    >
      {#if typeof content === 'string'}
        <span>{content}</span>
      {:else}
        <div aria-live="polite">
          <slot name="content" />
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* Ensure tooltip is keyboard accessible */
  .tooltip-container:hover .tooltip,
  .tooltip-container:focus-within .tooltip {
    visibility: visible;
    opacity: 1;
  }

  /* Additional focus styles */
  .tooltip:focus {
    outline: 2px solid var(--color-primary-600);
    outline-offset: 2px;
  }
</style>
```

## Additional Enhancements

### For Dynamic Content:
```typescript
<script>
  let dynamicContent = '';
  $: {
    if (visible && dynamicContent) {
      // Announce dynamic content changes to screen readers
      const announcement = document.createElement('div');
      announcement.setAttribute('role', 'status');
      announcement.setAttribute('aria-live', 'polite');
      announcement.textContent = dynamicContent;
      document.body.appendChild(announcement);

      // Remove after announcement
      setTimeout(() => announcement.remove(), 1000);
    }
  }
</script>
```

### Keyboard Support:
```typescript
<!-- Add keyboard trigger support -->
<div
  class="tooltip-trigger"
  tabindex="0"
  aria-describedby={tooltipId}
  on:keydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      visible = !visible;
    }
  }}
>
  <slot name="trigger" />
</div>
```

## Related Components
- Tooltip.svelte
- Help text components
- Contextual information components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md
- WAI-ARIA Tooltips: https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/
- ARIA Live Regions: https://www.w3.org/WAI/ARIA/apg/practices/aria-live/