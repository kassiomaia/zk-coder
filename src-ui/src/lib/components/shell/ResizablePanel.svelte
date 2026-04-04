<script lang="ts">
	/**
	 * ResizablePanel Component
	 * A resizable panel with persistent width using the same approach as agent trace window.
	 * Supports left or right positioning with configurable min/max widths.
	 *
	 * The panel and resizer are rendered as siblings within a wrapper container.
	 */
	import { onMount, onDestroy } from 'svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		/** Panel position - which side the resize handle is on */
		position?: 'left' | 'right';
		/** Default width in pixels */
		defaultWidth?: number;
		/** Minimum width in pixels */
		minWidth?: number;
		/** Maximum width in pixels */
		maxWidth?: number;
		/** Storage key for persisting width */
		storageKey?: string;
		/** Whether the panel is currently open */
		open?: boolean;
		/** Callback when width changes */
		onWidthChange?: (width: number) => void;
		/** Panel content */
		children: Snippet;
	}

	let {
		position = 'right',
		defaultWidth = 320,
		minWidth = 240,
		maxWidth = 600,
		storageKey,
		open = true,
		onWidthChange,
		children
	}: Props = $props();

	// Initialize width - will be overridden by localStorage if present
	function getInitialWidth() {
		return defaultWidth;
	}

	let width = $state(getInitialWidth());
	let isResizing = $state(false);
	let startX = 0;
	let startWidth = 0;

	// Load persisted width on mount
	onMount(() => {
		if (storageKey) {
			try {
				const saved = localStorage.getItem(storageKey);
				if (saved) {
					const savedWidth = parseInt(saved, 10);
					if (!isNaN(savedWidth) && savedWidth >= minWidth && savedWidth <= maxWidth) {
						width = savedWidth;
					}
				}
			} catch (e) {
				console.warn('Failed to load panel width:', e);
			}
		}
	});

	// Persist width changes
	$effect(() => {
		if (storageKey && width >= minWidth && width <= maxWidth) {
			try {
				localStorage.setItem(storageKey, String(width));
			} catch (e) {
				console.warn('Failed to save panel width:', e);
			}
		}
		onWidthChange?.(width);
	});

	function handleMouseDown(event: MouseEvent) {
		isResizing = true;
		startX = event.clientX;
		startWidth = width;
		document.body.style.cursor = 'col-resize';
		document.body.style.userSelect = 'none';
		document.addEventListener('mousemove', handleMouseMove);
		document.addEventListener('mouseup', handleMouseUp);
	}

	function handleMouseMove(event: MouseEvent) {
		if (!isResizing) return;

		const delta = event.clientX - startX;

		// Drag in the direction you want the panel to grow
		// position="right" (left sidebar): drag left to increase
		// position="left" (right sidebar): drag right to increase
		const newWidth = position === 'right' ? startWidth - delta : startWidth + delta;

		width = Math.max(minWidth, Math.min(maxWidth, newWidth));
	}

	function handleMouseUp() {
		isResizing = false;
		startX = 0;
		startWidth = 0;
		document.body.style.cursor = '';
		document.body.style.userSelect = '';
		document.removeEventListener('mousemove', handleMouseMove);
		document.removeEventListener('mouseup', handleMouseUp);
	}

	// Cleanup on destroy
	onDestroy(() => {
		if (isResizing) {
			document.removeEventListener('mousemove', handleMouseMove);
			document.removeEventListener('mouseup', handleMouseUp);
		}
	});

	const panelStyle = $derived(open ? `width: ${width}px;` : '');
	const wrapperClass = $derived(`resizable-panel-wrapper ${position}`);
	const resizerClass = $derived(
		`resizer ${position === 'left' ? 'resizer-left' : 'resizer-right'} ${isResizing ? 'resizer-active' : ''}`
	);
</script>

{#if open}
	<div class={wrapperClass}>
		<div class="resizable-panel" style={panelStyle}>
			{@render children()}
		</div>

		<!-- svelte-ignore a11y_mouse_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<button
			class={resizerClass}
			type="button"
			aria-label="Resize panel"
			onmousedown={handleMouseDown}
		></button>
	</div>
{/if}

<style>
	/* Wrapper contains both panel and resizer */
	.resizable-panel-wrapper {
		display: flex;
		align-items: stretch;
		min-height: 100%;
		flex-shrink: 0;
	}

	/* For right sidebar (resizer on left edge) */
	.resizable-panel-wrapper.right {
		flex-direction: row;
	}

	/* For left sidebar (resizer on right edge) */
	.resizable-panel-wrapper.left {
		flex-direction: row-reverse;
	}

	.resizable-panel {
		display: flex;
		flex-direction: column;
		min-height: 100%;
		background: var(--depth-bg-2);
		/* No transition on width for instant resize response */
		overflow: hidden;
	}

	/* Resizer button - simple 2px bar with extended hit area */
	.resizer {
		position: relative;
		width: 2px;
		padding: 0;
		border: 0;
		appearance: none;
		background: transparent;
		cursor: col-resize;
		z-index: 10;
		flex-shrink: 0;
		/* Extend hit area */
		margin-left: -1px;
		margin-right: -1px;
		transition: background-color 0.15s;
		-webkit-app-region: no-drag;
	}

	/* Active/hover states */
	.resizer:hover,
	.resizer-active {
		background: var(--color-primary-500);
		transition-delay: 80ms;
	}

	/* Reduced motion */
	@media (prefers-reduced-motion: reduce) {
		.resizer {
			transition: none;
		}
	}

	/* Touch support */
	@media (pointer: coarse) {
		.resizer {
			width: 4px;
		}
	}
</style>
