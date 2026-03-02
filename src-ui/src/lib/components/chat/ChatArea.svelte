<script lang="ts">
	/**
	 * ChatArea Component (Enhanced)
	 * Main chat interface with message display and input.
	 */
	import ChatInput from './input/ChatInput.svelte';
	import AnsweringIndicator from './AnsweringIndicator.svelte';
	import ThreadIndicator from './ThreadIndicator.svelte';
	import MarkdownRenderer from '../markdown/MarkdownRenderer.svelte';
	import type { Message } from '$lib/types/message';
	import type { Thread } from '$lib/types/thread';
	import styles from './ChatArea.module.css';

	interface Props {
		/** Array of messages to display */
		messages: Message[];

		/** Available threads for looking up thread info */
		threads?: Thread[];

		/** Loading state */
		loading?: boolean;

		/** Is assistant typing */
		isTyping?: boolean;

		/** Error message to display */
		error?: string;

		/** Loading state for messages */
		messagesLoading?: 'idle' | 'loading' | 'success' | 'error';

		/** Thread is resolved - disables input */
		threadResolved?: boolean;

		/** Send message handler */
		onSendMessage?: (content: string) => void;

		/** Retry handler */
		onRetry?: () => void;

		/** Edit message handler */
		onEditMessage?: (id: string) => void;

		/** Delete message handler */
		onDeleteMessage?: (id: string) => void;

		/** Retry message handler (for failed messages) */
		onRetryMessage?: (id: string) => void;

		/** Thread click handler */
		onThreadClick?: (threadId: string) => void;
	}

	let {
		messages,
		threads = [],
		loading = false,
		isTyping = false,
		error,
		messagesLoading = 'idle',
		threadResolved = false,
		onSendMessage,
		onRetry,
		onEditMessage,
		onDeleteMessage,
		onRetryMessage,
		onThreadClick
	}: Props = $props();

	// Input state
	let inputValue = $state('');

	// Animation state for input container
	let inputAnimatingIn = $state(false);
	let inputAnimatingOut = $state(false);
	let showInput = $state(!threadResolved);
	let previousResolved = $state(threadResolved);

	// Watch for threadResolved changes and animate transitions
	$effect(() => {
		const currentResolved = threadResolved;

		// Only animate when the resolved state actually changes
		if (currentResolved !== previousResolved) {
			const wasResolved = previousResolved;
			const nowResolved = currentResolved;

			previousResolved = currentResolved;

			if (wasResolved && !nowResolved) {
				// Transition: locked -> unlocked
				// Wait for banner to slide out (200ms), then slide input up
				setTimeout(() => {
					showInput = true;
					inputAnimatingIn = true;
					setTimeout(() => {
						inputAnimatingIn = false;
					}, 200);
				}, 200);
			} else if (!wasResolved && nowResolved) {
				// Transition: unlocked -> locked
				// Animate input out (slide down), then banner will slide in
				inputAnimatingOut = true;
				setTimeout(() => {
					showInput = false;
					inputAnimatingOut = false;
				}, 150);
			}
			// If wasResolved and nowResolved: both locked - no animation (stay hidden)
			// If !wasResolved and !nowResolved: both unlocked - no animation (stay visible)
		}
	});

	// Group messages by time and role
	interface MessageGroup {
		messages: Message[];
		role: 'user' | 'assistant' | 'system';
		timestamp: Date;
	}

	let messageGroups = $derived(() => {
		if (!messages || messages.length === 0) return [];

		const groups: MessageGroup[] = [];
		let currentGroup: Message[] = [];

		for (const message of messages) {
			if (currentGroup.length === 0) {
				currentGroup.push(message);
			} else {
				const lastMessage = currentGroup[currentGroup.length - 1];
				const timeDiff = message.timestamp.getTime() - lastMessage.timestamp.getTime();
				const sameRole = message.role === lastMessage.role;

				// Group if within 5 minutes and same role
				if (timeDiff < 5 * 60 * 1000 && sameRole) {
					currentGroup.push(message);
				} else {
					groups.push({
						messages: currentGroup,
						role: currentGroup[0].role,
						timestamp: currentGroup[0].timestamp
					});
					currentGroup = [message];
				}
			}
		}

		if (currentGroup.length > 0) {
			groups.push({
				messages: currentGroup,
				role: currentGroup[0].role,
				timestamp: currentGroup[0].timestamp
			});
		}

		return groups;
	});

	function handleSubmit() {
		if (inputValue.trim() && onSendMessage) {
			onSendMessage(inputValue.trim());
			inputValue = '';
		}
	}

	function handleRetry() {
		onRetry?.();
	}

	function formatGroupTime(date: Date): string {
		return new Date(date).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	function getRoleClass(role: string): string {
		return `role-${role}`;
	}

	function getMessageStatusClass(status: string | undefined): string {
		if (status === 'error') return styles.messageError;
		if (status === 'sending') return styles.messageSending;
		return '';
	}

	/**
	 * Get thread title by ID.
	 */
	function getThreadTitle(threadId: string | undefined): string | undefined {
		if (!threadId) return undefined;
		return threads.find((t) => t.id === threadId)?.title;
	}
</script>

<div class={styles.chatArea}>
	<!-- Messages area -->
	<div class={styles.messagesWrapper}>
		{#if messagesLoading === 'loading'}
			<div class={styles.loading}>
				<div class={styles.spinner}></div>
				<span>Loading messages...</span>
			</div>
		{:else if messagesLoading === 'error' || error}
			<div class={styles.error}>
				<svg
					class={styles.errorIcon}
					width="48"
					height="48"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<circle cx="12" cy="12" r="10" />
					<line x1="12" y1="8" x2="12" y2="12" />
					<line x1="12" y1="16" x2="12.01" y2="16" />
				</svg>
				<p class={styles.errorMessage}>{error || 'Failed to load messages'}</p>
				{#if onRetry}
					<button class={styles.retryButton} onclick={handleRetry} type="button">
						Retry
					</button>
				{/if}
			</div>
		{:else if messages.length === 0}
			<div class={styles.empty}>
				<div class={styles.emptyHeader}>
					<svg
						class={styles.emptyIcon}
						width="20"
						height="20"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
					</svg>
					<span class={styles.emptyText}>Start a conversation</span>
				</div>
				<p class={styles.emptyHint}>Type a message below to begin</p>
			</div>
		{:else}
			<!-- Message groups -->
			{#each messageGroups() as group}
				<div class={styles.messageGroup}>
					<div class={styles.groupHeader}>
						<span class={styles.groupRole}>{group.role}</span>
						<span class={styles.groupTime}>{formatGroupTime(group.timestamp)}</span>
					</div>
					<div class={styles.groupMessages}>
						{#each group.messages as message (message.id)}
							<div class={[styles.message, styles[getRoleClass(message.role)], getMessageStatusClass(message.status)].filter(Boolean).join(' ')}>
								<!-- Thread indicator -->
								{#if message.threadId && getThreadTitle(message.threadId)}
									<ThreadIndicator
										thread={{ id: message.threadId, title: getThreadTitle(message.threadId)! } as any}
										onclick={() => onThreadClick?.(message.threadId!)}
										compact
									/>
								{/if}
								<div class={styles.messageContent}>
									<MarkdownRenderer content={message.content} />
								</div>
								<!-- Message status indicator -->
								{#if message.status === 'sending' && message.content.trim() === ''}
									<AnsweringIndicator />
								{:else if message.status === 'error'}
									<div class={styles.statusIndicator} aria-label="Failed to send">
										<button
											class={styles.retryIconButton}
											onclick={() => onRetryMessage?.(message.id)}
											title="Retry sending"
											type="button"
										>
											<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
												<path d="M1 4v6h6" />
												<path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
											</svg>
										</button>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/each}

			<!-- Answering indicator -->
			{#if isTyping}
				<AnsweringIndicator />
			{/if}
		{/if}
	</div>

	<!-- Input area -->
	{#if showInput}
		<div class={[styles.inputContainer, inputAnimatingIn && styles.animateIn].filter(Boolean).join(' ')}>
			<ChatInput
				value={inputValue}
				onchange={(v) => (inputValue = v)}
				onsubmit={handleSubmit}
				placeholder="Send a message..."
				loading={loading}
				showIntent={true}
			/>
		</div>
	{/if}
</div>
