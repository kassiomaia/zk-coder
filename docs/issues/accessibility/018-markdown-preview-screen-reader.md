# Screen reader support for MarkdownPreview

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-MarkdownPreview.md
- **Location**: src-ui/src/lib/components/chat/input/MarkdownPreview.svelte:89-99
- **Priority**: Low
- **Category**: Accessibility
- **Status**: Open

## Description
While the component has basic ARIA attributes, it could be more accessible with better semantic HTML structure and keyboard navigation support. The current implementation may not provide optimal screen reader experience.

## Impact
Users with visual impairments may have difficulty navigating through the markdown content as it's rendered, potentially missing important structural elements or links within the preview.

## Suggested Fix
Add proper heading hierarchy, ensure keyboard navigation works, and add more descriptive ARIA labels.

```html
<!-- Fix example -->
<div
  class="markdown-preview"
  role="article"
  aria-label="Markdown preview"
  tabindex="0"
  on:keydown={handleKeydown}
>
  {#if content}
    <!-- Ensure proper heading hierarchy -->
    <div class="preview-container">
      {@html renderedContent}

      <!-- Add landmark regions for navigation -->
      <nav class="preview-toc" aria-label="Preview navigation">
        <!-- Table of contents elements if any -->
      </nav>
    </div>
  {:else}
    <div class="empty-preview" aria-live="polite">
      No content to preview
    </div>
  {/if}
</div>
```

```javascript
// Enhanced keyboard navigation
function handleKeydown(event) {
  if (event.key === 'Tab') {
    // Allow normal tab navigation
    return;
  }

  // Add keyboard shortcuts for navigation if needed
  if (event.key === 'f' && event.ctrlKey) {
    event.preventDefault();
    focusFirstHeading();
  }
}
```

## Related Components
- MarkdownPreview.svelte
- MarkdownRenderer.svelte
- ChatInput.svelte

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-MarkdownPreview.md
- WAI-ARIA Authoring Practices: https://www.w3.org/WAI/ARIA/apg/