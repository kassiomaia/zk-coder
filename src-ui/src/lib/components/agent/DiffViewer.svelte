<script lang="ts">
	/**
	 * DiffViewer Component
	 * Renders unified or side-by-side diffs using Monaco Editor.
	 * Also supports a "Full File" view mode for inspecting the final result.
	 */
	import { onMount, onDestroy } from 'svelte';
	import * as monaco from 'monaco-editor';
	import { registerCustomThemes } from '$lib/utils/monaco-themes';

	interface Props {
		/** Original content for diff */
		originalContent?: string;
		/** Modified content for diff or single view */
		modifiedContent: string;
		/** File path to determine language */
		filePath: string;
		/** View mode: 'diff' or 'full' */
		mode?: 'diff' | 'full';
		/** Diff style: 'side-by-side' or 'inline' (unified) */
		diffStyle?: 'side-by-side' | 'inline';
		/** Preferred editor theme — auto-detects from app theme if omitted */
		theme?: 'vs-dark' | 'vs-light' | 'tokyo-night' | 'coder-dark' | 'coder-light';
		/** Whether to show the minimap */
		minimap?: boolean;
	}

	/** Detect the current app theme from the DOM, with localStorage fallback */
	function detectTheme(): 'coder-dark' | 'coder-light' {
		const attr = document.documentElement.getAttribute('data-theme');
		if (attr) return attr === 'dark' ? 'coder-dark' : 'coder-light';
		// Fallback: layout may not have set the attribute yet (child onMount fires first)
		const saved = localStorage.getItem('theme');
		if (saved) return saved === 'dark' ? 'coder-dark' : 'coder-light';
		return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'coder-dark' : 'coder-light';
	}

	let {
		originalContent = '',
		modifiedContent = '',
		filePath,
		mode = 'diff',
		diffStyle = 'side-by-side',
		theme: themeProp,
		minimap = true
	}: Props = $props();

	let resolvedTheme = $state<string>(themeProp ?? 'coder-light');

	let container = $state<HTMLDivElement | null>(null);
	let diffEditor = $state<monaco.editor.IStandaloneDiffEditor | null>(null);
	let codeEditor = $state<monaco.editor.IStandaloneCodeEditor | null>(null);
	let fontSize = $state(13);

	// Handle CTRL + Scroll for zoom
	function handleWheel(e: any) {
		const browserEvent = e.browserEvent || e;
		if (browserEvent.ctrlKey || browserEvent.metaKey) {
			browserEvent.preventDefault();
			const delta = browserEvent.deltaY > 0 ? -1 : 1;
			const newSize = Math.max(8, Math.min(32, fontSize + delta));
			if (newSize !== fontSize) {
				fontSize = newSize;
				updateEditorOptions();
			}
		}
	}

	function updateEditorOptions() {
		const options = {
			fontSize,
			lineHeight: Math.round(fontSize * 1.5)
		};
		
		if (diffEditor) {
			diffEditor.updateOptions(options);
		}
		if (codeEditor) {
			codeEditor.updateOptions(options);
		}
	}

	// Determine language from file extension
	function getLanguage(path: string): string {
		const ext = path.split('.').pop()?.toLowerCase();
		switch (ext) {
			case 'js':
			case 'mjs':
			case 'cjs':
				return 'javascript';
			case 'ts':
			case 'mts':
				return 'typescript';
			case 'py':
				return 'python';
			case 'rs':
				return 'rust';
			case 'json':
				return 'json';
			case 'html':
				return 'html';
			case 'css':
				return 'css';
			case 'scss':
				return 'scss';
			case 'md':
				return 'markdown';
			case 'svelte':
				return 'html'; // Monaco doesn't have native svelte, html is close
			case 'sh':
			case 'bash':
				return 'shell';
			case 'yml':
			case 'yaml':
				return 'yaml';
			default:
				return 'plaintext';
		}
	}

	let language = $derived(getLanguage(filePath));

	// Effect to update models and switch between editor types
	$effect(() => {
		if (!container) return;

		// Clean up previous editors if mode switches
		if (mode === 'diff') {
			if (codeEditor) {
				codeEditor.dispose();
				codeEditor = null;
			}
			if (!diffEditor) {
				initDiffEditor();
			} else {
				updateDiffModels();
			}
		} else {
			if (diffEditor) {
				diffEditor.dispose();
				diffEditor = null;
			}
			if (!codeEditor) {
				initCodeEditor();
			} else {
				updateCodeModel();
			}
		}
	});

	// Update resolved theme when prop changes or auto-detect
	$effect(() => {
		resolvedTheme = themeProp ?? detectTheme();
	});

	// Apply theme to editor when it changes
	$effect(() => {
		if (monaco.editor) {
			monaco.editor.setTheme(resolvedTheme);
		}
	});

	// Effect for diffStyle changes
	$effect(() => {
		if (diffEditor) {
			diffEditor.updateOptions({
				renderSideBySide: diffStyle === 'side-by-side'
			});
		}
	});

	function initDiffEditor() {
		if (!container) return;
		diffEditor = monaco.editor.createDiffEditor(container, {
			theme: resolvedTheme,
			automaticLayout: true,
			renderSideBySide: diffStyle === 'side-by-side',
			readOnly: true,
			minimap: { enabled: minimap },
			scrollBeyondLastLine: false,
			fontSize,
			lineHeight: Math.round(fontSize * 1.5),
			fontFamily: 'var(--font-mono)',
			renderLineHighlight: 'all',
			scrollbar: {
				vertical: 'auto',
				horizontal: 'auto',
				useShadows: false,
				verticalHasArrows: false,
				horizontalHasArrows: false,
				verticalScrollbarSize: 10,
				horizontalScrollbarSize: 10
			},
			diffWordWrap: 'on',
			renderIndicators: true,
			originalEditable: false,
			ignoreTrimWhitespace: false,
			folding: true,
			bracketPairColorization: { enabled: true },
			smoothScrolling: true,
			cursorBlinking: 'smooth',
			cursorSmoothCaretAnimation: 'on',
			mouseWheelZoom: false,
			fastScrollSensitivity: 5,
			hideCursorInOverviewRuler: true,
			overviewRulerBorder: false,
			renderOverviewRuler: false
		});
		
		// Add wheel listener for zoom
		diffEditor.getModifiedEditor().onMouseWheel(handleWheel);
		diffEditor.getOriginalEditor().onMouseWheel(handleWheel);
		
		// Also add native listener to handle the preventDefault more reliably
		container.addEventListener('wheel', handleWheel, { passive: false });
		
		updateDiffModels();
	}

	function initCodeEditor() {
		if (!container) return;
		codeEditor = monaco.editor.create(container, {
			theme: resolvedTheme,
			automaticLayout: true,
			readOnly: true,
			minimap: { enabled: minimap },
			scrollBeyondLastLine: false,
			fontSize,
			lineHeight: Math.round(fontSize * 1.5),
			fontFamily: 'var(--font-mono)',
			renderLineHighlight: 'all',
			scrollbar: {
				vertical: 'auto',
				horizontal: 'auto',
				useShadows: false,
				verticalScrollbarSize: 10,
				horizontalScrollbarSize: 10
			},
			wordWrap: 'on',
			folding: true,
			bracketPairColorization: { enabled: true },
			smoothScrolling: true,
			cursorBlinking: 'smooth',
			cursorSmoothCaretAnimation: 'on',
			renderWhitespace: 'none',
			overviewRulerBorder: false,
			hideCursorInOverviewRuler: true
		});
		
		// Add wheel listener for zoom
		codeEditor.onMouseWheel(handleWheel);
		
		// Also add native listener to handle the preventDefault more reliably
		container.addEventListener('wheel', handleWheel, { passive: false });
		
		updateCodeModel();
	}

	function updateDiffModels() {
		if (!diffEditor) return;
		const originalModel = monaco.editor.createModel(originalContent, language);
		const modifiedModel = monaco.editor.createModel(modifiedContent, language);
		diffEditor.setModel({
			original: originalModel,
			modified: modifiedModel
		});
	}

	function updateCodeModel() {
		if (!codeEditor) return;
		const model = monaco.editor.createModel(modifiedContent, language);
		codeEditor.setModel(model);
	}

	onMount(() => {
		registerCustomThemes();

		// Auto-detect theme on mount if no explicit prop
		if (!themeProp) {
			resolvedTheme = detectTheme();
		}

		// Observe app theme changes on the DOM
		const observer = new MutationObserver(() => {
			if (!themeProp) {
				resolvedTheme = detectTheme();
			}
		});
		observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

		return () => {
			observer.disconnect();
		};
	});

	onDestroy(() => {
		diffEditor?.dispose();
		codeEditor?.dispose();
		container?.removeEventListener('wheel', handleWheel);
	});
</script>

<div class="diff-viewer-wrapper">
	<div bind:this={container} class="monaco-host"></div>
</div>

<style>
	.diff-viewer-wrapper {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: 100%;
		min-height: 0;
		background: var(--color-editor-bg);
		overflow: hidden;
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
	}

	.monaco-host {
		flex: 1;
		min-height: 0;
		width: 100%;
		height: 100%;
		position: relative;
	}

	/* Monaco System Normalization - let the wrapper control the base background */
	:global(.monaco-editor),
	:global(.monaco-editor-background),
	:global(.monaco-editor .margin) {
		background-color: transparent !important;
	}

	:global(.monaco-diff-editor .diffOverview) {
		display: none !important;
	}

	/* Refined scrollbars - theme-aware */
	:global(.monaco-scrollable-element > .scrollbar > .slider) {
		background: var(--color-border-secondary) !important;
		border-radius: var(--radius-full) !important;
		opacity: 0.5;
		transition: opacity var(--transition-fast);
	}

	:global(.monaco-scrollable-element > .scrollbar > .slider:hover) {
		opacity: 0.8;
	}

	:global(.monaco-scrollable-element > .scrollbar.active > .slider) {
		opacity: 1;
	}

	/* Hide gradient shadows */
	:global(.monaco-editor .vertical-gradient-shadow) {
		display: none !important;
	}

	/* Subtle line numbers - reveal on hover */
	:global(.monaco-editor .margin-view-overlays .line-numbers) {
		opacity: 0.5;
		transition: opacity var(--transition-fast);
	}

	:global(.monaco-editor:hover .margin-view-overlays .line-numbers) {
		opacity: 1;
	}
</style>
