<script lang="ts">
    /**
     * MarkdownRenderer Component
     * Renders markdown content with syntax highlighting, tables, task lists, and image support.
     * Includes content validation and sanitization for security.
     */
    import {browser} from '$app/environment';
    import {escapeHtml, renderMarkdown} from '$lib/utils/markdown';
    import CodeBlock from './code/CodeBlock.svelte';
    import {
        type ContentValidationConfig,
        defaultValidationConfig,
        mightNeedSanitization,
        validateContent
    } from '$lib/utils/contentValidation';
    import styles from './MarkdownRenderer.module.css';

    interface Props {
    /** Raw markdown content */
    content: string;
    /** Additional CSS class */
    class?: string;
    /**
     * Validation configuration options.
     * Set to null to disable validation (use only for trusted content).
     */
    validationConfig?: ContentValidationConfig | null;
    /** Whether to skip validation for trusted content (default: false) */
    skipValidation?: boolean;
  }

  let {
    content,
    class: className = '',
    validationConfig = {},
    skipValidation = false
  }: Props = $props();

  // Validate and render content
  let validationResult = $derived.by(() => {
    // Skip validation if explicitly requested or if config is null
    if (skipValidation || validationConfig === null) {
      return {
        isValid: true,
        sanitizedContent: content ?? '',
        error: undefined,
        warnings: []
      };
    }

    // Quick check for content that doesn't need sanitization
    if (!mightNeedSanitization(content)) {
      return {
        isValid: true,
        sanitizedContent: content ?? '',
        error: undefined,
        warnings: []
      };
    }

    // Full validation with merged config
    const mergedConfig = { ...defaultValidationConfig, ...validationConfig };
    return validateContent(content, mergedConfig);
  });

  // Only render markdown on the client to avoid SSR issues with highlight.js
  // On the server, show plain text as fallback using comprehensive HTML escaping
  let renderedHtml = $derived.by(() => {
    if (!validationResult.isValid) {
      // Return error message HTML for invalid content
      return `<div class="${styles.validationError}" role="alert">
        <span class="${styles.errorIcon}">!</span>
        <span class="${styles.errorMessage}">${escapeHtml(validationResult.error ?? 'Content validation failed')}</span>
      </div>`;
    }

    const contentToRender = validationResult.sanitizedContent;
    return browser
      ? renderMarkdown(contentToRender)
      : escapeHtml(contentToRender).replace(/\n/g, '<br>');
  });

  let blocks = $derived.by(() => {
    if (!browser) return [];
    
    const container = document.createElement('div');
    container.innerHTML = renderedHtml;
    
    const newBlocks: Array<{ type: 'html' | 'code', content: string, language?: string, code?: string }> = [];
    
    Array.from(container.childNodes).forEach(node => {
      if (node instanceof HTMLElement && node.classList.contains('code-block')) {
        const codeElem = node.querySelector('code');
        newBlocks.push({
          type: 'code',
          content: codeElem?.innerHTML || '',
          code: codeElem?.innerText || '',
          language: node.getAttribute('data-language') || ''
        });
      } else {
        const wrapper = document.createElement('div');
        if (node.nodeType === Node.TEXT_NODE) {
          wrapper.textContent = node.textContent;
        } else {
          wrapper.appendChild(node.cloneNode(true));
        }
        newBlocks.push({
          type: 'html',
          content: wrapper.innerHTML
        });
      }
    });
    
    return newBlocks;
  });
</script>

<div class={`${styles.markdown} ${className}`}>
  {#if !browser}
    {@html renderedHtml}
  {:else}
    {#each blocks as block}
      {#if block.type === 'code'}
        <CodeBlock 
          code={block.code || ''} 
          language={block.language} 
          highlightedCode={block.content} 
        />
      {:else}
        {@html block.content}
      {/if}
    {/each}
  {/if}
</div>
