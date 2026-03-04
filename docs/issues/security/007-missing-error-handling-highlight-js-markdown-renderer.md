# Missing error handling for highlight.js in MarkdownRenderer

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_MarkdownRenderer.svelte.md
- **Location**: src-ui/src/lib/utils/markdown.ts:45-76
- **Priority**: Low
- **Category**: Security
- **Status**: DONE

## Description
While there is try-catch for highlight.js, it silently fails and falls back to plain text without logging the error.

## Impact
- Difficult debugging for syntax highlighting issues
- No visibility into highlighting failures during development
- Could miss potential security-related highlighting errors
- Silent failures may hide underlying problems with code blocks

## Suggested Fix
Implement proper error logging for highlighting failures while still providing the fallback.

```typescript
// Enhanced highlight function with proper error handling
async function highlightCode(language: string, code: string): Promise<string> {
  try {
    // Check if language is supported
    if (!isValidLanguage(language)) {
      console.warn(`Unsupported language: ${language}`);
      return escapeHtml(code);
    }

    // Try to highlight the code
    const highlighted = await hljs.highlight(code, { language }).value;

    // Log success for debugging
    console.debug(`Successfully highlighted ${language} code`);

    return highlighted;
  } catch (error) {
    // Log the error with details
    console.error('Code highlighting failed:', {
      language,
      codePreview: code.substring(0, 100),
      error: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined
    });

    // Provide fallback
    console.warn('Falling back to plain text for:', language);
    return escapeHtml(code);
  }
}

// Language validation
function isValidLanguage(language: string): boolean {
  // List of supported languages
  const supportedLanguages = hljs.listLanguages();

  // Check if the language is supported
  if (supportedLanguages.includes(language)) {
    return true;
  }

  // Check aliases
  const languageInfo = hljs.getLanguage(language);
  return languageInfo !== undefined;
}

// Enhanced escape function
function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

// Usage in markdown processing
async function processCodeBlocks(markdown: string): Promise<string> {
  const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g;

  return markdown.replace(codeBlockRegex, async (match, language, code) => {
    const highlighted = await highlightCode(language || 'plaintext', code);
    return `<pre><code class="hljs language-${language || 'plaintext'}">${highlighted}</code></pre>`;
  });
}
```

## Related Components
- MarkdownRenderer.svelte
- markdown.ts utility
- Syntax highlighting system

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_MarkdownRenderer.svelte.md
- highlight.js Documentation: https://highlightjs.readthedocs.io/en/latest/
- Error Logging Best Practices: https://developer.mozilla.org/en-US/docs/Web/API/console#error

## Proposed Solution

The fix involves adding proper error logging to the existing try-catch blocks in the `code` renderer function while maintaining the fallback behavior.

### Changes to `src-ui/src/lib/utils/markdown.ts`

Replace the existing `code` function (lines 39-61) with an enhanced version that logs errors:

```typescript
code(token: any): string | false {
    const code = token.text || '';
    const lang = token.lang || '';
    let highlighted: string;

    if (lang && hljs.getLanguage(lang)) {
        try {
            highlighted = hljs.highlight(code, { language: lang }).value;
        } catch (error) {
            // Log highlighting error with details for debugging
            console.warn('[Markdown] Code highlighting failed for language:', lang, {
                error: error instanceof Error ? error.message : String(error),
                codePreview: code.substring(0, 100) + (code.length > 100 ? '...' : '')
            });
            highlighted = escapeHtml(code);
        }
    } else {
        try {
            highlighted = hljs.highlightAuto(code).value;
        } catch (error) {
            // Log auto-detection error for debugging
            console.warn('[Markdown] Auto-detection highlighting failed:', {
                error: error instanceof Error ? error.message : String(error),
                codePreview: code.substring(0, 100) + (code.length > 100 ? '...' : '')
            });
            highlighted = escapeHtml(code);
        }
    }

    const langAttr = lang ? ` data-language="${escapeHtml(lang)}"` : '';
    return `<pre class="code-block"${langAttr}><code>${highlighted}</code></pre>`;
}
```

### Key Points
1. Uses `console.warn` instead of `console.error` since these are recoverable errors with fallback
2. Includes `[Markdown]` prefix for easy filtering in console
3. Logs the error message and a preview of the code (truncated to 100 chars)
4. Maintains the existing fallback behavior to escaped HTML
5. No breaking changes to the API or functionality

### Testing
1. Test with valid code blocks to ensure highlighting still works
2. Test with invalid/unsupported languages to verify fallback and logging
3. Test with code that might cause highlight.js to throw errors

## Resolution Status
- **Status**: DONE
- **Started**: 2026-03-04
- **Completed**: 2026-03-04
- **Assignee**: Issue Resolver Agent

## Lessons Learned
1. Silent error handling in try-catch blocks can hide important debugging information
2. Using `console.warn` for recoverable errors with fallback is appropriate - it alerts developers without breaking the app
3. Including context (language, code preview) in error logs helps diagnose issues faster
4. Using a consistent prefix like `[Markdown]` makes it easy to filter logs in the console