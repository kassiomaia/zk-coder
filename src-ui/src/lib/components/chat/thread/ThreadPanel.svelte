<script lang="ts">
	/**
	 * ThreadPanel Component
	 * Content panel showing messages within a thread.
	 */
	import styles from './ThreadPanel.module.css';
	import ChatArea from '../ChatArea.svelte';
	import type { Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';

	interface Props {
		/** Current thread being displayed */
		thread: Thread;
		/** Messages in the thread */
		messages: Message[];
		/** Breadcrumb path for navigation */
		breadcrumbPath?: { id: string; title: string }[];
		/** Handler when breadcrumb is clicked */
		onNavigate?: (id: string) => void;
		/** Handler to send a message */
		onSendMessage?: (content: string) => void;
		/** Handler for retry */
		onRetry?: () => void;
		/** Loading state */
		loading?: boolean;
		/** Error state */
		error?: string;
	}

	let {
		thread,
		messages,
		breadcrumbPath = [],
		onNavigate,
		onSendMessage,
		onRetry,
		loading = false,
		error
	}: Props = $props();
</script>

<div class={styles.panel}>
	<!-- Header with context and title -->
	<header class={styles.header}>
		{#if breadcrumbPath.length > 0}
			<span class={styles.context}>{breadcrumbPath[0].title}</span>
		{/if}
		<h1 class={styles.title}>
			{thread.title}
			{#if thread.metadata.resolved}
				<span class={styles.resolvedIcon} title="Resolved">
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
				</span>
			{/if}
		</h1>
		{#if thread.metadata.summary}
			<p class={styles.summary}>{thread.metadata.summary}</p>
		{/if}
	</header>

	<!-- Chat area -->
	<div class={styles.content}>
		<ChatArea {messages} {loading} {error} {onSendMessage} {onRetry} />
	</div>

	<!-- Resolved message - subtle styling -->
	{#if thread.metadata.resolved}
		<div class={styles.resolvedMessage}>
			<p>This thread has been resolved. Reopen it to continue the conversation.</p>
		</div>
	{/if}
</div>
