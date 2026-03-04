<script lang="ts">
	/**
	 * ChatMessage Component
	 * Displays individual messages in a conversation with support for user, assistant, and system roles.
	 */
	import styles from './ChatMessage.module.css';
	import ThreadIndicator from './chat/ThreadIndicator.svelte';
	import MarkdownRenderer from './markdown/MarkdownRenderer.svelte';
	import AnsweringIndicator from './chat/AnsweringIndicator.svelte';
	import type { MessageStatus } from '$lib/types/message';

	interface Props {
		/** Unique identifier for the message */
		id: string;

		/** Role of the message sender */
		role: 'user' | 'assistant' | 'system';

		/** Message content (HTML rendered from markdown) */
		content: string;

		/** When the message was sent */
		timestamp: Date;

		/** Message status */
		status?: MessageStatus;

		/** Thread ID if message belongs to a thread */
		threadId?: string;

		/** Thread title for display (required if threadId is set) */
		threadTitle?: string;

		/** Handler when thread indicator is clicked */
		onThreadClick?: () => void;

		/** Edit handler */
		onEdit?: () => void;

		/** Delete handler */
		onDelete?: () => void;

		/** Retry handler for failed messages */
		onRetry?: () => void;
	}

	let {
		id,
		role,
		content,
		timestamp,
		status = 'sent',
		threadId,
		threadTitle,
		onThreadClick,
		onEdit,
		onDelete,
		onRetry
	}: Props = $props();

	let isHovered = $state(false);

	let messageClasses = $derived(
		[
			styles.message,
			styles[role],
			status === 'sending' && styles.sending,
			status === 'error' && styles.error
		]
			.filter(Boolean)
			.join(' ')
	);

	function formatTime(date: Date): string {
		return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}
</script>

<article
	class={messageClasses}
	aria-label="{role} message"
	onmouseenter={() => (isHovered = true)}
	onmouseleave={() => (isHovered = false)}
>
	{#if role === 'assistant'}
		<span class={styles.sender}>Assistant</span>
	{/if}

	<div class={styles.content}>
		{#if role === 'assistant' && (status === 'sending' || !content || content.trim() === '')}
			<AnsweringIndicator />
		{:else}
			<MarkdownRenderer {content} class={role === 'user' ? styles.userMarkdown : ''} />
		{/if}
	</div>

	<footer class={styles.footer}>
		{#if threadId && threadTitle}
			<ThreadIndicator
				thread={{ id: threadId, title: threadTitle } as any}
				onclick={onThreadClick}
				compact
			/>
		{/if}

		<time datetime={timestamp.toISOString()} class={styles.timestamp}>
			{formatTime(timestamp)}
		</time>

		{#if status === 'error' && onRetry}
			<button
				class={styles.retryButton}
				onclick={onRetry}
				aria-label="Retry sending message"
				type="button"
			>
				Retry
			</button>
		{/if}

		{#if isHovered && role !== 'system' && (onEdit || onDelete)}
			<div class={styles.actions}>
				{#if onEdit}
					<button
						class={styles.actionButton}
						onclick={onEdit}
						aria-label="Edit message"
						type="button"
					>
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
							<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
							<path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
						</svg>
					</button>
				{/if}
				{#if onDelete}
					<button
						class={styles.actionButton}
						onclick={onDelete}
						aria-label="Delete message"
						type="button"
					>
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
							<polyline points="3 6 5 6 21 6" />
							<path
								d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
							/>
						</svg>
					</button>
				{/if}
			</div>
		{/if}
	</footer>
</article>
