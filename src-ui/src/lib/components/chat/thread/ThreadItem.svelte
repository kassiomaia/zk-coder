<script lang="ts">
	/**
	 * ThreadItem Component
	 * Individual thread item in the thread list.
	 */
	import styles from './ThreadItem.module.css';
	import ListItemActions from '$lib/components/shared/ListItemActions.svelte';
	import InlineRenameInput from '$lib/components/shared/InlineRenameInput.svelte';
	import { formatRelativeTime } from '$lib/utils/formatRelativeTime';
	import type { Thread } from '$lib/types/thread';

	interface Props {
		/** Thread data */
		thread: Thread;
		/** Whether this thread is selected */
		selected?: boolean;
		/** Visual variant: 'default' for full size, 'inline' for reduced size with indentation */
		variant?: 'default' | 'inline';
		/** Click handler */
		onclick: () => void;
		/** Resolve handler */
		onresolve?: () => void;
		/** Delete handler */
		ondelete?: () => void;
		/** Archive handler */
		onarchive?: () => void;
		/** Rename handler */
		onrename?: (newTitle: string) => void;
		/** Optional inline styles (used for animation delays) */
		style?: string;
	}

	let {
		thread,
		selected = false,
		variant = 'default',
		onclick,
		onresolve,
		ondelete,
		onarchive,
		onrename,
		style = ''
	}: Props = $props();

	let isHovered = $state(false);
	let isMenuOpen = $state(false);
	let isEditing = $state(false);

	function startEdit() {
		isEditing = true;
	}

	function handleRename(newTitle: string) {
		onrename?.(newTitle);
		isEditing = false;
	}
</script>

<li
	class={[styles.item, selected && styles.selected, isMenuOpen && styles.menuOpen].filter(Boolean).join(' ')}
	role="option"
	aria-selected={selected}
	data-variant={variant}
	{style}
	onmouseenter={() => (isHovered = true)}
	onmouseleave={() => (isHovered = false)}
>
	{#if isEditing}
		<InlineRenameInput
			value={thread.title}
			onSave={handleRename}
			onCancel={() => (isEditing = false)}
			maxLength={100}
		>
			{#snippet icon()}
				<div class={styles.icon}>
					{#if thread.metadata.resolved}
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
							<polyline points="22 4 12 14.01 9 11.01" />
						</svg>
					{:else}
						<svg
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
							<line x1="9" y1="10" x2="15" y2="10" />
							<line x1="9" y1="14" x2="13" y2="14" />
						</svg>
					{/if}
				</div>
			{/snippet}
		</InlineRenameInput>
	{/if}

	<button
		class={[styles.button, isEditing && styles.hidden].filter(Boolean).join(' ')}
		onclick={onclick}
		aria-label="Select thread: {thread.title}"
		type="button"
		tabindex={isEditing ? -1 : 0}
	>
		<div class={styles.icon}>
			{#if thread.metadata.resolved}
				<svg
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
					<polyline points="22 4 12 14.01 9 11.01" />
				</svg>
			{:else}
				<svg
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
					<line x1="9" y1="10" x2="15" y2="10" />
					<line x1="9" y1="14" x2="13" y2="14" />
				</svg>
			{/if}
		</div>
		<div class={styles.content}>
			<span class={styles.title}>{thread.title}</span>
			<span class={styles.meta}>
				<span class={styles.messageCount}>{thread.messageCount} messages</span>
				<span class={styles.dot}>·</span>
				<span class={styles.time}>{formatRelativeTime(thread.updatedAt)}</span>
			</span>
		</div>
	</button>

	{#if !isEditing && (isHovered || isMenuOpen) && (onresolve || onrename || ondelete || onarchive)}
		<ListItemActions
			onresolve={onresolve}
			isResolved={thread.metadata.resolved}
			onrename={onrename ? startEdit : undefined}
			onarchive={onarchive}
			ondelete={ondelete}
			iconSize={16}
			onMenuOpen={() => (isMenuOpen = true)}
			onMenuClose={() => (isMenuOpen = false)}
		/>
	{/if}
</li>
