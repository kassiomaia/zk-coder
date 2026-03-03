<script lang="ts">
	/**
	 * ConversationItem Component
	 * Individual conversation item in the sidebar list.
	 */
	import styles from './ConversationItem.module.css';
	import ListItemActions from '$lib/components/shared/ListItemActions.svelte';
	import { formatRelativeTime } from '$lib/utils/formatRelativeTime';
	import type { Conversation } from '$lib/types/thread';

	interface Props {
		/** Conversation data */
		conversation: Conversation;
		/** Whether this conversation is selected */
		selected?: boolean;
		/** Click handler */
		onclick: () => void;
		/** Double-click handler (typically opens thread panel if threads exist) */
		ondoubleclick?: () => void;
		/** Delete handler */
		ondelete?: () => void;
		/** Rename handler */
		onrename?: (newTitle: string) => void;
		/** Archive handler */
		onarchive?: () => void;
	}

	let { conversation, selected = false, onclick, ondoubleclick, ondelete, onrename, onarchive }: Props = $props();

	let showActions = $state(false);
	let isEditing = $state(false);
	let editTitle = $state('');
	let inputRef: HTMLInputElement | undefined = $state();

	function startEdit() {
		editTitle = conversation.title;
		isEditing = true;
		setTimeout(() => inputRef?.focus(), 0);
	}

	function finishEdit() {
		if (editTitle.trim() && editTitle !== conversation.title) {
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

	function formatThreadCount(count: number): string {
		return count === 1 ? '1 thread' : `${count} threads`;
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
			ondblclick={() => {
				// Only trigger double-click action if conversation has threads
				if (conversation.threads && conversation.threads.length > 0) {
					ondoubleclick?.();
				}
			}}
			aria-label="Select conversation: {conversation.title}"
			type="button"
		>
			<div class={styles.icon}>
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
				</svg>
			</div>
			<div class={styles.content}>
				<span class={styles.title}>{conversation.title}</span>
				<span class={styles.meta}>
					{#if conversation.threads && conversation.threads.length > 0}
						<span class={styles.threadCount}>{formatThreadCount(conversation.threads.length)}</span>
						<span class={styles.dot}>·</span>
					{/if}
					<span class={styles.time}>{formatRelativeTime(conversation.updatedAt)}</span>
				</span>
			</div>
		</button>

		{#if showActions && (ondelete || onrename || onarchive)}
			<ListItemActions
				ondelete={ondelete}
				onrename={onrename ? startEdit : undefined}
				onarchive={onarchive}
				iconSize={14}
			/>
		{/if}
	{/if}
</li>
