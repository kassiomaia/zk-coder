<script lang="ts">
	/**
	 * ThreadPanel Component
	 * Content panel showing messages within a thread.
	 */
	import styles from './ThreadPanel.module.css';
	import ChatArea from '../ChatArea.svelte';
	import type { Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';
	import { fade, slide } from 'svelte/transition';

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
		/** Handler to reopen the thread */
		onReopenThread?: () => void;
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
		onReopenThread,
		loading = false,
		error
	}: Props = $props();

	// Derive resolved state for styling
	let isResolved = $derived(thread.metadata.resolved);

	// Local state for animation
	let isReopening = $state(false);
	let bannerVisible = $state(!isResolved); // Start hidden if resolved
	let bannerAnimatingOut = $state(false);
	let bannerAnimatingIn = $state(false);

	// Track previous thread ID and resolved state to detect thread switches
	let previousThreadId = $state(thread.id);
	let previousResolved = $state(isResolved);

	// Watch for thread changes and resolved status changes
	$effect(() => {
		const currentThreadId = thread.id;
		const currentResolved = isResolved;

		// Check if thread changed OR if resolved status changed for the same thread
		const threadChanged = currentThreadId !== previousThreadId;
		const resolvedChanged = currentResolved !== previousResolved;

		if (threadChanged || resolvedChanged) {
			const wasResolved = previousResolved;
			const nowResolved = currentResolved;

			// Reset animation states
			bannerAnimatingOut = false;
			bannerAnimatingIn = false;

			if (!wasResolved && nowResolved) {
				// Thread became resolved (either via switch or clicking resolve)
				// Animate input out first, then show banner with slide-in animation
				setTimeout(() => {
					// Render the banner first
					bannerVisible = true;

					// Apply animation class after a microtask to ensure browser registers the element
					Promise.resolve().then(() => {
						bannerAnimatingIn = true;
						setTimeout(() => {
							bannerAnimatingIn = false;
						}, 200);
					});
				}, 150);
			} else if (wasResolved && !nowResolved) {
				// Thread became unresolved (either via switch or clicking reopen)
				// Hide banner immediately
				bannerVisible = false;
			} else if (nowResolved) {
				// Already resolved, just show banner without animation
				bannerVisible = true;
			} else {
				// Not resolved, hide banner
				bannerVisible = false;
			}

			previousThreadId = currentThreadId;
			previousResolved = currentResolved;
		}
	});

	async function handleReopen() {
		if (!onReopenThread || isReopening) return;

		isReopening = true;

		// First, animate the banner out (slide down)
		bannerAnimatingOut = true;

		// Wait for banner to finish sliding out (200ms)
		await new Promise(resolve => setTimeout(resolve, 200));

		bannerVisible = false;
		bannerAnimatingOut = false;

		// Then call the reopen handler which will update the thread state
		await onReopenThread();

		// Wait for input animation to complete
		await new Promise(resolve => setTimeout(resolve, 200));

		isReopening = false;
	}
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
		<ChatArea
			{messages}
			{loading}
			{error}
			{onSendMessage}
			{onRetry}
			threadResolved={isResolved}
		/>
	</div>

	<!-- Resolved message - neutral, informational styling -->
	{#if isResolved && bannerVisible}
		<div class={[styles.resolvedMessage, bannerAnimatingOut && styles.animateOut, bannerAnimatingIn && styles.animateIn].filter(Boolean).join(' ')}>
			<button
				class={styles.reopenButton}
				onclick={handleReopen}
				disabled={isReopening}
				type="button"
				aria-label="Reopen thread"
				title="Reopen thread"
			>
				{#if isReopening}
					<svg class={styles.spinning} width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<path d="M21 12a9 9 0 1 1-6.219-8.56" />
					</svg>
				{:else}
					<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
						<path d="M7 11V7a5 5 0 0 1 10 0v4" />
					</svg>
				{/if}
			</button>
			<div class={styles.resolvedContent}>
				<p class={styles.resolvedTitle}>This thread has been resolved</p>
				<p class={styles.resolvedHint}>Click the lock icon to reopen the thread.</p>
			</div>
		</div>
	{/if}
</div>
