<script lang="ts">
	/**
	 * ChatMessageGroup Component
	 * Groups related messages together with a shared header.
	 * Messages are grouped by time (5-minute intervals) and role.
	 */
	import ChatMessage from '$lib/components/ChatMessage.svelte';
	import type { Message } from '$lib/types/message';
	import styles from './ChatMessageGroup.module.css';

	interface Props {
		/** Messages in this group */
		messages: Message[];

		/** Optional label for the group (e.g., "Today") */
		label?: string;

		/** Handler for editing a message */
		onEdit?: (id: string) => void;

		/** Handler for deleting a message */
		onDelete?: (id: string) => void;

		/** Handler for retrying a failed message */
		onRetry?: (id: string) => void;
	}

	let { messages, label, onEdit, onDelete, onRetry }: Props = $props();

	/**
	 * Format the timestamp for the group header.
	 */
	function formatGroupTime(messages: Message[]): string {
		if (messages.length === 0) return '';

		const first = messages[0];
		const role = first.role;

		// Format based on role
		if (role === 'assistant') {
			return formatTime(first.timestamp);
		}

		// For user, show the time of the first message
		return formatTime(first.timestamp);
	}

	function formatTime(date: Date): string {
		return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	/**
	 * Get the role for this group (all messages have same role).
	 */
	let groupRole = $derived(messages.length > 0 ? messages[0].role : 'user');

	/**
	 * Classes for the group container.
	 */
	let containerClasses = $derived(
		[styles.group, styles[groupRole]].filter(Boolean).join(' ')
	);
</script>

<div class={containerClasses} role="group" aria-label="{groupRole} message group">
	{#if label}
		<div class={styles.label} aria-hidden="true">
			{label}
		</div>
	{/if}

	<div class={styles.header}>
		<span class={styles.role}>{groupRole === 'user' ? 'You' : 'Assistant'}</span>
		<time class={styles.time} datetime={messages[0]?.timestamp.toISOString()}>
			{formatGroupTime(messages)}
		</time>
	</div>

	<div class={styles.messages}>
		{#each messages as message (message.id)}
			<div class={styles.messageWrapper}>
				<ChatMessage
					id={message.id}
					role={message.role}
					content={message.content}
					timestamp={message.timestamp}
					status={message.status}
					onEdit={onEdit ? () => onEdit(message.id) : undefined}
					onDelete={onDelete ? () => onDelete(message.id) : undefined}
					onRetry={onRetry ? () => onRetry(message.id) : undefined}
				/>
			</div>
		{/each}
	</div>
</div>
