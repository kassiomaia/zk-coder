# SSR fallback HTML escape should be comprehensive

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_MarkdownRenderer.svelte.md
- **Location**: src-ui/src/lib/components/markdown/MarkdownRenderer.svelte:26-33
- **Priority**: Medium
- **Category**: Security
- **Status**: Open

## Description
The `escapeHtmlSimple` function only escapes basic HTML characters but lacks comprehensive protection against all HTML entities and potential XSS vectors.

## Impact
- Incomplete escaping allows potential XSS attacks through HTML entities
- Could allow script injection via encoded characters
- Missing protection for non-ASCII characters that could be interpreted as HTML
- No sanitization for complex HTML attributes and event handlers

## Suggested Fix
Update the escape function to handle all HTML entities, consider using a library like DOMPurify for comprehensive sanitization.

```typescript
// Comprehensive HTML escape function
function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;')
    .replace(/`/g, '&#x60;')
    // Handle additional HTML entities
    .replace(/&nbsp;/g, '&nbsp;')
    .replace(/&lt;/g, '&lt;')
    .replace(/&gt;/g, '&gt;')
    .replace(/&amp;/g, '&amp;')
    .replace(/&quot;/g, '&quot;');
}

// Better approach: Use DOMPurify for full sanitization
import DOMPurify from 'dompurify';

function sanitizeHtml(html: string): string {
  // Basic configuration for safe HTML
  return DOMPurify.sanitize(html, {
    ALLOWED_TAGS: [], // Disallow all tags
    ALLOWED_ATTR: [], // Disallow all attributes
    FORBID_TAGS: ['script', 'style', 'iframe', 'object', 'embed'],
    FORBID_ATTR: ['onerror', 'onclick', 'onload', 'javascript:']
  });
}

// SSR-safe fallback
function safeContentFallback(content: string): string {
  if (typeof window === 'undefined') {
    // SSR mode: sanitize completely
    return sanitizeHtml(content);
  }
  // Client mode: let DOMPurify handle it
  return content;
}
```

## Related Components
- MarkdownRenderer.svelte
- SSR handling system
- Content sanitization utilities

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_MarkdownRenderer.svelte.md
- DOMPurify documentation: https://github.com/cure53/DOMPurify
- OWASP XSS Prevention: https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html