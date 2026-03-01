<script lang="ts">
	/**
	 * ThreadPanel Component
	 * Content panel showing messages within a thread.
	 */
	import styles from './ThreadPanel.module.css';
	import ThreadBreadcrumbs from './ThreadBreadcrumbs.svelte';
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

	let inputText = $state('');

	function handleSubmit() {
		if (inputText.trim() && onSendMessage) {
			onSendMessage(inputText.trim());
			inputText = '';
		}
	}
</script>

<div class={styles.panel}>
	<!-- Header with breadcrumbs -->
	<header class={styles.header}>
		{#if breadcrumbPath.length > 0}
			<ThreadBreadcrumbs path={breadcrumbPath} onNavigate={onNavigate} />
		{/if}
		<div class={styles.titleRow}>
			<h1 class={styles.title}>{thread.title}</h1>
			{#if thread.metadata.resolved}
				<span class={styles.resolvedBadge}>
					<svg
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="20 6 9 17 4 12" />
					</svg>
					Resolved
				</span>
			{/if}
		</div>
		{#if thread.metadata.summary}
			<p class={styles.summary}>{thread.metadata.summary}</p>
		{/if}
	</header>

	<!-- Chat area -->
	<div class={styles.content}>
		<ChatArea
			{messages}
			{loading}
			{error}
			onRetry={onRetry}
		/>
	</div>

	<!-- Input area -->
	{#if !thread.metadata.resolved}
		<div class={styles.input}>
			<textarea
				class={styles.textarea}
				bind:value={inputText}
				placeholder="Type your message..."
				onkeydown={(e) => {
					if (e.key === 'Enter' && !e.shiftKey) {
						e.preventDefault();
						handleSubmit();
					}
				}}
				rows={1}
				aria-label="Message input"
			></textarea>
			<button
				class={styles.sendButton}
				onclick={handleSubmit}
				disabled={!inputText.trim()}
				aria-label="Send message"
				type="button"
			>
				<svg
					width="18"
					height="18"
					viewBox="0 0 24 24"
					fill="currentColor"
				>
					<path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
				</svg>
			</button>
		</div>
	{:else}
		<div class={styles.resolvedMessage}>
			<p>This thread has been resolved. Reopen it to continue the conversation.</p>
		</div>
	{/if}
</div>
