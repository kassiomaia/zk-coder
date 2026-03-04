# Performance issues with large content in MarkdownPreview

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-MarkdownPreview.md
- **Location**: src-ui/src/lib/components/chat/input/MarkdownPreview.svelte:18
- **Priority**: Medium
- **Category**: Performance
- **Status**: Open

## Description
The parseMarkdown function uses multiple regex replacements on the entire content string, which can cause performance issues with large markdown documents.

## Impact
When processing large markdown documents, the current implementation performs multiple regex replacements sequentially on the entire content string. This can lead to:
- Poor performance with large documents
- UI lag during markdown rendering
- Increased memory usage
- Potential browser thread blocking

## Suggested Fix
Use a proper markdown parser that implements efficient parsing algorithms, or implement streaming/chunking for large content.

```javascript
// Example of optimized approach using a markdown parser
import { marked } from 'marked';

function parseMarkdown(content) {
  // Use a proper markdown parser for better performance
  return marked(content);
}

// Alternative: Implement streaming for very large content
function parseMarkdownLarge(content) {
  if (content.length > 10000) {
    // Process in chunks for large content
    return processInChunks(content);
  }
  return parseMarkdown(content);
}
```

## Related Components
- MarkdownPreview.svelte
- Chat input components

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-MarkdownPreview.md