<script lang="ts">
	/**
	 * ThreadPanel Component
	 * Content panel showing messages within a thread.
	 */
	import styles from './ThreadPanel.module.css';
	import ThreadBreadcrumbs from './ThreadBreadcrumbs.svelte';
	import ChatArea from '../ChatArea.svelte';
	import StatusBadge from '../shared/StatusBadge.svelte';
	import type { Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';
	import type { StatusVariant } from '../shared/StatusBadge.svelte';

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

	let statusVariant = $derived<StatusVariant>(thread.metadata.resolved ? 'resolved' : 'open');
</script>

<div class={styles.panel}>
	<!-- Header with breadcrumbs -->
	<header class={styles.header}>
		{#if breadcrumbPath.length > 0}
			<ThreadBreadcrumbs path={breadcrumbPath} {onNavigate} />
		{/if}
		<div class={styles.titleRow}>
			<h1 class={styles.title}>{thread.title}</h1>
			<StatusBadge variant={statusVariant} showIcon={thread.metadata.resolved} />
		</div>
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
