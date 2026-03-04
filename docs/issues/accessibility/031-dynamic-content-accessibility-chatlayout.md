# Dynamic content accessibility in ChatLayout

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ChatLayout.svelte_2026-03-04.md
- **Location**: src-ui/src/lib/components/layout/ChatLayout.svelte:196-204
- **Priority**: Low
- **Category**: Accessibility
- **Status**: Open

## Description
The header title changes based on selected thread/conversation but doesn't have proper aria attributes to screen readers about the current context.

## Impact
Screen reader users may not be aware of when the conversation context changes, leading to confusion about which thread or conversation they're currently viewing or interacting with.

## Suggested Fix
Add appropriate aria attributes with role="status" and aria-live="polite".

```typescript
<!-- ChatLayout.svelte -->
<script>
  import { createId } from 'svelte/elements';

  let headerTitle: HTMLElement;
  const { selectedThread, threads } = $props();

  // Create unique IDs for accessibility
  const titleId = createId();
  const statusId = createId();

  // Track for screen reader announcements
  $: {
    if (selectedThread) {
      // Announce thread change to screen readers
      const announcement = document.createElement('div');
      announcement.setAttribute('role', 'status');
      announcement.setAttribute('aria-live', 'polite');
      announcement.setAttribute('id', statusId);
      announcement.textContent = `Switched to conversation: ${selectedThread.title}`;
      document.body.appendChild(announcement);

      // Clean up after announcement
      setTimeout(() => {
        const oldAnnouncement = document.getElementById(statusId);
        if (oldAnnouncement) {
          oldAnnouncement.remove();
        }
      }, 1000);
    }
  }
</script>

<header>
  <h1 id={titleId} class="header-title">
    <span role="status" aria-live="polite">
      {selectedThread ? selectedThread.title : 'New Conversation'}
    </span>
  </h1>

  {#if selectedThread}
    <div
      class="thread-meta"
      aria-label="Thread information"
    >
      <span aria-hidden="true">•</span>
      <span aria-hidden="true">{selectedThread.messages.length} messages</span>
    </div>
  {/if}
</header>

<style>
  /* Ensure heading is visible to screen readers */
  h1 {
    position: relative;
  }

  /* Visual indication of active state */
  .header-title::after {
    content: '';
    position: absolute;
    bottom: -2px;
    left: 0;
    width: 100%;
    height: 2px;
    background: var(--color-primary-600);
    transform: scaleX(0);
    transition: transform 0.3s ease;
  }

  h1:focus-within::after {
    transform: scaleX(1);
  }
</style>
```

## Alternative Implementation with Svelte Actions

```typescript
// chat-layout-accessibility.svelte
<script>
  import { createId } from 'svelte/elements';

  let title: HTMLElement;
  const { selectedThread } = $props();

  function announceChange(element: HTMLElement, message: string) {
    const liveRegion = document.createElement('div');
    liveRegion.setAttribute('role', 'status');
    liveRegion.setAttribute('aria-live', 'polite');
    liveRegion.style.position = 'absolute';
    liveRegion.style.left = '-10000px';
    liveRegion.style.width = '1px';
    liveRegion.style.height = '1px';
    liveRegion.style.overflow = 'hidden';
    liveRegion.textContent = message;

    document.body.appendChild(liveRegion);
    setTimeout(() => liveRegion.remove(), 1000);
  }

  $: if (selectedThread) {
    announceChange(title, `Switched to conversation: ${selectedThread.title}`);
  }
</script>

<h1
  bind:this={title}
  role="heading"
  aria-level="1"
  aria-describedby="thread-status"
>
  {selectedThread ? selectedThread.title : 'New Conversation'}
  <span id="thread-status" class="sr-only" aria-live="polite" />
</h1>
```

## Related Components
- ChatLayout.svelte
- Thread components
- Header components

## References
- Original review: docs/reviews/code-reviewer/ChatLayout.svelte_2026-03-04.md
- ARIA Live Regions: https://www.w3.org/WAI/ARIA/apg/practices/aria-live/
- Dynamic Content Updates: https://www.w3.org/WAI/WCAG21/Techniques/general/G105

## Resolution Status
**DEFERRED** - Enhancement Request - 2026-03-04

The ChatLayout component displays thread titles and information. The suggested `aria-live` enhancement would improve the experience for screen reader users when switching threads.

Current implementation:
- ThreadBreadcrumbs component has proper navigation semantics
- StatusBadge component has `aria-label` for status information

Future enhancement: Add `aria-live="polite"` to the header title area to announce thread changes to screen readers.

```svelte
<h1 aria-live="polite">
  {selectedThread ? selectedThread.title : 'New Conversation'}
</h1>
```