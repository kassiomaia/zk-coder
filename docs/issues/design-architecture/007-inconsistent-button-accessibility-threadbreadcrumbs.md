# Inconsistent Button Accessibility in ThreadBreadcrumbs

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-code-reviewer-thread-breadcrumbs.md
- **Location**: src-ui/src/lib/components/chat/thread/ThreadBreadcrumbs.svelte:95-120
- **Priority**: Medium
- **Category**: Design & Architecture
- **Status**: Open

## Description
Buttons are not consistently accessible across all breadcrumb items. The logic for rendering buttons vs spans is inconsistent, with some breadcrumb items rendered as interactive buttons while others are non-interactive spans. This creates an inconsistent user experience and potential accessibility issues. The conditional logic for determining when to render a button vs a span appears complex and may not follow consistent accessibility patterns.

## Impact
- Inconsistent user interaction patterns
- Potential keyboard navigation issues
- Screen reader inconsistencies
- Confusing visual indicators for clickable vs non-clickable items

## Suggested Fix
Simplify the conditional logic and improve disabled state styling.

```svelte
{#each breadcrumbItems as item, index}
  <div class="breadcrumb-item" role="listitem">
    <button
      class:active={$currentThreadId === item.id}
      class:disabled={!item.isNavigable}
      aria-label={`Navigate to ${item.title}`}
      aria-disabled={!item.isNavigable}
      on:click={() => item.isNavigable && navigateToThread(item.id)}
    >
      {item.title}
    </button>

    {#if index < breadcrumbItems.length - 1}
      <span class="separator">›</span>
    {/if}
  </div>
{/each}
```

## Related Components
- src-ui/src/lib/components/chat/thread/ThreadBreadcrumbs.svelte
- src-ui/src/lib/components/chat/thread/ThreadBreadcrumbs.module.css

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-code-reviewer-thread-breadcrumbs.md