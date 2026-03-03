<script lang="ts">
	/**
	 * Tooltip Component
	 * A reusable tooltip system with smart positioning and accessibility support.
	 */
	import styles from './Tooltip.module.css';
	import type { Snippet } from 'svelte';

	interface Props {
		/** Tooltip content (string or snippet) */
		content: string | Snippet;
		/** Position relative to trigger */
		position?: 'top' | 'bottom' | 'left' | 'right';
		/** Delay before showing tooltip (ms) */
		showDelay?: number;
		/** Delay before hiding tooltip (ms) */
		hideDelay?: number;
		/** Disable tooltip */
		disabled?: boolean;
		/** Maximum width of tooltip */
		maxWidth?: string;
		/** Child element that triggers the tooltip */
		children?: Snippet;
	}

	let {
		content,
		position = 'top',
		showDelay = 400,
		hideDelay = 100,
		disabled = false,
		maxWidth = '200px',
		children
	}: Props = $props();

	let visible = $state(false);
	let actualPosition = $state<'top' | 'bottom' | 'left' | 'right'>('top');
	let triggerRef: HTMLElement | null = $state(null);
	let tooltipRef: HTMLDivElement | null = $state(null);
	let showTimeout: ReturnType<typeof setTimeout> | null = null;
	let hideTimeout: ReturnType<typeof setTimeout> | null = null;

	// Check for reduced motion preference
	const prefersReducedMotion =
		typeof window !== 'undefined' &&
		window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;

	function clearAllTimeouts() {
		if (showTimeout) {
			clearTimeout(showTimeout);
			showTimeout = null;
		}
		if (hideTimeout) {
			clearTimeout(hideTimeout);
			hideTimeout = null;
		}
	}

	function handleShow() {
		if (disabled) return;
		clearAllTimeouts();
		showTimeout = setTimeout(() => {
			updatePosition();
			visible = true;
		}, showDelay);
	}

	function handleHide() {
		clearAllTimeouts();
		hideTimeout = setTimeout(() => {
			visible = false;
		}, hideDelay);
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape' && visible) {
			visible = false;
		}
	}

	function updatePosition() {
		if (!triggerRef || !tooltipRef) return;

		const triggerRect = triggerRef.getBoundingClientRect();
		const tooltipRect = tooltipRef.getBoundingClientRect();
		const viewportWidth = window.innerWidth;
		const viewportHeight = window.innerHeight;

		let newPosition = position;

		// Smart positioning - flip if tooltip would go off screen
		const spaceTop = triggerRect.top;
		const spaceBottom = viewportHeight - triggerRect.bottom;
		const spaceLeft = triggerRect.left;
		const spaceRight = viewportWidth - triggerRect.right;

		if (position === 'top' && spaceTop < tooltipRect.height + 8) {
			newPosition = 'bottom';
		} else if (position === 'bottom' && spaceBottom < tooltipRect.height + 8) {
			newPosition = 'top';
		} else if (position === 'left' && spaceLeft < tooltipRect.width + 8) {
			newPosition = 'right';
		} else if (position === 'right' && spaceRight < tooltipRect.width + 8) {
			newPosition = 'left';
		}

		actualPosition = newPosition;
	}

	let positionStyle = $derived.by(() => {
		if (!triggerRef) return '';

		const triggerRect = triggerRef.getBoundingClientRect();
		const scrollX = window.scrollX;
		const scrollY = window.scrollY;

		const positions: Record<string, string> = {
			top: `
				left: ${triggerRect.left + scrollX + triggerRect.width / 2}px;
				top: ${triggerRect.top + scrollY - 8}px;
				transform: translateX(-50%) translateY(-100%);
			`,
			bottom: `
				left: ${triggerRect.left + scrollX + triggerRect.width / 2}px;
				top: ${triggerRect.bottom + scrollY + 8}px;
				transform: translateX(-50%);
			`,
			left: `
				left: ${triggerRect.left + scrollX - 8}px;
				top: ${triggerRect.top + scrollY + triggerRect.height / 2}px;
				transform: translateX(-100%) translateY(-50%);
			`,
			right: `
				left: ${triggerRect.right + scrollX + 8}px;
				top: ${triggerRect.top + scrollY + triggerRect.height / 2}px;
				transform: translateY(-50%);
			`
		};

		return positions[actualPosition];
	});

	let tooltipClasses = $derived([
		styles.tooltip,
		styles[actualPosition],
		visible && styles.visible,
		prefersReducedMotion && styles.noAnimation
	]
		.filter(Boolean)
		.join(' '));
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if children}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class={styles.trigger}
		onmouseenter={handleShow}
		onmouseleave={handleHide}
		onfocusin={handleShow}
		onfocusout={handleHide}
		bind:this={triggerRef}
	>
		{@render children()}
	</div>
{/if}

{#if !disabled && content}
	<div
		class={tooltipClasses}
		style="position: absolute; {positionStyle} max-width: {maxWidth};"
		bind:this={tooltipRef}
		role="tooltip"
		aria-hidden={!visible}
	>
		<div class={styles.content}>
			{#if typeof content === 'string'}
				{content}
			{:else}
				{@render content()}
			{/if}
		</div>
		<div class={styles.arrow}></div>
	</div>
{/if}
