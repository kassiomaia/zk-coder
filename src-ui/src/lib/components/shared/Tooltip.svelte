<script lang="ts">
	/**
	 * Tooltip Component
	 * A premium tooltip system with pixel-perfect micro-alignment and smart positioning.
	 *
	 * Features:
	 * - Sub-pixel precise positioning for crisp rendering
	 * - Intelligent collision detection with viewport edges
	 * - Arrow tracks tooltip center, clamps to trigger bounds
	 * - Smooth animations with position-aware origins
	 * - Scroll/resize aware positioning
	 */
	import styles from './Tooltip.module.css';
	import type { Snippet } from 'svelte';

	type Position = 'top' | 'bottom' | 'left' | 'right';

	interface Props {
		/** Tooltip content (string or snippet) */
		content: string | Snippet;
		/** Preferred position relative to trigger */
		position?: Position;
		/** Delay before showing tooltip (ms) */
		showDelay?: number;
		/** Delay before hiding tooltip (ms) */
		hideDelay?: number;
		/** Disable tooltip */
		disabled?: boolean;
		/** Maximum width of tooltip */
		maxWidth?: string;
		/** Gap between tooltip and trigger (px) */
		gap?: number;
		/** Offset from edge of viewport (px) */
		viewportPadding?: number;
		/** Child element that triggers the tooltip */
		children?: Snippet;
	}

	let {
		content,
		position = 'top',
		showDelay = 400,
		hideDelay = 100,
		disabled = false,
		maxWidth = '220px',
		gap = 8,
		viewportPadding = 8,
		children
	}: Props = $props();

	// State
	let visible = $state(false);
	let actualPosition = $state<Position>(position);
	let triggerRef: HTMLElement | null = $state(null);
	let tooltipRef: HTMLDivElement | null = $state(null);
	let arrowOffset = $state(0); // Arrow horizontal offset from center
	let showTimeout: ReturnType<typeof setTimeout> | null = null;
	let hideTimeout: ReturnType<typeof setTimeout> | null = null;

	// Constants for micro-alignment
	const ARROW_SIZE = 6; // Match CSS arrow size
	const ARROW_OFFSET_MIN = 8; // Minimum distance from tooltip edge

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
			requestAnimationFrame(() => {
				calculatePosition();
				visible = true;
			});
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

	/**
	 * Calculate optimal tooltip position with micro-alignment.
	 * Ensures tooltip stays within viewport while maintaining visual harmony.
	 */
	function calculatePosition() {
		if (!triggerRef || !tooltipRef) return;

		const triggerRect = triggerRef.getBoundingClientRect();
		const tooltipRect = tooltipRef.getBoundingClientRect();
		const viewportWidth = window.innerWidth;
		const viewportHeight = window.innerHeight;

		// Determine best position based on available space
		const spaces = {
			top: triggerRect.top - viewportPadding,
			bottom: viewportHeight - triggerRect.bottom - viewportPadding,
			left: triggerRect.left - viewportPadding,
			right: viewportWidth - triggerRect.right - viewportPadding
		};

		const requiredSpace = {
			top: tooltipRect.height + gap,
			bottom: tooltipRect.height + gap,
			left: tooltipRect.width + gap,
			right: tooltipRect.width + gap
		};

		// Check if preferred position works, otherwise find best alternative
		let bestPosition = position;
		const positionPriority: Position[] = [position];

		// Add fallback positions based on available space
		if (position === 'top' || position === 'bottom') {
			positionPriority.push(position === 'top' ? 'bottom' : 'top', 'right', 'left');
		} else {
			positionPriority.push(position === 'left' ? 'right' : 'left', 'bottom', 'top');
		}

		for (const pos of positionPriority) {
			if (spaces[pos] >= requiredSpace[pos]) {
				bestPosition = pos;
				break;
			}
		}

		actualPosition = bestPosition;

		// Calculate arrow offset to keep it aligned with trigger center
		// but clamped within tooltip bounds
		calculateArrowOffset(triggerRect, tooltipRect, bestPosition);
	}

	/**
	 * Calculate arrow offset to ensure it points to trigger center
	 * while staying within tooltip bounds.
	 */
	function calculateArrowOffset(
		triggerRect: DOMRect,
		tooltipRect: DOMRect,
		pos: Position
	) {
		if (pos === 'top' || pos === 'bottom') {
			// Horizontal arrow offset
			const triggerCenter = triggerRect.left + triggerRect.width / 2;
			const tooltipCenter = tooltipRect.left + tooltipRect.width / 2;
			const offset = triggerCenter - tooltipCenter;

			// Clamp arrow position within tooltip bounds
			const maxOffset = tooltipRect.width / 2 - ARROW_OFFSET_MIN - ARROW_SIZE;
			arrowOffset = Math.max(-maxOffset, Math.min(maxOffset, offset));
		} else {
			// Vertical arrow offset
			const triggerCenter = triggerRect.top + triggerRect.height / 2;
			const tooltipCenter = tooltipRect.top + tooltipRect.height / 2;
			const offset = triggerCenter - tooltipCenter;

			// Clamp arrow position within tooltip bounds
			const maxOffset = tooltipRect.height / 2 - ARROW_OFFSET_MIN - ARROW_SIZE;
			arrowOffset = Math.max(-maxOffset, Math.min(maxOffset, offset));
		}
	}

	/**
	 * Calculate pixel-perfect position styles.
	 * Uses Math.round() for sub-pixel precision on displays.
	 */
	let positionStyle = $derived.by(() => {
		if (!triggerRef) return '';

		const triggerRect = triggerRef.getBoundingClientRect();
		const tooltipWidth = tooltipRef?.getBoundingClientRect().width ?? 0;
		const tooltipHeight = tooltipRef?.getBoundingClientRect().height ?? 0;

		// Calculate trigger center points (sub-pixel precise)
		const triggerCenterX = Math.round(triggerRect.left + triggerRect.width / 2);
		const triggerCenterY = Math.round(triggerRect.top + triggerRect.height / 2);

		// Position calculations with gap
		const positions: Record<Position, { left: number; top: number; transform: string }> = {
			top: {
				left: triggerCenterX,
				top: Math.round(triggerRect.top - gap),
				transform: 'translateX(-50%) translateY(-100%)'
			},
			bottom: {
				left: triggerCenterX,
				top: Math.round(triggerRect.bottom + gap),
				transform: 'translateX(-50%)'
			},
			left: {
				left: Math.round(triggerRect.left - gap),
				top: triggerCenterY,
				transform: 'translateX(-100%) translateY(-50%)'
			},
			right: {
				left: Math.round(triggerRect.right + gap),
				top: triggerCenterY,
				transform: 'translateY(-50%)'
			}
		};

		const pos = positions[actualPosition];

		// Viewport clamping for edge cases
		const viewportWidth = window.innerWidth;
		const viewportHeight = window.innerHeight;
		let clampedLeft = pos.left;
		let clampedTop = pos.top;

		if (actualPosition === 'top' || actualPosition === 'bottom') {
			// Clamp horizontal position to keep tooltip in viewport
			const halfWidth = tooltipWidth / 2;
			const minLeft = viewportPadding + halfWidth;
			const maxLeft = viewportWidth - viewportPadding - halfWidth;
			clampedLeft = Math.max(minLeft, Math.min(maxLeft, pos.left));
		} else {
			// Clamp vertical position for left/right tooltips
			const halfHeight = tooltipHeight / 2;
			const minTop = viewportPadding + halfHeight;
			const maxTop = viewportHeight - viewportPadding - halfHeight;
			clampedTop = Math.max(minTop, Math.min(maxTop, pos.top));
		}

		return `left: ${clampedLeft}px; top: ${clampedTop}px; transform: ${pos.transform};`;
	});

	// Arrow position style based on calculated offset
	let arrowStyle = $derived.by(() => {
		if (actualPosition === 'top' || actualPosition === 'bottom') {
			return `transform: translateX(calc(-50% + ${Math.round(arrowOffset)}px)) rotate(45deg);`;
		} else {
			return `transform: translateY(calc(-50% + ${Math.round(arrowOffset)}px)) rotate(45deg);`;
		}
	});

	let tooltipClasses = $derived([
		styles.tooltip,
		styles[actualPosition],
		visible && styles.visible,
		prefersReducedMotion && styles.noAnimation
	]
		.filter(Boolean)
		.join(' '));

	// Reposition on scroll/resize
	function handleScroll() {
		if (visible && triggerRef && tooltipRef) {
			requestAnimationFrame(() => {
				calculatePosition();
			});
		}
	}
</script>

<svelte:window on:keydown={handleKeyDown} on:scroll={handleScroll} />

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
		style="position: fixed; {positionStyle} max-width: {maxWidth};"
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
		<div class={styles.arrow} style={arrowStyle}></div>
	</div>
{/if}
