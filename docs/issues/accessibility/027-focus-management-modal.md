# Focus management in Modal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/Modal.svelte_2026-03-04.md
- **Location**: src-ui/src/lib/components/shared/Modal.svelte:95
- **Priority**: Medium
- **Category**: Accessibility
- **Status**: Open

## Description
Modal doesn't implement proper focus trapping when opened, which can cause accessibility issues for keyboard users.

## Impact
Without focus trapping, keyboard users may accidentally navigate outside the modal or have difficulty finding the focus when using the keyboard. This can lead to a confusing and inaccessible experience.

## Suggested Fix
Implement focus trapping by managing focus within the modal when it opens and returning focus to the trigger element when closed.

```typescript
<!-- Modal.svelte with focus management -->
<script>
  import { focusTrap } from 'svelte-focus-trap';
  import { afterUpdate } from 'svelte';

  let modal: HTMLElement;
  let triggerElement: HTMLElement;
  const { active = false } = $props();

  // Store trigger element for focus return
  $: if (active) {
    triggerElement = document.activeElement as HTMLElement;
  }

  // Focus trap when modal becomes active
  $: if (active) {
    focusTrap(modal);
    // Focus first focusable element
    modal.focus();
  }

  // Return focus to trigger when modal closes
  afterUpdate(() => {
    if (!active && triggerElement) {
      setTimeout(() => triggerElement.focus(), 0);
    }
  });
</script>

<div
  bind:this={modal}
  role="dialog"
  aria-modal="true"
  class:active
>
  <!-- Modal content -->
</div>
```

## Alternative Implementation
For cases without external libraries:

```typescript
<script>
  let modal: HTMLElement;
  const { active = false } = $props();

  function getFocusableElements(element: HTMLElement): HTMLElement[] {
    return Array.from(element.querySelectorAll(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    ));
  }

  $: if (active) {
    const focusable = getFocusableElements(modal);
    if (focusable.length > 0) {
      focusable[0].focus();
    }
  }
</script>

<!-- Tab index handling for last/first element -->
```

## Related Components
- Modal.svelte
- Focus trap utilities

## References
- Original review: docs/reviews/code-reviewer/Modal.svelte_2026-03-04.md
- Focus Management: https://www.w3.org/WAI/ARIA/apg/practices/focus-management/
- WAI-ARIA Authoring Practices: https://www.w3.org/WAI/ARIA/apg/patterns/dialog/