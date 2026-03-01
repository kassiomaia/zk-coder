<script lang="ts">
	/**
	 * MarkdownPreview Component
	 * Renders a preview of markdown content with syntax highlighting.
	 */
	import styles from './MarkdownPreview.module.css';

	interface Props {
		/** Markdown content to render */
		content: string;
		/** Additional CSS class */
		class?: string;
	}

	let { content, class: className }: Props = $props();

	// Simple markdown parsing (in production, use a library like markdown-it)
	let renderedHtml = $derived(parseMarkdown(content));

	function parseMarkdown(text: string): string {
		if (!text) return '';

		let html = text;

		// Escape HTML
		html = html.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

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

		// Links
		html = html.replace(
			/\[([^\]]+)\]\(([^)]+)\)/g,
			`<a href="$2" class="${styles.link}" target="_blank" rel="noopener noreferrer">$1</a>`
		);

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

		return html;
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
