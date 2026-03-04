<script lang="ts">
	/**
	 * MarkdownPreview Component
	 * Renders a preview of markdown content with syntax highlighting.
	 * Includes HTML sanitization to prevent XSS attacks.
	 */
	import DOMPurify from 'dompurify';
	import styles from './MarkdownPreview.module.css';

	interface Props {
		/** Markdown content to render */
		content: string;
		/** Additional CSS class */
		class?: string;
	}

	let { content, class: className }: Props = $props();

	// Simple markdown parsing with security sanitization
	let renderedHtml = $derived(parseMarkdown(content));

	/**
	 * Sanitizes a URL to prevent XSS attacks via dangerous protocols.
	 * Blocks javascript:, data:, vbscript:, and file: URLs.
	 */
	function sanitizeUrl(url: string): string {
		const trimmed = url.trim().toLowerCase();
		const dangerousProtocols = ['javascript:', 'data:', 'vbscript:', 'file:'];
		const safeProtocols = ['http:', 'https:', 'mailto:'];

		// Check for dangerous protocols
		for (const protocol of dangerousProtocols) {
			if (trimmed.startsWith(protocol)) {
				return '#';
			}
		}

		// Allow relative URLs (starting with /, #, or ?)
		if (trimmed.startsWith('/') || trimmed.startsWith('#') || trimmed.startsWith('?')) {
			return url;
		}

		// Check if it starts with a safe protocol
		for (const protocol of safeProtocols) {
			if (trimmed.startsWith(protocol)) {
				return url;
			}
		}

		// Default case: treat as potentially unsafe, return safe placeholder
		// This catches edge cases like "onclick=alert(1)" or other malformed URLs
		return '#';
	}

	/**
	 * Escapes special HTML characters to prevent injection.
	 */
	function escapeHtml(text: string): string {
		return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
	}

	function parseMarkdown(text: string): string {
		if (!text) return '';

		let html = text;

		// Escape HTML first
		html = escapeHtml(html);

		// Code blocks (must be before inline code)
		html = html.replace(/```(\w*)\n([\s\S]*?)```/g, (_, lang, code) => {
			return `<pre class="${styles.codeBlock}"><code class="${styles.code}" data-lang="${lang}">${code.trim()}</code></pre>`;
		});

		// Inline code
		html = html.replace(/`([^`]+)`/g, `<code class="${styles.inlineCode}">$1</code>`);

		// Headers
		html = html.replace(/^### (.+)$/gm, `<h3 class="${styles.h3}">$1</h3>`);
		html = html.replace(/^## (.+)$/gm, `<h2 class="${styles.h2}">$1</h2>`);
		html = html.replace(/^# (.+)$/gm, `<h1 class="${styles.h1}">$1</h1>`);

		// Bold and italic
		html = html.replace(/\*\*\*(.+?)\*\*\*/g, `<strong><em>$1</em></strong>`);
		html = html.replace(/\*\*(.+?)\*\*/g, `<strong>$1</strong>`);
		html = html.replace(/\*(.+?)\*/g, `<em>$1</em>`);
		html = html.replace(/___(.+?)___/g, `<strong><em>$1</em></strong>`);
		html = html.replace(/__(.+?)__/g, `<strong>$1</strong>`);
		html = html.replace(/_(.+?)_/g, `<em>$1</em>`);

		// Strikethrough
		html = html.replace(/~~(.+?)~~/g, `<del class="${styles.del}">$1</del>`);

		// Links - with URL sanitization to prevent XSS
		html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, linkText, url) => {
			const safeUrl = sanitizeUrl(url);
			return `<a href="${safeUrl}" class="${styles.link}" target="_blank" rel="noopener noreferrer">${linkText}</a>`;
		});

		// Lists
		html = html.replace(/^- (.+)$/gm, `<li class="${styles.li}">$1</li>`);
		html = html.replace(/(<li.*<\/li>\n?)+/g, `<ul class="${styles.ul}">$&</ul>`);

		// Ordered lists
		html = html.replace(/^\d+\. (.+)$/gm, `<li class="${styles.li}">$1</li>`);

		// Blockquotes
		html = html.replace(
			/^&gt; (.+)$/gm,
			`<blockquote class="${styles.blockquote}">$1</blockquote>`
		);

		// Horizontal rule
		html = html.replace(/^---$/gm, `<hr class="${styles.hr}" />`);

		// Paragraphs (double newline)
		html = html.replace(/\n\n/g, `</p><p class="${styles.p}">`);

		// Line breaks
		html = html.replace(/\n/g, '<br />');

		// Wrap in paragraph if not already wrapped
		if (!html.startsWith('<')) {
			html = `<p class="${styles.p}">${html}</p>`;
		}

		// Final sanitization with DOMPurify as defense-in-depth
		// This catches any edge cases we might have missed
		const sanitizedHtml = DOMPurify.sanitize(html, {
			// Allow only the tags we explicitly generate
			ALLOWED_TAGS: [
				'p',
				'br',
				'strong',
				'em',
				'del',
				'h1',
				'h2',
				'h3',
				'ul',
				'ol',
				'li',
				'blockquote',
				'hr',
				'a',
				'pre',
				'code'
			],
			// Allow only safe attributes
			ALLOWED_ATTR: ['class', 'href', 'target', 'rel', 'data-lang'],
			// Disable data attributes except for our explicit data-lang
			ALLOW_DATA_ATTR: false,
			// Add rel="noopener noreferrer" to all links for security
			ADD_ATTR: ['target', 'rel']
		});

		return sanitizedHtml;
	}
</script>

<div
	class={[styles.preview, className].filter(Boolean).join(' ')}
	role="region"
	aria-label="Markdown preview"
>
	{#if content}
		{@html renderedHtml}
	{:else}
		<p class={styles.empty}>Nothing to preview</p>
	{/if}
</div>
