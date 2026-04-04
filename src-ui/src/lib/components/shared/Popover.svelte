<script lang="ts">
	/**
	 * Popover Component
	 * Unified popover/dropdown menu for all context menus and action dropdowns.
	 *
	 * Design Features:
	 * - Portal rendering for proper z-index stacking
	 * - Click-outside and keyboard handling
	 * - Synchronized item stagger animation
	 * - Position anchoring to trigger element
	 * - Destructive/success/warning variants
	 */
	import { onMount } from 'svelte';
	import styles from './Popover.module.css';
	import type { PopoverItem, PopoverOrigin } from './popover.types';
	import { generatePopoverId } from '$lib/utils/idGenerator';
	import { requestPopoverOpen, onPopoverCloseAll } from '$lib/stores/popoverStore';

	interface Props {
		open: boolean;
		onClose: () => void;
		items: PopoverItem[];
		onSelect: (item: PopoverItem) => void;
		anchorEl?: HTMLElement | null;
		origin?: PopoverOrigin;
		class?: string;
	}

	let {
		open,
		onClose,
		items,
		onSelect,
		anchorEl = null,
		origin = 'top-right',
		class: className = ''
	}: Props = $props();

	let popoverRef: HTMLDivElement | null = $state(null);
	let position = $state({ top: 0, left: 0, right: 0 });
	const id = generatePopoverId();

	function updatePosition() {
		if (!anchorEl) return;

		const rect = anchorEl.getBoundingClientRect();
		const popoverWidth = 172;

		if (origin === 'top-right' || origin === 'bottom-right') {
			position = {
				top: origin === 'bottom-right' ? rect.bottom + 4 : rect.top,
				left: 0,
				right: window.innerWidth - rect.right
			};
		} else {
			position = {
				top: origin === 'bottom-left' ? rect.bottom + 4 : rect.top,
				left: rect.left,
				right: 0
			};
		}
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (!open) return;

		if (e.key === 'Escape') {
			e.preventDefault();
			onClose();
		}
	}

	function handleWindowClick(e: MouseEvent) {
		if (
			open &&
			popoverRef &&
			!popoverRef.contains(e.target as Node) &&
			anchorEl &&
			!anchorEl.contains(e.target as Node)
		) {
			onClose();
		}
	}

	function handleSelect(item: PopoverItem) {
		if (item.disabled) return;
		onSelect(item);
		onClose();
	}

	function portal(node: HTMLElement) {
		document.body.appendChild(node);
		return {
			destroy() {
				if (node.parentNode) {
					node.parentNode.removeChild(node);
				}
			}
		};
	}

	$effect(() => {
		if (open) {
			updatePosition();
			// Notify other popovers to close
			requestPopoverOpen(id);
		}
	});

	// Listen for close-all events from other popovers
	let unregisterCloseListener: (() => void) | null = null;

	onMount(() => {
		window.addEventListener('keydown', handleKeyDown);

		// Register listener for close-all events from other popovers
		unregisterCloseListener = onPopoverCloseAll((requestingId: string) => {
			// If another popover is requesting to open (not this one), close this one
			if (requestingId !== id && open) {
				onClose();
			}
		});

		return () => {
			window.removeEventListener('keydown', handleKeyDown);
			unregisterCloseListener?.();
		};
	});
</script>

<svelte:window onclick={handleWindowClick} onscroll={onClose} onresize={onClose} />

{#if open}
	<div
		use:portal
		bind:this={popoverRef}
		class={[
			styles.popover,
			origin === 'top-left' && styles.originTopLeft,
			origin === 'top-right' && styles.originTopRight,
			origin === 'bottom-left' && styles.originBottomLeft,
			origin === 'bottom-right' && styles.originBottomRight,
			className
		]
			.filter(Boolean)
			.join(' ')}
		role="menu"
		style={origin === 'top-right' || origin === 'bottom-right'
			? `top: ${position.top}px; right: ${position.right}px;`
			: `top: ${position.top}px; left: ${position.left}px;`}
	>
		{#each items as item, index (item.id)}
			<button
				class={[
					styles.item,
					item.variant === 'success' && styles.itemSuccess,
					item.variant === 'warning' && styles.itemWarning,
					item.variant === 'destructive' && styles.itemDestructive
				]
					.filter(Boolean)
					.join(' ')}
				onclick={() => handleSelect(item)}
				role="menuitem"
				type="button"
				disabled={item.disabled}
			>
				{#if item.iconHtml}
					<span class={styles.itemIcon}>
						{@html item.iconHtml}
					</span>
				{/if}
				<span class={styles.itemLabel}>{item.label}</span>
				{#if item.shortcut}
					<span class={styles.itemShortcut}>{item.shortcut}</span>
				{/if}
			</button>
		{/each}
	</div>
{/if}
