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
    /** Preferred editor theme — auto-detects from app theme if omitted */
    theme?: 'vs-dark' | 'vs-light' | 'tokyo-night' | 'coder-dark' | 'coder-light';
    /** Initial font size */
    fontSize?: number;
    /** Whether to show the minimap (default: false) */
    minimap?: boolean;
    /** Additional CSS classes */
    class?: string;
  }

  /** Detect the current app theme from the DOM, with localStorage fallback */
  function detectTheme(): 'coder-dark' | 'coder-light' {
    const attr = document.documentElement.getAttribute('data-theme');
    if (attr) return attr === 'dark' ? 'coder-dark' : 'coder-light';
    const saved = localStorage.getItem('theme');
    if (saved) return saved === 'dark' ? 'coder-dark' : 'coder-light';
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'coder-dark' : 'coder-light';
  }

  let {
    value = '',
    language = 'json',
    readOnly = true,
    theme: themeProp,
    fontSize = 12,
    minimap = false,
    class: className = ''
  }: Props = $props();

  // Initial values - will be updated by effects below
  function getInitialTheme() {
    return themeProp ?? 'coder-light';
  }

  function getInitialFontSize() {
    return fontSize;
  }

  let resolvedTheme = $state<string>(getInitialTheme());

  let container = $state<HTMLDivElement | null>(null);
  let editor = $state<monaco.editor.IStandaloneCodeEditor | null>(null);
  let currentFontSize = $state(getInitialFontSize());

  // Update editor value when prop changes
  $effect(() => {
    if (editor && value !== editor.getValue()) {
      editor.setValue(value);
    }
  });

  // Update resolved theme when prop changes or auto-detect
  $effect(() => {
    resolvedTheme = themeProp ?? detectTheme();
  });

  // Apply theme to editor when it changes
  $effect(() => {
    if (editor) {
      monaco.editor.setTheme(resolvedTheme);
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

    // Register custom themes BEFORE detecting/applying theme
    registerCustomThemes();

    // Auto-detect theme on mount if no explicit prop
    if (!themeProp) {
      resolvedTheme = detectTheme();
    }
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
      theme: resolvedTheme,
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

    // Observe app theme changes on the DOM
    const observer = new MutationObserver(() => {
      if (!themeProp) {
        resolvedTheme = detectTheme();
      }
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

    return () => {
      observer.disconnect();
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
