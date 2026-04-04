<script lang="ts">
	/**
	 * ListItemActions Component
	 * Unified context menu actions for list items (conversations and threads).
	 * Displays a "..." button that opens a menu on click.
	 */
	import Popover from './Popover.svelte';
	import type { PopoverItem } from './popover.types';
	import type { ListItemActionsProps } from './list-item-actions.types';
	import { MoreIcon } from '$lib/icons';
	import styles from './ListItemActions.module.css';

	let {
		ondelete,
		onrename,
		onarchive,
		onresolve,
		isResolved = false,
		isArchived = false,
		iconSize = 16,
		onMenuOpen,
		onMenuClose
	}: ListItemActionsProps = $props();

	let isOpen = $state(false);
	let triggerRef: HTMLButtonElement | null = $state(null);

	function svgIcon(paths: string): string {
		return `<svg
      width="${iconSize}"
      height="${iconSize}"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >${paths}</svg>`;
	}

	const ICONS = {
		resolve: svgIcon('<polyline points="20 6 9 17 4 12" />'),
		reopen: svgIcon(
			'<path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />' +
				'<path d="M3 3v5h5" />' +
				'<path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />' +
				'<path d="M16 21h5v-5" />'
		),
		rename: svgIcon(
			'<path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />' + '<path d="m15 5 4 4" />'
		),
		archive: svgIcon(
			'<polyline points="21 8 21 21 3 21 3 8" />' +
				'<rect x="1" y="3" width="22" height="5" />' +
				'<line x1="10" y1="12" x2="14" y2="12" />'
		),
		delete: svgIcon(
			'<polyline points="3 6 5 6 21 6" />' +
				'<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />'
		)
	};

	function buildItems(): PopoverItem[] {
		const items: PopoverItem[] = [];

		if (onresolve) {
			items.push({
				id: isResolved ? 'reopen' : 'resolve',
				label: isResolved ? 'Reopen' : 'Resolve',
				variant: 'success',
				iconHtml: isResolved ? ICONS.reopen : ICONS.resolve
			});
		}

		if (onrename) {
			items.push({
				id: 'rename',
				label: 'Rename',
				iconHtml: ICONS.rename
			});
		}

		if (onarchive) {
			items.push({
				id: 'archive',
				label: isArchived ? 'Restore' : 'Archive',
				variant: isArchived ? 'default' : 'warning',
				iconHtml: ICONS.archive
			});
		}

		if (ondelete) {
			items.push({
				id: 'delete',
				label: 'Delete',
				variant: 'destructive',
				iconHtml: ICONS.delete
			});
		}

		return items;
	}

	function handleSelect(item: PopoverItem) {
		switch (item.id) {
			case 'resolve':
			case 'reopen':
				onresolve?.();
				break;
			case 'rename':
				onrename?.();
				break;
			case 'archive':
				onarchive?.();
				break;
			case 'delete':
				ondelete?.();
				break;
		}
	}

	function toggleMenu(e: MouseEvent) {
		e.stopPropagation();
		if (isOpen) {
			closeMenu();
		} else {
			isOpen = true;
			onMenuOpen?.();
		}
	}

	function closeMenu() {
		if (!isOpen) return;
		isOpen = false;
		onMenuClose?.();
	}

	let items = $derived(buildItems());
</script>

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
		<MoreIcon size={iconSize} />
	</button>

	<Popover
		open={isOpen}
		onClose={closeMenu}
		{items}
		onSelect={handleSelect}
		anchorEl={triggerRef}
		origin="top-right"
	/>
</div>
