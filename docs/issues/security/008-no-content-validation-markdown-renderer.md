# No content validation in MarkdownRenderer

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_MarkdownRenderer.svelte.md
- **Location**: src-ui/src/lib/components/markdown/MarkdownRenderer.svelte:17
- **Priority**: Low
- **Category**: Security
- **Status**: Open

## Description
The component accepts any string content without validation, potentially allowing malicious content to pass through.

## Impact
- No protection against malicious markdown content
- Potential for XSS through crafted markdown
- Could render unsafe HTML if markdown is processed unsafely
- No content length limits could lead to DoS attacks
- No schema validation for expected content types

## Suggested Fix
Add content validation or sanitization props to give users control over what content is allowed.

```typescript
// Content validation schema
interface ContentValidationConfig {
  maxContentLength?: number;
  allowHtml?: boolean;
  allowedProtocols?: string[];
  disallowedTags?: string[];
  sanitizeLinks?: boolean;
  requireSchema?: boolean;
}

// Default validation configuration
const defaultValidation: ContentValidationConfig = {
  maxContentLength: 1000000, // 1MB
  allowHtml: false,
  allowedProtocols: ['http:', 'https:'],
  disallowedTags: ['script', 'iframe', 'object', 'embed'],
  sanitizeLinks: true,
  requireSchema: false
};

// Content validation function
function validateContent(content: string, config: ContentValidationConfig = defaultValidation): {
  isValid: boolean;
  sanitized?: string;
  error?: string;
} {
  // Check length
  if (config.maxContentLength && content.length > config.maxContentLength) {
    return {
      isValid: false,
      error: `Content exceeds maximum length of ${config.maxContentLength} characters`
    };
  }

  // Basic markdown structure validation
  if (config.requireSchema) {
    const hasValidStructure = /^#[\s\S]*|^##[\s\S]*|^###[\s\S]*|^- [\s\S]*|\d+\. [\s\S]*/.test(content);
    if (!hasValidStructure && content.trim().length > 0) {
      return {
        isValid: false,
        error: 'Content must follow markdown structure'
      };
    }
  }

  // HTML content validation
  if (!config.allowHtml && /<[a-z][\s\S]*>/i.test(content)) {
    return {
      isValid: false,
      error: 'HTML content is not allowed'
    };
  }

  // Sanitize if needed
  let sanitizedContent = content;
  if (config.sanitizeLinks) {
    sanitizedContent = sanitizeLinks(sanitizedContent, config.allowedProtocols);
  }

  return {
    isValid: true,
    sanitized: sanitizedContent
  };
}

// Link sanitization function
function sanitizeLinks(content: string, allowedProtocols: string[] = ['http:', 'https:']): string {
  // Match markdown links
  return content.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (match, text, url) => {
    try {
      const urlObj = new URL(url);

      // Check protocol
      if (!allowedProtocols.includes(urlObj.protocol)) {
        console.warn(`Disallowed protocol in URL: ${url}`);
        return `[${text}](#)`;
      }

      // Check for javascript: data: etc.
      if (urlObj.protocol === 'javascript:' || urlObj.protocol === 'data:') {
        console.warn(`Unsafe protocol in URL: ${url}`);
        return `[${text}](#)`;
      }

      return match;
    } catch (error) {
      // Invalid URL
      console.warn(`Invalid URL in markdown: ${url}`);
      return `[${text}](#)`;
    }
  });
}

// Svelte component implementation
<script lang="ts">
  import { onMount } from 'svelte';

  export let content: string = '';
  export let validate: ContentValidationConfig = defaultValidation;

  let validation: { isValid: boolean; error?: string };
  let processedContent: string;

  onMount(() => {
    validateAndProcessContent();
  });

  $: content, () => {
    validateAndProcessContent();
  });

  function validateAndProcessContent() {
    validation = validateContent(content, validate);

    if (validation.isValid) {
      processedContent = processMarkdown(content);
    } else {
      processedContent = `<div class="error">Content validation failed: ${validation.error}</div>`;
    }
  }
</script>

<div class="markdown-renderer">
  {#if validation.error}
    <div class="validation-error" role="alert">
      ⚠️ {validation.error}
    </div>
  {/if}

  {@html processedContent}
</div>
```

## Related Components
- MarkdownRenderer.svelte
- Content sanitization utilities
- Markdown processing pipeline

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_MarkdownRenderer.svelte.md
- OWASP Markdown Security: https://owasp.org/www-community/attacks/Markdown_injection
- Content Security Policy: https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP