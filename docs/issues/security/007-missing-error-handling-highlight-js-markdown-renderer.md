# Missing error handling for highlight.js in MarkdownRenderer

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_MarkdownRenderer.svelte.md
- **Location**: src-ui/src/lib/utils/markdown.ts:45-56
- **Priority**: Low
- **Category**: Security
- **Status**: Open

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