<script lang="ts">
	/**
	 * ThreadItem Component
	 * Individual thread item in the thread list.
	 */
	import styles from './ThreadItem.module.css';
	import ListItemActions from '$lib/components/shared/ListItemActions.svelte';
	import { formatRelativeTime } from '$lib/utils/formatRelativeTime';
	import type { Thread } from '$lib/types/thread';

	interface Props {
		/** Thread data */
		thread: Thread;
		/** Whether this thread is selected */
		selected?: boolean;
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
	}

	let { thread, selected = false, onclick, onresolve, ondelete, onarchive, onrename }: Props = $props();

	let showActions = $state(false);
	let isEditing = $state(false);
	let editTitle = $state('');
	let inputRef: HTMLInputElement | undefined = $state();

	function startEdit() {
		editTitle = thread.title;
		isEditing = true;
		setTimeout(() => inputRef?.focus(), 0);
	}

	function finishEdit() {
		if (editTitle.trim() && editTitle !== thread.title) {
			onrename?.(editTitle.trim());
		}
		isEditing = false;
	}

	function cancelEdit() {
		isEditing = false;
		editTitle = '';
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			finishEdit();
		} else if (e.key === 'Escape') {
			e.preventDefault();
			cancelEdit();
		}
	}
</script>

<li
	class={[styles.item, selected && styles.selected].filter(Boolean).join(' ')}
	role="option"
	aria-selected={selected}
	onmouseenter={() => (showActions = true)}
	onmouseleave={() => (showActions = false)}
>
	{#if isEditing}
		<div class={styles.editWrapper}>
			<input
				bind:this={inputRef}
				class={styles.editInput}
				type="text"
				bind:value={editTitle}
				onkeydown={handleKeydown}
				onblur={finishEdit}
				maxlength={100}
			/>
		</div>
	{:else}
		<button
			class={styles.button}
			onclick={onclick}
			aria-label="Select thread: {thread.title}"
			type="button"
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

		{#if showActions && (onresolve || onrename || ondelete || onarchive)}
			<ListItemActions
				onresolve={onresolve}
				isResolved={thread.metadata.resolved}
				onrename={onrename ? startEdit : undefined}
				onarchive={onarchive}
				ondelete={ondelete}
				iconSize={16}
			/>
		{/if}
	{/if}
</li>
