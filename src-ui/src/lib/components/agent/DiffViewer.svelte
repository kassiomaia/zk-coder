<script lang="ts">
    /**
     * DiffViewer Component
     * Renders unified or side-by-side diffs using Diff2Html.
     * Features syntax highlighting, word-level diffing, and custom dark theme.
     */
    import {onDestroy} from 'svelte';
    import {Diff2HtmlUI} from 'diff2html/lib/ui/js/diff2html-ui';
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
	let currentView = $state<'unified' | 'side-by-side'>('unified');

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
			container.innerHTML = `<div class="diff-error">Failed to render diff: \${error}</div>`;
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

<div class="diff-viewer-wrapper">
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
		height: 100%;
		min-height: 0;
		background: var(--color-bg-primary);
		border: 1px solid var(--color-border-primary);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.diff-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) var(--space-4);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border-primary);
		flex-shrink: 0;
	}

	.diff-title {
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.1em;
		color: var(--color-text-muted);
	}

	.view-toggle {
		padding: 2px 8px;
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-secondary);
		background: var(--color-neutral-800);
		border: 1px solid var(--color-border-subtle);
		border-radius: 4px;
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.view-toggle:hover {
		background: var(--color-neutral-700);
		color: var(--color-text-primary);
	}

	.diff2html-container {
		flex: 1;
		min-height: 0;
		overflow: hidden !important; /* Prevent the main container from scrolling */
		background: var(--color-bg-primary);
		display: flex;
		flex-direction: column;
	}

	:global(.d2h-file-diff) {
		overflow: auto !important; /* Only this element should scroll */
		flex: 1 !important;
		min-height: 0 !important;
		background: transparent !important;
		scrollbar-width: thin;
		scrollbar-color: var(--color-neutral-700) transparent;
	}

	:global(.d2h-file-diff::-webkit-scrollbar) {
		width: 6px;
		height: 6px;
	}

	:global(.d2h-file-diff::-webkit-scrollbar-track) {
		background: transparent;
	}

	:global(.d2h-file-diff::-webkit-scrollbar-thumb) {
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
	}

	:global(.d2h-files-diff) {
		display: flex !important;
		flex-direction: column !important;
		flex: 1 !important;
		min-height: 0 !important;
		overflow: hidden !important;
	}

	:global(.d2h-wrapper) {
		display: flex !important;
		flex-direction: column !important;
		flex: 1 !important;
		min-height: 0 !important;
		overflow: hidden !important;
		border: none !important;
		margin: 0 !important;
		padding: 0 !important;
		background: transparent !important;
	}

	:global(.d2h-file-wrapper) {
		border: none !important;
		margin: 0 !important;
		margin-bottom: 0 !important;
		display: flex !important;
		flex-direction: column !important;
		flex: 1 !important;
		min-height: 0 !important;
		overflow: hidden !important;
	}

	/* Target split view specifically for vertical alignment */
	:global(.d2h-file-side-diff) {
		margin-bottom: 0 !important;
		display: table-cell !important;
		width: 50% !important; 
		vertical-align: top !important;
		position: relative !important;
		float: none !important; /* Ensure table-cell behavior is not overridden by floats */
	}

	:global(.d2h-files-diff) {
		display: table !important;
		width: 100% !important;
		table-layout: fixed !important;
		border-spacing: 0 !important;
		border-collapse: collapse !important;
	}

	:global(.d2h-file-side-diff table) {
		width: 100% !important;
		table-layout: fixed !important;
		border-collapse: collapse !important;
	}

	/* Reset code lines to block/table-row to ensure they fill the side panels */
	:global(.d2h-code-side-line) {
		display: table-row !important;
		width: 100% !important;
	}

	:global(.d2h-code-side-linenumber) {
		width: 50px !important;
		min-width: 50px !important;
		display: table-cell !important;
	}

	:global(.d2h-code-side-line-ctn) {
		display: table-cell !important;
		width: 100% !important;
	}

	/* Prevent internal diff2html headers/containers from introducing independent scrolls */
	:global(.d2h-file-diff) {
		overflow-y: auto !important;
		overflow-x: auto !important;
		flex: 1 !important;
		min-height: 0 !important;
		display: block !important;
		height: 100% !important; /* Ensure it takes full height to enable scrolling */
		scrollbar-width: thin;
		scrollbar-color: var(--color-neutral-700) transparent;
	}

	:global(.d2h-files-diff) {
		display: table !important;
		width: 100% !important;
		height: 100% !important; /* Fixed height for table within scroller */
		table-layout: fixed !important;
		border-spacing: 0 !important;
		border-collapse: collapse !important;
		min-height: 100% !important;
	}

	:global(.d2h-file-side-diff) {
		margin-bottom: 0 !important;
		display: table-cell !important;
		width: 50% !important; 
		height: 100% !important; /* Ensure columns fill scroller height */
		vertical-align: top !important;
		position: relative !important;
		float: none !important;
	}

	:global(.d2h-file-side-diff table) {
		width: 100% !important;
		height: 100% !important; /* Ensure internal table fills the column */
		table-layout: fixed !important;
		border-collapse: collapse !important;
	}

	:global(.d2h-file-wrapper) {
		display: flex !important;
		flex-direction: column !important;
		flex: 1 !important;
		min-height: 0 !important;
	}

	/* Ensure the "Original" and "Modified" headers don't push the tables out of sync */
	:global(.d2h-file-side-diff .d2h-code-side-linenumber) {
		position: sticky !important;
		left: 0 !important;
		z-index: 2 !important;
	}

	:global(.d2h-file-header) {
		display: none !important;
	}

	/* Ensure the header (hunk lines) stay while code scrolls if we want, 
	   but for now let's just make the whole diff area scrollable 
	*/

	/* CORE ALIGNMENT: Standard Table Flow */
	:global(.d2h-diff-table) {
		display: table !important;
		width: 100% !important;
		border-collapse: collapse !important;
		table-layout: auto !important;
		font-family: var(--font-mono) !important;
		font-size: 12px !important;
		border: none !important;
	}

	:global(.d2h-code-line) {
		display: table-row !important;
		background: transparent !important;
	}

	:global(.d2h-code-linenumber),
	:global(.d2h-code-side-linenumber) {
		display: table-cell !important;
		position: relative !important;
		float: none !important; /* Eliminate float-based detachment */
		width: 50px !important;
		min-width: 50px !important;
		background: var(--color-bg-secondary) !important;
		color: var(--color-text-muted) !important;
		border: none !important;
		border-right: 1px solid var(--color-border-subtle) !important;
		text-align: right !important;
		padding: 0 8px !important;
		line-height: 20px !important;
		user-select: none !important;
		vertical-align: top !important;
	}

	:global(.d2h-code-line-ctn) {
		display: table-cell !important;
		position: relative !important;
		float: none !important; /* Eliminate float-based detachment */
		padding: 0 16px !important;
		line-height: 20px !important;
		vertical-align: top !important;
		width: 100% !important;
		white-space: pre !important;
		word-break: normal !important;
	}

	/* TINTS & MARKERS */
	:global(.d2h-del) {
		background: rgba(248, 81, 73, 0.1) !important;
	}

	:global(.d2h-ins) {
		background: rgba(63, 185, 80, 0.1) !important;
	}

	/* Fix white background in split view (d2h-code-line/d2h-code-side-line) */
	:global(.d2h-code-line),
	:global(.d2h-code-side-line),
	:global(.d2h-code-line-ctn),
	:global(.d2h-code-side-linenumber) {
		background: transparent !important;
	}

	/* Ensure the empty/spacer side of a diff also uses a dark background */
	:global(.d2h-code-side-emptyplaceholder),
	:global(.d2h-emptyplaceholder) {
		background: var(--color-bg-primary) !important;
		border: none !important;
		display: table-row !important; /* Ensure it behaves like a row to maintain height */
	}

	/* Fix: Ensure split view tables always have at least 100% height to trigger scroll even if one side is empty */
	:global(.d2h-files-diff) {
		display: table !important;
		width: 100% !important;
		height: 100% !important;
		table-layout: fixed !important;
		border-spacing: 0 !important;
		border-collapse: collapse !important;
		min-height: 100% !important;
	}

	:global(.d2h-code-line-prefix) {
		display: inline-block !important;
		width: 1ch !important;
		margin-right: 8px !important;
		opacity: 0.5 !important;
		font-weight: bold !important;
	}

	/* Hunk header (The \"@@\" line) */
	:global(.d2h-info) {
		background: var(--color-neutral-900) !important;
		border: none !important;
		color: var(--color-text-muted) !important;
		font-size: 11px !important;
		font-style: italic !important;
		padding: 6px 16px !important;
		border-bottom: 1px solid var(--color-border-subtle) !important;
		border-top: 1px solid var(--color-border-subtle) !important;
	}

	:global(.d2h-code-linenumber.d2h-info),
	:global(.d2h-code-side-linenumber.d2h-info) {
		background: var(--color-neutral-800) !important;
		border: none !important;
	}

	/* Syntax highlighting colors */
	:global(.hljs-keyword) { color: #ff79c6 !important; }
	:global(.hljs-string) { color: #f1fa8c !important; }
	:global(.hljs-number) { color: #bd93f9 !important; }
	:global(.hljs-comment) { color: #6272a4 !important; }
	:global(.hljs-function) { color: #50fa7b !important; }
	:global(.hljs-title) { color: #8be9fd !important; }

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
</style>
