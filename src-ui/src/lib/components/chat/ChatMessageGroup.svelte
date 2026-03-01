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
	let containerClasses = $derived([styles.group, styles[groupRole]].filter(Boolean).join(' '));
</script>

<div class={containerClasses} role="group" aria-label="{groupRole} message group">
	{#if label}
		<div class={styles.label} aria-hidden="true">
			{label}
		</div>
	{/if}

	<div class={styles.header}>
		<div class={styles.avatar}>
			{#if groupRole === 'user'}
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
					<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
					<circle cx="12" cy="7" r="4" />
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
					<path d="M12 8V4H8" />
					<rect width="16" height="12" x="4" y="8" rx="2" />
					<path d="M2 14h2" />
					<path d="M20 14h2" />
					<path d="M15 13v2" />
					<path d="M9 13v2" />
				</svg>
			{/if}
		</div>
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
