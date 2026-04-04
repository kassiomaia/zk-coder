<script lang="ts">
	/**
	 * AppHeader Component
	 * Desktop-style application header for the main chromeless window.
	 * Provides window controls, command palette trigger, and drag support.
	 */
	import { onMount } from 'svelte';
	import { Minus, Square, Maximize2, X, Terminal } from 'lucide-svelte';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
	import type { Snippet } from 'svelte';

	interface Props {
		/** Optional subtitle shown next to title */
		subtitle?: string;
		/** Optional icon snippet */
		icon?: Snippet;
		/** Whether window controls are shown (default: true) */
		showControls?: boolean;
		/** Command palette trigger callback */
		onCommandPalette?: () => void;
	}

	let { subtitle, icon, showControls = true, onCommandPalette }: Props = $props();

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
		if (
			(e.target as HTMLElement).closest('button') ||
			(e.target as HTMLElement).closest('.command-trigger')
		)
			return;
		await handleMaximize();
	}

	async function handleMouseDown(e: MouseEvent) {
		if (e.button !== 0) return;
		if (
			(e.target as HTMLElement).closest('button') ||
			(e.target as HTMLElement).closest('.command-trigger')
		)
			return;
		await currentWindow?.startDragging();
	}

	function handleCommandPaletteKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'p' && !e.shiftKey) {
			e.preventDefault();
			onCommandPalette?.();
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<header
	class="app-header"
	onmousedown={handleMouseDown}
	ondblclick={handleDblClick}
	onkeydown={handleCommandPaletteKey}
>
	<!-- Left: Logo and app title -->
	<div class="header-left">
		<div class="app-brand">
			<div class="brand-icon">
				<Terminal size={14} strokeWidth={2} />
			</div>
			<span class="app-name">Coder</span>
		</div>
		{#if subtitle}
			<div class="header-subtitle">
				<code>{subtitle}</code>
			</div>
		{/if}
	</div>

	<!-- Spacer -->
	<div class="header-spacer"></div>

	<!-- Right: Window controls -->
	{#if showControls}
		<div class="window-controls">
			<button class="window-btn" onclick={handleMinimize} title="Minimize" type="button">
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
			<button class="window-btn close" onclick={handleClose} title="Close" type="button">
				<X size={14} />
			</button>
		</div>
	{/if}
</header>

<style>
	.app-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 44px;
		padding: 0 var(--space-3);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border-subtle);
		flex-shrink: 0;
		user-select: none;
		-webkit-user-select: none;
		position: relative;
		z-index: var(--z-sticky);
	}

	/* Left section - brand */
	.header-left {
		display: flex;
		align-items: baseline;
		gap: var(--space-3);
		pointer-events: none;
		flex-shrink: 0;
		min-width: 0;
	}

	.app-brand {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.brand-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 20px;
		height: 20px;
		color: var(--color-text-tertiary);
	}

	.app-name {
		font-size: var(--text-sm);
		font-weight: var(--font-semibold);
		color: var(--color-text-secondary);
		letter-spacing: var(--tracking-tight);
	}

	.header-subtitle {
		display: flex;
		align-items: center;
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
	}

	.header-subtitle code {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		opacity: 0.8;
		background: var(--color-bg-tertiary);
		padding: 1px var(--space-1);
		border-radius: var(--radius-sm);
	}

	/* Spacer */
	.header-spacer {
		flex: 1;
	}

	/* Window controls */
	.window-controls {
		display: flex;
		align-items: center;
		gap: 0;
		pointer-events: auto;
		flex-shrink: 0;
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
		border-radius: var(--radius-sm);
	}

	.window-btn:hover {
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
	}

	.window-btn.close:hover {
		background: var(--color-error-light);
		color: var(--color-error);
	}

	/* Traffic light positioning for macOS feel */
	@media (platform: 'darwin') {
		.window-controls {
			order: -1;
			margin-left: calc(var(--space-3) * -1);
			margin-right: var(--space-2);
		}
	}

	/* Compact mode for smaller windows */
	@media (max-width: 900px) {
		.header-subtitle {
			display: none;
		}
	}

	/* Reduced motion */
	@media (prefers-reduced-motion: reduce) {
		.window-btn {
			transition: none;
		}
	}
</style>
