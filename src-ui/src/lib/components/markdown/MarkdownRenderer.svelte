<script lang="ts">
	/**
	 * MarkdownRenderer Component
	 * Renders markdown content with syntax highlighting, tables, task lists, and image support.
	 */
	import { browser } from '$app/environment';
	import { renderMarkdown } from '$lib/utils/markdown';
	import styles from './MarkdownRenderer.module.css';

	interface Props {
		/** Raw markdown content */
		content: string;
		/** Additional CSS class */
		class?: string;
	}

	let { content, class: className = '' }: Props = $props();

	// Only render markdown on the client to avoid SSR issues with highlight.js
	// On the server, show plain text as fallback
	let renderedHtml = $derived(browser ? renderMarkdown(content) : escapeHtmlSimple(content));

	/**
	 * Simple HTML escape for SSR fallback
	 */
	function escapeHtmlSimple(str: string): string {
		if (!str) return '';
		return str
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/\n/g, '<br>');
	}
</script>

<div class={`${styles.markdown} ${className}`}>
	{@html renderedHtml}
</div>
