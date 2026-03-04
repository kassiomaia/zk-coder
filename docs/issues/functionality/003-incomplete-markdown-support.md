# [ISSUE-003] - Incomplete Markdown Support

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-MarkdownRenderer.svelte.md`
- **Component/File**: `src-ui/src/lib/components/MarkdownRenderer.svelte`
- **Line Numbers**: TBD

## Description
The MarkdownRenderer component doesn't support all Markdown features. Tables, code blocks with syntax highlighting, and some advanced formatting features are missing or not implemented correctly.

## Impact
Functionality - This affects:
- User ability to format messages
- Rich communication capabilities
- Developer documentation display
- Content presentation quality

## Suggested Fix
Complete Markdown support:

```typescript
// Use a comprehensive markdown renderer
import { marked } from 'marked';
import { highlight } from 'highlight.js';

// Configure marked for better support
marked.setOptions({
  highlight: function(code, lang) {
    if (lang && highlight.getLanguage(lang)) {
      try {
        return highlight.highlight(code, { language: lang }).value;
      } catch (err) {
        console.error('Highlight error:', err);
      }
    }
    return highlight.highlightAuto(code).value;
  },
  breaks: true,
  gfm: true
});

function renderMarkdown(content: string) {
  return marked.parse(content);
}

// Handle different content types
$: renderedContent = $derived(() => {
  if (content.type === 'code') {
    return renderWithSyntaxHighlight(content);
  }
  return renderMarkdown(content);
});
```

## Related Issues
- None