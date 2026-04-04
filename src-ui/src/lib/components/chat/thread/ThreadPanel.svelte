<script lang="ts">
	/**
	 * ThreadPanel Component
	 * Content panel showing messages within a thread.
	 */
	import styles from './ThreadPanel.module.css';
	import ChatArea from '../ChatArea.svelte';
	import Button from '../../shared/Button.svelte';
	import Tooltip from '../../shared/Tooltip.svelte';
	import { chatStore, unreadThreadIds } from '$lib/stores';
	import type { Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';
	import type { AgentStatus } from '$lib/utils/agentStatus';
	import { slide } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';

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
		onSendMessage?: (content: string, options?: { modelId?: string }) => void;
		/** Handler to open settings */
		onOpenSettings?: () => void;
		/** Handler for retry */
		onRetry?: () => void;
		/** Handler to reopen the thread */
		onReopenThread?: () => void;
		/** Handler to toggle the thread sidebar */
		onToggleSidebar?: () => void;
		/** Initial model ID from conversation settings */
		initialModelId?: string | null;
		/** Handler when model selection changes */
		onModelChange?: (modelId: string) => void;
		/** Handler for message feedback (like/dislike) */
		onMessageFeedback?: (messageId: string, feedback: 'like' | 'dislike' | null) => void;
		/** Handler to approve an agent action */
		onApproveAction?: (sessionId: string, actionId: string) => void;
		/** Handler to deny an agent action */
		onDenyAction?: (sessionId: string, actionId: string) => void;
		/** Handler to continue execution from paused state */
		onContinueExecution?: (sessionId: string) => void;
		/** Handler to replan and continue from paused state */
		onReplanExecution?: (sessionId: string) => void;
		/** Loading state */
		loading?: boolean;
		/** Error state */
		error?: string;
		/** Whether agent/LLM is currently active */
		isTyping?: boolean;
		/** Status for the thread-linked agent session */
		agentStatus?: AgentStatus | null;
		/** Session ID linked to the visible thread */
		agentSessionId?: string | null;
	}

	let {
		thread,
		messages,
		breadcrumbPath = [],
		onNavigate,
		onSendMessage,
		onOpenSettings,
		onRetry,
		onReopenThread,
		onToggleSidebar,
		initialModelId = null,
		onModelChange,
		onMessageFeedback,
		onApproveAction,
		onDenyAction,
		onContinueExecution,
		onReplanExecution,
		loading = false,
		error,
		isTyping = false,
		agentStatus = null,
		agentSessionId = null
	}: Props = $props();

	// Derive resolved state for styling
	let isResolved = $derived(thread.metadata.resolved);

	// Only clear unread on visible-thread changes or unread transitions to avoid re-entrant store churn.
	let previousVisibleThreadId = $state<string | null>(null);
	let previousVisibleThreadUnread = $state(false);

	$effect(() => {
		const currentThreadId = thread?.id;
		const isUnread = currentThreadId ? $unreadThreadIds.includes(currentThreadId) : false;

		if (!currentThreadId) {
			previousVisibleThreadId = null;
			previousVisibleThreadUnread = false;
			return;
		}

		const threadChanged = currentThreadId !== previousVisibleThreadId;
		const unreadBecameVisible = isUnread && !previousVisibleThreadUnread;

		if (threadChanged || unreadBecameVisible) {
			previousVisibleThreadId = currentThreadId;
			previousVisibleThreadUnread = isUnread;

			if (isUnread) {
				chatStore.markThreadRead(currentThreadId);
			}
			return;
		}

		previousVisibleThreadId = currentThreadId;
		previousVisibleThreadUnread = isUnread;
	});

	// Local state for animation
	let isReopening = $state(false);
	let bannerVisible = $state(true);
	let bannerAnimatingOut = $state(false);
	let bannerAnimatingIn = $state(false);
	let isExpanded = $state(false);
	let scrollOpacity = $state(1);
	let restrictedFeedback = $state(false);

	// Crossfade for icons
	// Using simple fade for now as crossfade has type issues in strict mode
	// and for simple icon swaps fade is often sufficient

	// Timer for restricted feedback - cleaned up on component destroy
	let feedbackTimer: ReturnType<typeof setTimeout> | null = null;

	// Cleanup all timers on component destroy
	$effect(() => {
		return () => {
			if (feedbackTimer) {
				clearTimeout(feedbackTimer);
				feedbackTimer = null;
			}
		};
	});

	function triggerRestrictedFeedback() {
		restrictedFeedback = true;
		// Clear any existing timer before setting a new one
		if (feedbackTimer) {
			clearTimeout(feedbackTimer);
		}
		feedbackTimer = setTimeout(() => {
			restrictedFeedback = false;
			feedbackTimer = null;
		}, 300);
	}

	// Track previous thread ID and resolved state to detect thread switches
	let previousThreadId = $state('');
	let previousResolved = $state(false);
	let initialized = $state(false);

	function handleChatScroll(scrollTop: number, scrollHeight: number, clientHeight: number) {
		const distanceToBottom = scrollHeight - scrollTop - clientHeight;
		// Modulate opacity: 1 when at bottom, fades to 0.6 as you scroll up
		// This prevents the banner from dominating when reading history
		const newOpacity = Math.max(0.6, 1 - Math.min(distanceToBottom / 300, 0.4));
		if (Math.abs(scrollOpacity - newOpacity) > 0.05) {
			scrollOpacity = newOpacity;
		}
	}

	function toggleExpanded() {
		isExpanded = !isExpanded;
	}

	// Watch for thread changes and resolved status changes
	$effect(() => {
		const currentThreadId = thread.id;
		const currentResolved = isResolved;

		// First run - initialize state without animation
		if (!initialized) {
			previousThreadId = currentThreadId;
			previousResolved = currentResolved;
			bannerVisible = currentResolved;
			initialized = true;
			return;
		}

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
		await new Promise((resolve) => setTimeout(resolve, 200));

		bannerVisible = false;
		bannerAnimatingOut = false;

		// Then call the reopen handler which will update the thread state
		await onReopenThread();

		// Wait for input animation to complete
		await new Promise((resolve) => setTimeout(resolve, 200));

		isReopening = false;
	}
</script>

<div class={styles.panel}>
	<!-- Header with context and title -->
	<header class={styles.header}>
		<div class={styles.leftActions}>
			{#if onToggleSidebar}
				<button
					class={styles.sidebarToggle}
					onclick={onToggleSidebar}
					aria-label="Toggle sidebar"
					type="button"
				>
					<svg
						width="18"
						height="18"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<line x1="3" y1="12" x2="21" y2="12" />
						<line x1="3" y1="6" x2="21" y2="6" />
						<line x1="3" y1="18" x2="21" y2="18" />
					</svg>
				</button>
			{/if}

			<div class={styles.identity}>
				{#if breadcrumbPath.length > 0}
					<span class={styles.context}>{breadcrumbPath[0].title}</span>
				{/if}
				<h1 class={styles.title}>
					{thread.title}
				</h1>
			</div>
		</div>

		{#if thread.metadata.resolved}
			<div class={styles.statusIndicator}>
				<Tooltip
					content="This thread is resolved and locked. It is currently in read-only mode to preserve the conversation state."
					position="bottom"
				>
					<div class={styles.resolvedBadge}>
						<span class={styles.resolvedIcon}>
							<svg
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2.5"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
								<polyline points="22 4 12 14.01 9 11.01" />
							</svg>
						</span>
						<span class={styles.resolvedText}>Resolved</span>
					</div>
				</Tooltip>
			</div>
		{/if}
	</header>
	<div class={styles.headerDivider}></div>

	{#if thread.metadata.summary}
		<div class={styles.summaryContainer}>
			<p class={styles.summary}>{thread.metadata.summary}</p>
		</div>
	{/if}

	<!-- Chat area -->
	<div class={styles.content}>
		<ChatArea
			{messages}
			threadId={thread.id}
			{loading}
			{error}
			{isTyping}
			{agentStatus}
			{agentSessionId}
			{onSendMessage}
			{onOpenSettings}
			{onRetry}
			threadResolved={isResolved}
			{initialModelId}
			{onModelChange}
			{onMessageFeedback}
			{onApproveAction}
			{onDenyAction}
			{onContinueExecution}
			{onReplanExecution}
			onScroll={(scrollTop, scrollHeight, clientHeight) =>
				handleChatScroll(scrollTop, scrollHeight, clientHeight)}
		/>
	</div>

	<!-- Resolved message - neutral, informational styling -->
	{#if isResolved && bannerVisible}
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div
			class={[
				styles.resolvedMessage,
				bannerAnimatingOut && styles.animateOut,
				bannerAnimatingIn && styles.animateIn,
				isExpanded && styles.expanded,
				restrictedFeedback && styles.restrictedFeedback
			]
				.filter(Boolean)
				.join(' ')}
			style:--scroll-opacity={scrollOpacity}
			onclick={triggerRestrictedFeedback}
			onkeydown={(e) => e.key === 'Enter' && triggerRestrictedFeedback()}
			role="alert"
			aria-live="polite"
		>
			<div class={styles.statusIconContainer}>
				<svg
					width="20"
					height="20"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
					<path d="M7 11V7a5 5 0 0 1 10 0v4" />
				</svg>
			</div>

			<button
				class={styles.resolvedContent}
				onclick={(e) => {
					e.stopPropagation();
					toggleExpanded();
				}}
				type="button"
				aria-expanded={isExpanded}
			>
				<div class={styles.statusHeader}>
					<span class={styles.resolvedLabel}>Thread Locked</span>
					<span class={styles.resolvedSubtext}>Read-only mode active</span>
				</div>

				{#if isExpanded}
					<div transition:slide={{ duration: 200, easing: cubicOut }} class={styles.detailsWrapper}>
						<p class={styles.resolvedHint}>This thread is locked. Unlock to continue messaging.</p>
					</div>
				{/if}
			</button>

			<div class={styles.actionControl}>
				<Button
					variant="primary"
					size="sm"
					onclick={handleReopen}
					disabled={isReopening}
					loading={isReopening}
					class={styles.reopenButton}
				>
					Unlock
				</Button>

				<svg
					class={[styles.chevron, isExpanded && styles.rotated].filter(Boolean).join(' ')}
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<polyline points="6 9 12 15 18 9"></polyline>
				</svg>
			</div>
		</div>
	{/if}
</div>
