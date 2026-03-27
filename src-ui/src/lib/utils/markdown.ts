/**
 * Markdown rendering utility using marked with highlight.js
 * Supports: syntax highlighting, tables, task lists, and base64 images
 */
import {marked} from 'marked';
import hljs from 'highlight.js';

/**
 * Escape HTML special characters comprehensively
 * Protects against XSS attacks by escaping all potentially dangerous characters
 *
 * @param str - The string to escape
 * @returns The escaped string safe for HTML rendering
 */
export function escapeHtml(str: string): string {
	return str
		.replace(/&/g, '&amp;') // Must be first to avoid double-escaping
		.replace(/</g, '&lt;') // Prevent HTML tag injection
		.replace(/>/g, '&gt;') // Prevent HTML tag injection
		.replace(/"/g, '&quot;') // Prevent attribute breakout
		.replace(/'/g, '&#039;') // Prevent attribute breakout (single quotes)
		.replace(/`/g, '&#x60;') // Prevent template literal injection
		.replace(/\//g, '&#x2F;'); // Prevent </script> attacks
}

// Custom renderer for images, links, and code blocks
const renderer = {
	image(token: any): string | false {
		const src = token.href || '';
		const alt = token.text || '';
		const title = token.title;
		const titleAttr = title ? ` title="${escapeHtml(title)}"` : '';
		const lazyAttr = src.startsWith('data:image') ? ' loading="lazy"' : '';
		return `<img src="${escapeHtml(src)}" alt="${escapeHtml(alt)}"${titleAttr}${lazyAttr} class="markdown-image" data-zoomable="true" />`;
	},

	link(token: any): string | false {
		const url = token.href || '#';
		const text = token.text || '';
		const title = token.title;
		const titleAttr = title ? ` title="${escapeHtml(title)}"` : '';
		return `<a href="${escapeHtml(url)}"${titleAttr} target="_blank" rel="noopener noreferrer" class="markdown-link">${text}</a>`;
	},

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
};

marked.use({ renderer });

// Configure marked options for chat messages
marked.setOptions({
	gfm: true,
	breaks: false // Single newlines are ignored, double newlines create paragraphs
});

/**
 * Normalize line endings and render markdown content to HTML
 */
export function renderMarkdown(content: string): string {
	if (!content) return '';

	// Normalize line endings: \r\n -> \n, \r -> \n
	// Also collapse multiple consecutive newlines to double newlines max
	let normalized = content
		.replace(/\r\n/g, '\n')
		.replace(/\r/g, '\n')
		.replace(/\n{3,}/g, '\n\n') // 3+ newlines -> 2
		.trim();

	return marked.parse(normalized) as string;
}

/**
 * Render inline markdown content to HTML (no paragraph wrapper)
 */
export function renderMarkdownInline(content: string): string {
	if (!content) return '';
	return marked.parseInline(content) as string;
}

/**
 * Strip markdown formatting and return plain text
 */
export function stripMarkdown(content: string): string {
	if (!content) return '';
	const html = marked.parse(content) as string;
	return html
		.replace(/<[^>]*>/g, '')
		.replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>')
		.replace(/&amp;/g, '&')
		.replace(/&quot;/g, '"')
		.replace(/&#039;/g, "'")
		.trim();
}

export { marked };
