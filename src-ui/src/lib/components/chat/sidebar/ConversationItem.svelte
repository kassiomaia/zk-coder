<script lang="ts">
	/**
	 * ConversationItem Component
	 * Individual conversation item in the sidebar list.
	 */
	import styles from './ConversationItem.module.css';
	import ConversationActions from './ConversationActions.svelte';
	import type { Conversation } from '$lib/types/thread';

	interface Props {
		/** Conversation data */
		conversation: Conversation;
		/** Whether this conversation is selected */
		selected?: boolean;
		/** Click handler */
		onclick: () => void;
		/** Delete handler */
		ondelete?: () => void;
		/** Rename handler */
		onrename?: (newTitle: string) => void;
	}

	let { conversation, selected = false, onclick, ondelete, onrename }: Props = $props();

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

	function formatTime(date: Date): string {
		const d = new Date(date);
		const now = new Date();
		const diff = now.getTime() - d.getTime();
		const hours = Math.floor(diff / 3600000);

		if (hours < 1) return 'Just now';
		if (hours < 24) return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		return d.toLocaleDateString([], { month: 'short', day: 'numeric' });
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
					<span class={styles.time}>{formatTime(conversation.updatedAt)}</span>
				</span>
			</div>
		</button>

		{#if showActions && (ondelete || onrename)}
			<ConversationActions
				ondelete={ondelete}
				onrename={onrename ? startEdit : undefined}
			/>
		{/if}
	{/if}
</li>
