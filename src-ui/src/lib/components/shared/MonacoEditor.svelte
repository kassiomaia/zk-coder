<script lang="ts">
	/**
	 * MonacoEditor Component
	 * A lightweight wrapper for Monaco Editor in Svelte 5.
	 * Optimized for JSON payload inspection in the Trace window.
	 */
	import { onMount, onDestroy } from 'svelte';
	import * as monaco from 'monaco-editor';
	import { registerCustomThemes } from '$lib/utils/monaco-themes';

	interface Props {
		/** JSON or string value to display */
		value: string;
		/** Language for syntax highlighting (default: json) */
		language?: string;
		/** Whether to allow edits (default: true for inspection) */
		readOnly?: boolean;
		/** Preferred editor theme */
		theme?: 'vs-dark' | 'vs-light' | 'tokyo-night';
		/** Initial font size */
		fontSize?: number;
		/** Whether to show the minimap (default: false) */
		minimap?: boolean;
		/** Additional CSS classes */
		class?: string;
	}

	let { 
		value = '', 
		language = 'json', 
		readOnly = true, 
		theme = 'vs-dark',
		fontSize = 12,
		minimap = false,
		class: className = '' 
	}: Props = $props();

	let container = $state<HTMLDivElement | null>(null);
	let editor = $state<monaco.editor.IStandaloneCodeEditor | null>(null);
	let currentFontSize = $state(fontSize);

	// Update editor value when prop changes
	$effect(() => {
		if (editor && value !== editor.getValue()) {
			editor.setValue(value);
		}
	});

	// Update theme when prop changes
	$effect(() => {
		if (editor) {
			monaco.editor.setTheme(theme);
		}
	});

	// Update font size when changed via scroll
	$effect(() => {
		if (editor) {
			editor.updateOptions({ fontSize: currentFontSize });
			// Force a layout update to ensure everything scales correctly
			editor.layout();
		}
	});

	// Update minimap when prop changes
	$effect(() => {
		if (editor) {
			editor.updateOptions({ minimap: { enabled: minimap } });
		}
	});

	onMount(() => {
		if (!container) return;

		// ...existing registration and worker config...
		registerCustomThemes();
		if (!(window as any).MonacoEnvironment) {
			(window as any).MonacoEnvironment = {
				getWorkerUrl: function (_moduleId: any, label: string) {
					if (label === 'json') {
						return '/node_modules/monaco-editor/esm/vs/language/json/json.worker.js';
					}
					return '/node_modules/monaco-editor/esm/vs/editor/editor.worker.js';
				}
			};
		}

		editor = monaco.editor.create(container, {
			value,
			language,
			readOnly,
			theme,
			automaticLayout: true,
			minimap: { enabled: minimap },
			scrollBeyondLastLine: false,
			fontSize: currentFontSize,
			lineNumbers: 'on',
			folding: true,
			wordWrap: 'on',
			tabSize: 2,
			renderLineHighlight: 'all',
			scrollbar: {
				vertical: 'auto',
				horizontal: 'auto'
			},
			mouseWheelZoom: false,
			contextmenu: true,
			copyWithSyntaxHighlighting: true
		});

		// Listen on window to catch wheel events even if they occur over Monaco's overlays
		const handleWheelRaw = (e: WheelEvent) => {
			if (!container || !container.contains(e.target as Node)) return;

			if (e.ctrlKey) {
				e.preventDefault();
				e.stopPropagation();
				const delta = e.deltaY > 0 ? -2 : 2;
				currentFontSize = Math.max(8, Math.min(48, currentFontSize + delta));
			}
		};

		window.addEventListener('wheel', handleWheelRaw, { passive: false, capture: true });

		return () => {
			window.removeEventListener('wheel', handleWheelRaw, { capture: true });
			editor?.dispose();
		};
	});
</script>

<div bind:this={container} class="monaco-container {className}"></div>

<style>
	.monaco-container {
		width: 100%;
		height: 100%;
		min-height: 200px;
		overflow: hidden;
		flex: 1;
		display: flex;
		flex-direction: column;
		user-select: text;
		-webkit-user-select: text;
	}

	.monaco-container :global(.monaco-editor) {
		flex: 1;
		width: 100% !important;
		height: 100% !important;
	}
</style>
