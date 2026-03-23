<script lang="ts">
	/**
	 * DiffViewer Component
	 * Renders unified or side-by-side diffs using Diff2Html.
	 * Features syntax highlighting, word-level diffing, and custom dark theme.
	 */
	import { onMount, onDestroy } from 'svelte';
	import { Diff2HtmlUI } from 'diff2html/lib/ui/js/diff2html-ui';
	import 'diff2html/bundles/css/diff2html.min.css';
	import hljs from 'highlight.js/lib/core';
	import typescript from 'highlight.js/lib/languages/typescript';
	import javascript from 'highlight.js/lib/languages/javascript';
	import rust from 'highlight.js/lib/languages/rust';
	import python from 'highlight.js/lib/languages/python';
	import css from 'highlight.js/lib/languages/css';
	import scss from 'highlight.js/lib/languages/scss';
	import json from 'highlight.js/lib/languages/json';
	import yaml from 'highlight.js/lib/languages/yaml';
	import markdown from 'highlight.js/lib/languages/markdown';
	import html from 'highlight.js/lib/languages/xml';
	import svelte from 'highlight.js/lib/languages/xml'; // Reuse XML for Svelte

	// Register common languages for syntax highlighting
	hljs.registerLanguage('typescript', typescript);
	hljs.registerLanguage('javascript', javascript);
	hljs.registerLanguage('rust', rust);
	hljs.registerLanguage('python', python);
	hljs.registerLanguage('css', css);
	hljs.registerLanguage('scss', scss);
	hljs.registerLanguage('json', json);
	hljs.registerLanguage('yaml', yaml);
	hljs.registerLanguage('markdown', markdown);
	hljs.registerLanguage('html', html);
	hljs.registerLanguage('svelte', svelte);
	hljs.registerLanguage('xml', html);

	interface Props {
		/** Unified diff content (e.g., from `similar` crate output) */
		diff: string;
		/** Maximum height in pixels before scrolling */
		maxHeight?: number;
		/** View mode: unified or side-by-side */
		view?: 'unified' | 'side-by-side';
		/** Enable word-level diffing */
		wordLevelDiff?: boolean;
		/** Language for syntax highlighting (auto-detected if not specified) */
		language?: string;
	}

	let { diff, maxHeight = 500, view = 'unified', wordLevelDiff = true, language }: Props = $props();

	let container: HTMLElement;
	let diff2HtmlUi: Diff2HtmlUI | null = null;
	let currentView = $state<'unified' | 'side-by-side'>(view);

	// Update internal view when prop changes
	$effect(() => {
		currentView = view;
	});

	// Re-render when diff, currentView, or wordLevelDiff changes
	$effect(() => {
		if (container && diff) {
			renderDiff();
		}
	});

	function renderDiff() {
		if (!container) return;

		// Clear previous content
		container.innerHTML = '';

		try {
			const targetElement = container;
			const configuration = {
				drawFileList: false,
				matching: wordLevelDiff ? 'words' : 'lines',
				outputFormat: currentView === 'side-by-side' ? 'side-by-side' : 'line-by-line',
				highlight: true,
				// Don't use built-in theme - we use custom CSS
				style: ''
			};

			diff2HtmlUi = new Diff2HtmlUI(targetElement, diff, configuration);
			diff2HtmlUi.draw();
			diff2HtmlUi.highlightCode();
		} catch (error) {
			console.error('[DiffViewer] Failed to render diff:', error);
			container.innerHTML = `<div class="diff-error">Failed to render diff: ${error}</div>`;
		}
	}

	function toggleView() {
		currentView = currentView === 'unified' ? 'side-by-side' : 'unified';
	}

	onDestroy(() => {
		if (container) {
			container.innerHTML = '';
		}
		diff2HtmlUi = null;
	});
</script>

<div class="diff-viewer-wrapper" style:max-height="{maxHeight}px">
	<div class="diff-header">
		<span class="diff-title">Changes</span>
		<button
			class="view-toggle"
			onclick={toggleView}
			type="button"
			title="Switch between unified and side-by-side view"
		>
			{#if currentView === 'unified'}
				Side by Side
			{:else}
				Unified
			{/if}
		</button>
	</div>
	<div bind:this={container} class="diff2html-container"></div>
</div>

<style>
	.diff-viewer-wrapper {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--color-bg-primary);
		border-radius: var(--radius-md);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		line-height: var(--leading-normal);
		border: 1px solid var(--color-border-subtle);
	}

	.diff-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) var(--space-3);
		background: var(--color-surface);
		border-bottom: 1px solid var(--color-border-subtle);
		flex-shrink: 0;
	}

	.diff-title {
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
	}

	.view-toggle {
		padding: var(--space-1) var(--space-2);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		color: var(--color-text-muted);
		background: var(--color-neutral-800);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition:
			background var(--transition-fast),
			color var(--transition-fast);
	}

	.view-toggle:hover {
		background: var(--color-neutral-700);
		color: var(--color-text-primary);
	}

	.view-toggle:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: 2px;
	}

	.diff2html-container {
		overflow: auto;
		flex: 1;
		min-height: 0;
	}

	/* ============================================================================
	   Diff2Html Custom Theme - Matches Coder Design System
	   ============================================================================ */

	/* Base wrapper */
	:global(.d2h-wrapper) {
		background: transparent !important;
		border: none !important;
		padding: 0 !important;
	}

	/* File header */
	:global(.d2h-file-header) {
		background: var(--color-surface) !important;
		border-bottom: 1px solid var(--color-border-subtle) !important;
		border-top: 1px solid var(--color-border-subtle) !important;
		padding: var(--space-2) var(--space-3) !important;
	}

	:global(.d2h-file-name) {
		color: var(--color-text-primary) !important;
		font-size: var(--text-xs) !important;
		font-family: var(--font-mono) !important;
	}

	:global(.d2h-file-stats) {
		color: var(--color-text-muted) !important;
		font-size: var(--text-2xs) !important;
	}

	/* Line numbers */
	:global(.d2h-code-line-header),
	:global(.d2h-code-side-linenumber) {
		background: var(--color-bg-elevated) !important;
		color: var(--color-text-muted) !important;
		border-color: var(--color-border-subtle) !important;
		line-height: 1.6 !important;
	}

	/* Line content - unchanged */
	:global(.d2h-code-line) {
		color: var(--color-text-secondary) !important;
		background: var(--color-bg-primary) !important;
	}

	:global(.d2h-code-side-line) {
		color: var(--color-text-secondary) !important;
		background: var(--color-bg-primary) !important;
	}

	/* Deleted lines (red) */
	:global(.d2h-del) {
		background: var(--color-error-light) !important;
	}

	:global(.d2h-del .d2h-code-line),
	:global(.d2h-del .d2h-code-side-line) {
		background: var(--color-error-light) !important;
		color: var(--color-error-text) !important;
	}

	:global(.d2h-del .d2h-code-line-prefix),
	:global(.d2h-del .d2h-code-side-line-prefix) {
		color: var(--color-error) !important;
	}

	/* Added lines (green) */
	:global(.d2h-ins) {
		background: var(--color-success-subtle) !important;
	}

	:global(.d2h-ins .d2h-code-line),
	:global(.d2h-ins .d2h-code-side-line) {
		background: var(--color-success-subtle) !important;
		color: var(--color-text-primary) !important;
	}

	:global(.d2h-ins .d2h-code-line-prefix),
	:global(.d2h-ins .d2h-code-side-line-prefix) {
		color: var(--color-success) !important;
	}

	/* Diff with no changes */
	:global(.d2h-d-none) {
		background: var(--color-bg-elevated) !important;
	}

	/* Change markers */
	:global(.d2h-change) {
		background: var(--color-warning-light) !important;
	}

	/* Syntax highlighting colors (optimized for dark backgrounds) */
	:global(.hljs-keyword) {
		color: #ff79c6 !important;
	}
	:global(.hljs-string) {
		color: #f1fa8c !important;
	}
	:global(.hljs-number) {
		color: #bd93f9 !important;
	}
	:global(.hljs-comment) {
		color: #6272a4 !important;
	}
	:global(.hljs-function) {
		color: #50fa7b !important;
	}
	:global(.hljs-title) {
		color: #8be9fd !important;
	}
	:global(.hljs-params) {
		color: var(--color-text-secondary) !important;
	}
	:global(.hljs-built_in) {
		color: #ffb86c !important;
	}
	:global(.hljs-literal) {
		color: #bd93f9 !important;
	}
	:global(.hljs-class) {
		color: #ff79c6 !important;
	}
	:global(.hljs-attr) {
		color: #8be9fd !important;
	}
	:global(.hljs-tag) {
		color: #ff79c6 !important;
	}
	:global(.hljs-name) {
		color: #50fa7b !important;
	}
	:global(.hljs-property) {
		color: #66d9ef !important;
	}
	:global(.hljs-operator) {
		color: #ff79c6 !important;
	}
	:global(.hljs-punctuation) {
		color: var(--color-text-muted) !important;
	}
	:global(.hljs-subst) {
		color: var(--color-text-secondary) !important;
	}
	:global(.hljs-symbol) {
		color: #ffb86c !important;
	}
	:global(.hljs-variable) {
		color: #f8f8f2 !important;
	}
	:global(.hljs-bullet) {
		color: #8be9fd !important;
	}
	:global(.hljs-code) {
		color: #50fa7b !important;
	}
	:global(.hljs-emphasis) {
		font-style: italic !important;
	}
	:global(.hljs-strong) {
		font-weight: bold !important;
	}
	:global(.hljs-formula) {
		background: #3d3d3d !important;
	}
	:global(.hljs-link) {
		color: #8be9fd !important;
		text-decoration: underline !important;
	}
	:global(.hljs-quote) {
		color: #6272a4 !important;
	}
	:global(.hljs-selector-tag) {
		color: #ff79c6 !important;
	}
	:global(.hljs-selector-id) {
		color: #8be9fd !important;
	}
	:global(.hljs-selector-class) {
		color: #50fa7b !important;
	}
	:global(.hljs-selector-attr) {
		color: #bd93f9 !important;
	}
	:global(.hljs-selector-pseudo) {
		color: #ffb86c !important;
	}
	:global(.hljs-template-tag) {
		color: #ff79c6 !important;
	}
	:global(.hljs-template-variable) {
		color: #f1fa8c !important;
	}

	/* Scrollbar styling */
	.diff2html-container::-webkit-scrollbar {
		width: 6px;
		height: 6px;
	}

	.diff2html-container::-webkit-scrollbar-track {
		background: transparent;
	}

	.diff2html-container::-webkit-scrollbar-thumb {
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
	}

	.diff2html-container::-webkit-scrollbar-thumb:hover {
		background: var(--color-neutral-600);
	}

	/* Error state */
	.diff-error {
		padding: var(--space-4);
		color: var(--color-error);
		text-align: center;
	}
</style>
