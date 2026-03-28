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
	import type {Snippet} from 'svelte';

	type Position = 'top' | 'bottom' | 'left' | 'right';

	/**
	 * Allowed CSS properties for tooltip positioning.
	 * Whitelist approach prevents injection of unexpected properties.
	 */
	const ALLOWED_POSITION_PROPERTIES = ['left', 'top', 'transform', 'opacity'] as const;
	const ALLOWED_ARROW_PROPERTIES = ['transform'] as const;

	/**
	 * Allowed transform function patterns for validation.
	 * Only these specific transform patterns are permitted.
	 */
	const ALLOWED_TRANSFORM_PATTERNS = [
		/^translateX\(calc\(-50% \+ -?\d+px\)\)$/,
		/^translateY\(calc\(-50% \+ -?\d+px\)\)$/,
		/^translateX\(calc\(-100% \+ -?\d+px\)\)$/,
		/^translateY\(calc\(-100% \+ -?\d+px\)\)$/,
		/^translateX\(calc\(-50% \+ -?\d+px\)\) translateY\(-100%\)$/,
		/^translateX\(-100%\) translateY\(calc\(-50% \+ -?\d+px\)\)$/,
		/^translateY\(calc\(-50% \+ -?\d+px\)\)$/,
		/^translateX\(calc\(-50% \+ -?\d+px\)\) rotate\(45deg\)$/,
		/^translateY\(calc\(-50% \+ -?\d+px\)\) rotate\(45deg\)$/
	];

	/**
	 * Validates that a transform string contains only allowed patterns.
	 * Prevents CSS injection through malicious transform values.
	 */
	function isValidTransform(transform: string, allowedPatterns: RegExp[]): boolean {
		return allowedPatterns.some((pattern) => pattern.test(transform));
	}

	/**
	 * Sanitizes a numeric value to ensure it's safe for CSS.
	 * Returns a valid CSS pixel value or empty string if invalid.
	 */
	function sanitizeNumericValue(value: number): string {
		if (!Number.isFinite(value)) {
			console.warn('Tooltip: Non-finite numeric value detected, using 0');
			return '0px';
		}
		return `${Math.round(value)}px`;
	}

	/**
	 * Creates a safe style object for tooltip positioning.
	 * All values are validated and sanitized to prevent CSS injection.
	 */
	function createSafePositionStyle(
		left: number,
		top: number,
		transform: string,
		isInitialRender: boolean
	): Record<string, string> {
		const safeStyles: Record<string, string> = {
			position: 'fixed'
		};

		// Sanitize numeric position values
		safeStyles.left = sanitizeNumericValue(left);
		safeStyles.top = sanitizeNumericValue(top);

		// Validate transform against allowed patterns
		if (isValidTransform(transform, ALLOWED_TRANSFORM_PATTERNS)) {
			safeStyles.transform = transform;
		} else {
			console.warn('Tooltip: Invalid transform value rejected:', transform);
			safeStyles.transform = 'none';
		}

		// Handle initial render opacity
		if (isInitialRender) {
			safeStyles.opacity = '0';
		}

		return safeStyles;
	}

	/**
	 * Creates a safe style object for tooltip arrow positioning.
	 * Validates transform to prevent injection attacks.
	 */
	function createSafeArrowStyle(
		actualPosition: Position,
		arrowOffset: number
	): Record<string, string> {
		const safeStyles: Record<string, string> = {};

		let transform: string;
		if (actualPosition === 'top' || actualPosition === 'bottom') {
			transform = `translateX(calc(-50% + ${Math.round(arrowOffset)}px)) rotate(45deg)`;
		} else {
			transform = `translateY(calc(-50% + ${Math.round(arrowOffset)}px)) rotate(45deg)`;
		}

		// Validate the generated transform
		if (isValidTransform(transform, ALLOWED_TRANSFORM_PATTERNS)) {
			safeStyles.transform = transform;
		} else {
			console.warn('Tooltip: Invalid arrow transform rejected:', transform);
			safeStyles.transform = 'rotate(45deg)';
		}

		return safeStyles;
	}

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
	let actualPosition = $state<Position>('top');
	let triggerRef: HTMLElement | null = $state(null);
	let tooltipRef: HTMLDivElement | null = $state(null);
	let arrowOffset = $state(0); // Arrow horizontal offset from center
	let showTimeout: ReturnType<typeof setTimeout> | null = null;
	let hideTimeout: ReturnType<typeof setTimeout> | null = null;

	// Sync actualPosition with position prop changes
	$effect(() => {
		actualPosition = position;
	});

	// Use $effect to trigger calculation after DOM is updated and visible
	$effect(() => {
		if (visible && tooltipRef) {
			// Trigger calculation on next frame to ensure correct tooltip dimensions
			requestAnimationFrame(() => {
				calculatePosition();
			});
		}
	});

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
			visible = true;
			// Calculation is now handled by $effect upon visibility
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
	function calculateArrowOffset(triggerRect: DOMRect, tooltipRect: DOMRect, pos: Position) {
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
	 * Returns a safe style object to prevent CSS injection.
	 */
	let positionStyle = $derived.by((): Record<string, string> => {
		if (!triggerRef) return { position: 'fixed', opacity: '0' };

		const triggerRect = triggerRef.getBoundingClientRect();
		// Get tooltip dimensions if already rendered, otherwise use estimates
		// This handles the initial frame where tooltipRef is being established
		const tooltipWidth = tooltipRef?.offsetWidth ?? 0;
		const tooltipHeight = tooltipRef?.offsetHeight ?? 0;

		// Don't show if tooltip isn't measured yet, but keep rendered for measurement
		const isInitialRender = visible && (!tooltipWidth || !tooltipHeight);

		// Calculate trigger center points (sub-pixel precise)
		const triggerCenterX = Math.round(triggerRect.left + triggerRect.width / 2);
		const triggerCenterY = Math.round(triggerRect.top + triggerRect.height / 2);

		// Position calculations with gap
		const positions: Record<Position, { left: number; top: number; transform: string }> = {
			top: {
				left: triggerCenterX,
				top: Math.round(triggerRect.top - gap),
				transform: 'translateX(calc(-50% + 0px)) translateY(-100%)'
			},
			bottom: {
				left: triggerCenterX,
				top: Math.round(triggerRect.bottom + gap),
				transform: 'translateX(calc(-50% + 0px))'
			},
			left: {
				left: Math.round(triggerRect.left - gap),
				top: triggerCenterY,
				transform: 'translateX(-100%) translateY(calc(-50% + 0px))'
			},
			right: {
				left: Math.round(triggerRect.right + gap),
				top: triggerCenterY,
				transform: 'translateY(calc(-50% + 0px))'
			}
		};

		const pos = positions[actualPosition];

		// Viewport clamping for edge cases
		const viewportWidth = window.innerWidth;
		const viewportHeight = window.innerHeight;
		let clampedLeft = pos.left;
		let clampedTop = pos.top;
		let transform = pos.transform;

		if (actualPosition === 'top' || actualPosition === 'bottom') {
			// Clamp horizontal position to keep tooltip in viewport
			const halfWidth = tooltipWidth / 2;
			const minLeft = viewportPadding + halfWidth;
			const maxLeft = viewportWidth - viewportPadding - halfWidth;
			clampedLeft = Math.max(minLeft, Math.min(maxLeft, pos.left));
			
			// Adjust transform to account for clamping
			const drift = pos.left - clampedLeft;
			transform = pos.transform.replace('0px', `${Math.round(drift)}px`);
		} else {
			// Clamp vertical position for left/right tooltips
			const halfHeight = tooltipHeight / 2;
			const minTop = viewportPadding + halfHeight;
			const maxTop = viewportHeight - viewportPadding - halfHeight;
			clampedTop = Math.max(minTop, Math.min(maxTop, pos.top));

			// Adjust transform to account for clamping
			const drift = pos.top - clampedTop;
			transform = pos.transform.replace('0px', `${Math.round(drift)}px`);
		}

		// Use safe style object creation to prevent CSS injection
		return createSafePositionStyle(clampedLeft, clampedTop, transform, isInitialRender);
	});

	// Arrow position style based on calculated offset
	// Uses safe style object to prevent CSS injection
	let arrowStyle = $derived.by(() => {
		return createSafeArrowStyle(actualPosition, arrowOffset);
	});

	let tooltipClasses = $derived(
		[
			styles.tooltip,
			styles[actualPosition],
			visible && styles.visible,
			prefersReducedMotion && styles.noAnimation
		]
			.filter(Boolean)
			.join(' ')
	);

	// Track scrolling and resizing to keep tooltip attached
	$effect(() => {
		const update = () => {
			if (visible && triggerRef && tooltipRef) {
				requestAnimationFrame(calculatePosition);
			}
		};

		window.addEventListener('scroll', update, { capture: true, passive: true });
		window.addEventListener('resize', update, { passive: true });

		return () => {
			window.removeEventListener('scroll', update, { capture: true });
			window.removeEventListener('resize', update);
		};
	});

	// Reposition on scroll/resize
	function handleScroll() {
		if (visible && triggerRef && tooltipRef) {
			requestAnimationFrame(() => {
				calculatePosition();
			});
		}
	}
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
		{#if typeof children === 'string'}
			{children}
		{:else}
			{@render children()}
		{/if}
	</div>
{/if}

{#if !disabled && content}
	<div
		class={tooltipClasses}
		style={Object.entries(positionStyle)
			.map(([key, value]) => `${key}: ${value}`)
			.join('; ')}
		style:max-width={maxWidth}
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
		<div
			class={styles.arrow}
			style={Object.entries(arrowStyle)
				.map(([key, value]) => `${key}: ${value}`)
				.join('; ')}
		></div>
	</div>
{/if}
