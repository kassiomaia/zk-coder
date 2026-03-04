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

## Proposed Solution

### Analysis
After reviewing the codebase, the `markdown.ts` utility already has a more comprehensive `escapeHtml` function that handles quotes. The best approach is to:

1. **Export and reuse** the existing `escapeHtml` function from `markdown.ts`
2. **Enhance** it slightly to include backtick (`` ` ``) and forward slash (`/`) escaping
3. **Update** `MarkdownRenderer.svelte` to use the shared utility instead of its local implementation

### Implementation Plan

#### Step 1: Enhance the `escapeHtml` function in `markdown.ts`
Add backtick and forward slash escaping to make it comprehensive:
```typescript
export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;')
    .replace(/`/g, '&#x60;')  // Backtick for template literals
    .replace(/\//g, '&#x2F;'); // Forward slash for </script> prevention
}
```

#### Step 2: Update `MarkdownRenderer.svelte`
Import and use the shared `escapeHtml` function:
```typescript
import { renderMarkdown, escapeHtml } from '$lib/utils/markdown';

// Replace the local escapeHtmlSimple with the imported escapeHtml
let renderedHtml = $derived(browser ? renderMarkdown(content) : escapeHtml(content).replace(/\n/g, '<br>'));
```

### Rationale
- **Reusability**: Leverages existing tested code
- **No new dependencies**: Avoids adding DOMPurify (DOMPurify is overkill for this SSR fallback use case)
- **Consistency**: Uses the same escaping logic throughout the codebase
- **Comprehensive**: Covers all major XSS vectors including quotes, backticks, and forward slashes

## Resolution Status
- [x] In Progress
- [x] Done

## Implementation Summary

### Changes Made

1. **Enhanced `escapeHtml` function in `markdown.ts`** (src-ui/src/lib/utils/markdown.ts)
   - Added backtick escaping (`` ` `` -> `&#x60;`) to prevent template literal injection
   - Added forward slash escaping (`/` -> `&#x2F;`) to prevent `</script>` attacks
   - Added comprehensive documentation explaining each escape
   - Function is now exported for reuse across the codebase

2. **Updated `MarkdownRenderer.svelte`** (src-ui/src/lib/components/markdown/MarkdownRenderer.svelte)
   - Removed the local `escapeHtmlSimple` function
   - Imported the enhanced `escapeHtml` function from `markdown.ts`
   - Updated the SSR fallback to use the comprehensive escaping
   - Added clarifying comment about the escaping

### XSS Vectors Now Protected
- `&` -> `&amp;` (must be first to prevent double-escaping)
- `<` -> `&lt;` (HTML tag injection)
- `>` -> `&gt;` (HTML tag injection)
- `"` -> `&quot;` (attribute breakout)
- `'` -> `&#039;` (attribute breakout with single quotes)
- `` ` `` -> `&#x60;` (template literal injection)
- `/` -> `&#x2F;` (prevents `</script>` attacks)

### Files Modified
- `/home/kassiomaia/devel/zk/coder/src-ui/src/lib/utils/markdown.ts`
- `/home/kassiomaia/devel/zk/coder/src-ui/src/lib/components/markdown/MarkdownRenderer.svelte`