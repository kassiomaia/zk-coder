<script lang="ts">
	/**
	 * ListItemActions Component
	 * Unified context menu actions for list items (conversations and threads).
	 * Displays a "..." button that opens a menu on click.
	 */
	import { onDestroy, onMount } from 'svelte';
	import styles from './ListItemActions.module.css';

	interface Props {
		/** Delete handler */
		ondelete?: () => void;
		/** Rename handler */
		onrename?: () => void;
		/** Archive handler */
		onarchive?: () => void;
		/** Resolve/reopen handler (thread-specific) */
		onresolve?: () => void;
		/** Whether the item is resolved (for resolve button state) */
		isResolved?: boolean;
		/** Icon size: 14 for compact view, 16 for standard */
		iconSize?: 14 | 16;
		/** Callback when menu opens */
		onMenuOpen?: () => void;
		/** Callback when menu closes */
		onMenuClose?: () => void;
	}

	let {
		ondelete,
		onrename,
		onarchive,
		onresolve,
		isResolved = false,
		iconSize = 16,
		onMenuOpen,
		onMenuClose
	}: Props = $props();

	let isOpen = $state(false);
	let menuRef: HTMLDivElement | null = null;
	let triggerRef: HTMLButtonElement | null = null;
	let menuPosition = $state({ top: 0, right: 0 });
	const id = Math.random().toString(36).substr(2, 9); // Unique ID for this instance

	onMount(() => {
		const handleCloseOtherMenus = (e: Event) => {
			const customEvent = e as CustomEvent;
			if (customEvent.detail.id !== id) {
				closeMenu();
			}
		};

		window.addEventListener('coder:close-other-menus', handleCloseOtherMenus);
		return () => {
			window.removeEventListener('coder:close-other-menus', handleCloseOtherMenus);
		};
	});

	function toggleMenu(e: MouseEvent) {
		e.stopPropagation();
		if (isOpen) {
			closeMenu();
		} else {
			// Close other menus first
			window.dispatchEvent(new CustomEvent('coder:close-other-menus', { detail: { id } }));
			
			isOpen = true;
			updatePosition();
			onMenuOpen?.();
		}
	}

	function updatePosition() {
		if (triggerRef) {
			const rect = triggerRef.getBoundingClientRect();
			// Position below the button, right-aligned to the button's right edge
			menuPosition = {
				top: rect.bottom + 4,
				right: window.innerWidth - rect.right
			};
		}
	}

	function closeMenu() {
		if (!isOpen) return;
		isOpen = false;
		onMenuClose?.();
	}

	function handleAction(action: () => void) {
		action();
		closeMenu();
	}

	function handleWindowClick(e: MouseEvent) {
		if (isOpen && menuRef && !menuRef.contains(e.target as Node) && !triggerRef?.contains(e.target as Node)) {
			closeMenu();
		}
	}
	
	/**
	 * Simple portal action to move element to body
	 */
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
</script>

<svelte:window 
	onclick={handleWindowClick} 
	onscroll={closeMenu} 
	onresize={closeMenu} 
/>

<div class={styles.container} role="toolbar" aria-label="List item actions">
	<button
		bind:this={triggerRef}
		class={styles.trigger}
		onclick={toggleMenu}
		aria-label="More actions"
		aria-expanded={isOpen}
		aria-haspopup="true"
		type="button"
	>
		<svg
			width={iconSize}
			height={iconSize}
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<circle cx="12" cy="12" r="1" />
			<circle cx="19" cy="12" r="1" />
			<circle cx="5" cy="12" r="1" />
		</svg>
	</button>

	{#if isOpen}
		<div
			use:portal
			class={styles.menu}
			bind:this={menuRef}
			role="menu"
			style="top: {menuPosition.top}px; right: {menuPosition.right}px;"
		>
			{#if onresolve}
				<button
					class={[styles.action, styles.resolve].filter(Boolean).join(' ')}
					onclick={(e) => {
						e.stopPropagation();
						handleAction(onresolve);
					}}
					role="menuitem"
					type="button"
				>
					{#if isResolved}
						<svg
							width={iconSize}
							height={iconSize}
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
							<path d="M3 3v5h5" />
							<path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
							<path d="M16 21h5v-5" />
						</svg>
						<span>Reopen</span>
					{:else}
						<svg
							width={iconSize}
							height={iconSize}
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<polyline points="20 6 9 17 4 12" />
						</svg>
						<span>Resolve</span>
					{/if}
				</button>
			{/if}

			{#if onrename}
				<button
					class={styles.action}
					onclick={(e) => {
						e.stopPropagation();
						handleAction(onrename);
					}}
					role="menuitem"
					type="button"
				>
					<svg
						width={iconSize}
						height={iconSize}
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
						<path d="m15 5 4 4" />
					</svg>
					<span>Rename</span>
				</button>
			{/if}

			{#if onarchive}
				<button
					class={[styles.action, styles.archive].filter(Boolean).join(' ')}
					onclick={(e) => {
						e.stopPropagation();
						handleAction(onarchive);
					}}
					role="menuitem"
					type="button"
				>
					<svg
						width={iconSize}
						height={iconSize}
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="21 8 21 21 3 21 3 8" />
						<rect x="1" y="3" width="22" height="5" />
						<line x1="10" y1="12" x2="14" y2="12" />
					</svg>
					<span>Archive</span>
				</button>
			{/if}

			{#if ondelete}
				<button
					class={[styles.action, styles.delete].filter(Boolean).join(' ')}
					onclick={(e) => {
						e.stopPropagation();
						handleAction(ondelete);
					}}
					role="menuitem"
					type="button"
				>
					<svg
						width={iconSize}
						height={iconSize}
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="3 6 5 6 21 6" />
						<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
					</svg>
					<span>Delete</span>
				</button>
			{/if}
		</div>
	{/if}
</div>
