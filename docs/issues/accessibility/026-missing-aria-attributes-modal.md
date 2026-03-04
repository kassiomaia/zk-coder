# Missing ARIA attributes in Modal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/Modal.svelte_2026-03-04.md
- **Location**: src-ui/src/lib/components/shared/Modal.svelte:92-94
- **Priority**: Medium
- **Category**: Accessibility
- **Status**: Open

## Description
Modal lacks proper ARIA attributes for screen readers, particularly missing aria-describedby for connecting modal content to its description.

## Impact
Screen reader users may have difficulty understanding the modal's purpose and content without proper ARIA attributes to describe the modal and its relationship to other elements.

## Suggested Fix
Add aria-describedby when a content description is available and ensure proper role attributes for nested content.

```typescript
<!-- Modal.svelte -->
<div
  role="dialog"
  aria-modal="true"
  aria-labelledby={titleId}
  aria-describedby={descriptionId}
  class:active={active}
>
  {#if title}
    <h2 id={titleId}>{title}</h2>
  {/if}

  {#if description}
    <p id={descriptionId}>{description}</p>
  {/if}

  <div class="content">
    <slot />
  </div>

  <div class="actions">
    <!-- Action buttons -->
  </div>
</div>

<script>
  import { createId } from 'svelte/elements';

  const titleId = createId();
  const descriptionId = createId();

  const { active = false, title, description } = $props();
</script>
```

## Related Components
- Modal.svelte
- Dialog components

## References
- Original review: docs/reviews/code-reviewer/Modal.svelte_2026-03-04.md
- ARIA Dialog Pattern: https://www.w3.org/WAI/ARIA/apg/patterns/dialog/
- WAI-ARIA Authoring Practices: https://www.w3.org/WAI/ARIA/apg/