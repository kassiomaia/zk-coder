<script lang="ts">
  /**
   * WindowHeader Component
   * Shared header for chromeless Tauri windows.
   * Provides draggable title bar with window controls (minimize, maximize, close).
   * Matches the visual style of the Agent Trace shell header.
   */
  import { onMount } from 'svelte';
  import { Minus, Square, Maximize2, X } from 'lucide-svelte';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import type { Snippet } from 'svelte';

  interface Props {
    /** Window title */
    title: string;
    /** Optional subtitle (shown next to title in mono font) */
    subtitle?: string;
    /** Optional icon snippet rendered before the title */
    icon?: Snippet;
    /** Whether to show window controls (default: true) */
    showControls?: boolean;
  }

  let {
    title,
    subtitle,
    icon,
    showControls = true,
  }: Props = $props();

  let isMaximized = $state(false);
  let currentWindow: ReturnType<typeof getCurrentWebviewWindow> | null = null;

  onMount(() => {
    currentWindow = getCurrentWebviewWindow();
    currentWindow.onResized(async () => {
      if (currentWindow) {
        isMaximized = await currentWindow.isMaximized();
      }
    });
  });

  async function handleMinimize() {
    await currentWindow?.minimize();
  }

  async function handleMaximize() {
    if (!currentWindow) return;
    const isMax = await currentWindow.isMaximized();
    if (isMax) {
      await currentWindow.unmaximize();
    } else {
      await currentWindow.maximize();
    }
  }

  async function handleClose() {
    await currentWindow?.close();
  }

  async function handleDblClick(e: MouseEvent) {
    if ((e.target as HTMLElement).closest('button')) return;
    await handleMaximize();
  }

  async function handleMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest('button')) return;
    await currentWindow?.startDragging();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header
  class="header"
  onmousedown={handleMouseDown}
  ondblclick={handleDblClick}
>
  <div class="header-main">
    <div class="title-block">
      <h1 class="title">
        {#if icon}
          {@render icon()}
        {/if}
        {title}
      </h1>
      {#if subtitle}
        <p class="subtitle"><code>{subtitle}</code></p>
      {/if}
    </div>
  </div>

  {#if showControls}
    <div class="window-controls">
      <button
        class="window-btn"
        onclick={handleMinimize}
        title="Minimize"
        type="button"
      >
        <Minus size={14} />
      </button>
      <button
        class="window-btn"
        onclick={handleMaximize}
        title={isMaximized ? 'Restore' : 'Maximize'}
        type="button"
      >
        {#if isMaximized}
          <Square size={12} />
        {:else}
          <Maximize2 size={14} />
        {/if}
      </button>
      <button
        class="window-btn close"
        onclick={handleClose}
        title="Close"
        type="button"
      >
        <X size={14} />
      </button>
    </div>
  {/if}
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4);
    border-bottom: 1px solid var(--color-border-subtle);
    background: var(--color-bg-primary);
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  .header-main {
    display: flex;
    align-items: baseline;
    pointer-events: none;
  }

  .title-block {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
  }

  .title {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--font-semibold);
    color: var(--color-text-primary);
    letter-spacing: var(--tracking-tight);
  }

  .subtitle {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    font-family: var(--font-mono);
  }

  .subtitle code {
    color: var(--color-text-secondary);
    opacity: 0.8;
  }

  .window-controls {
    display: flex;
    align-items: center;
    gap: 0;
    pointer-events: auto;
    -webkit-app-region: no-drag;
  }

  .window-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: none;
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
    transition: all var(--transition-fast);
    border-radius: var(--radius-lg);
    pointer-events: auto;
    -webkit-app-region: no-drag;
  }

  .window-btn:hover {
    background: var(--color-surface-hover);
    color: var(--color-text-primary);
  }

  .window-btn.close:hover {
    background: var(--color-error-light);
    color: var(--color-error);
  }
</style>
