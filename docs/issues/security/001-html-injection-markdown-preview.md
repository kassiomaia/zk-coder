# [ISSUE-001] - HTML Injection Vulnerability in Markdown Preview

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer-ConfigModal.svelte-2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/MarkdownPreview.svelte`
- **Line Numbers**: TBD

## Description
The markdown preview component may be vulnerable to HTML injection attacks. Currently, it doesn't properly sanitize HTML content before rendering, which could allow malicious HTML/JavaScript to be executed in the user's browser.

## Impact
Security - This vulnerability could allow attackers to execute arbitrary JavaScript code in the application context, potentially leading to:
- Cross-site scripting (XSS) attacks
- User session hijacking
- Data theft
- Malicious script injection

## Suggested Fix
Implement proper HTML sanitization before rendering markdown content:

```typescript
import DOMPurify from 'dompurify';

// In the component:
$: safeContent = DOMPurify.sanitize(markdownContent);
```

Alternatively, use a markdown renderer that automatically sanitizes HTML, such as marked with sanitize options.

## Related Issues
- None

## Proposed Solution

### Root Cause Analysis
The `parseMarkdown` function in `MarkdownPreview.svelte` has two security concerns:

1. **Unsafe Link Handling (Primary Vulnerability)**: The link regex uses the URL directly in the `href` attribute without validation, allowing dangerous protocols like `javascript:`, `data:`, and `vbscript:`.

2. **Missing Defense-in-Depth**: While basic HTML escaping is performed, there's no explicit sanitization library as a final safety net to catch edge cases.

### Implementation Plan

#### Step 1: Install DOMPurify
```bash
cd src-ui && npm install dompurify && npm install -D @types/dompurify
```

#### Step 2: Update MarkdownPreview.svelte

1. **Add URL sanitization function** to block dangerous protocols:
```typescript
function sanitizeUrl(url: string): string {
	const trimmed = url.trim().toLowerCase();
	const dangerousProtocols = ['javascript:', 'data:', 'vbscript:', 'file:'];
	const safeProtocols = ['http:', 'https:', 'mailto:'];

	// Check for dangerous protocols
	if (dangerousProtocols.some(p => trimmed.startsWith(p))) {
		return '#';
	}

	// Allow relative URLs and safe protocols
	if (trimmed.startsWith('/') || trimmed.startsWith('#') || trimmed.startsWith('?')) {
		return url;
	}

	// Check if it starts with a safe protocol
	if (safeProtocols.some(p => trimmed.startsWith(p))) {
		return url;
	}

	// Default case: treat as relative URL but sanitize
	return url;
}
```

2. **Apply DOMPurify sanitization** on the final rendered HTML:
```typescript
import DOMPurify from 'dompurify';

// In parseMarkdown function, before return:
return DOMPurify.sanitize(html, {
	ALLOWED_TAGS: ['p', 'br', 'strong', 'em', 'del', 'h1', 'h2', 'h3', 'ul', 'ol', 'li', 'blockquote', 'hr', 'a', 'pre', 'code'],
	ALLOWED_ATTR: ['class', 'href', 'target', 'rel', 'data-lang'],
	ALLOW_DATA_ATTR: false
});
```

3. **Update link handling** to use sanitized URLs:
```typescript
html = html.replace(
	/\[([^\]]+)\]\(([^)]+)\)/g,
	(_, text, url) => {
		const safeUrl = sanitizeUrl(url);
		return `<a href="${safeUrl}" class="${styles.link}" target="_blank" rel="noopener noreferrer">${text}</a>`;
	}
);
```

### Testing Plan
1. Test with `javascript:` URLs - should be neutralized
2. Test with `data:` URLs - should be neutralized
3. Test with normal `https:` URLs - should work normally
4. Test with relative URLs - should work normally
5. Verify existing markdown rendering still works correctly

## Resolution Status
DONE

## Resolution Summary
Implemented a two-layer security fix:
1. **URL Sanitization**: Added `sanitizeUrl()` function to block dangerous protocols (`javascript:`, `data:`, `vbscript:`, `file:`)
2. **DOMPurify Sanitization**: Applied DOMPurify as defense-in-depth to sanitize final HTML output

### Files Changed
- `src-ui/package.json` - Added dompurify and @types/dompurify dependencies
- `src-ui/src/lib/components/chat/input/MarkdownPreview.svelte` - Implemented sanitization

### Commit
fix(#001): prevent HTML injection in MarkdownPreview component