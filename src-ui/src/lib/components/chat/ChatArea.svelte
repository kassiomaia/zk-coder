<script lang="ts">
	/**
	 * ChatArea Component (Enhanced)
	 * Main chat interface with message display and input.
	 */
	import ChatInput from './input/ChatInput.svelte';
	import AnsweringIndicator from './AnsweringIndicator.svelte';
	import ThreadIndicator from './ThreadIndicator.svelte';
	import MarkdownRenderer from '../markdown/MarkdownRenderer.svelte';
	import MessageActions from './MessageActions.svelte';
	import EmptyThreadState from './EmptyThreadState.svelte';
	import ClarificationBanner from './ClarificationBanner.svelte';
	import type { Message } from '$lib/types/message';
	import type { Thread } from '$lib/types/thread';
	import styles from './ChatArea.module.css';
	import { isLlmStreaming, streamingContent, nlpState } from '$lib/stores/chat';
	import { notifyAssistantComplete, initNotifications } from '$lib/utils/notifications';
	import { configStore } from '$lib/stores/configStore';
	import {
		saveScrollPosition,
		restoreScrollPosition,
		hasSavedPosition,
		initializeThreadScroll,
		cleanupThreadScroll
	} from '$lib/utils/scrollManager';
	import {
		registerThreadElement,
		unregisterThread,
		handleStreamingStart,
		handleStreamingEnd,
		handleContentUpdate,
		handleUserScroll as controllerHandleUserScroll,
		scrollToBottom as controllerScrollToBottom,
		enableAutoScroll as controllerEnableAutoScroll,
		isAutoScrollEnabled as controllerIsAutoScrollEnabled
	} from '$lib/utils/scrollController';
	import { onMount } from 'svelte';
	import { chatStore } from '$lib/stores';
	import type { ResolutionType } from '$lib/types/nlp';

	onMount(() => {
		initNotifications();
		// Initialize model selection from config default
		const defaultModel = configStore.getDefault('code');
		if (defaultModel) {
			selectedModelId = defaultModel;
		}
	});

	interface Props {
		/** Array of messages to display */
		messages: Message[];

		/** Current thread ID for scroll position tracking */
		threadId?: string;

		/** Available threads for looking up thread info */
		threads?: Thread[];

		/** Empty state context: 'none' (no conv), 'conversation' (conv but no thread), 'thread' (thread selected) */
		emptyContext?: 'none' | 'conversation' | 'thread';

		/** Optional conversation title for empty state context */
		conversationTitle?: string;

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

		/** Send message handler - can accept content string or object with content and modelId */
		onSendMessage?: (content: string, options?: { modelId?: string }) => void;

		/** Open settings handler */
		onOpenSettings?: () => void;

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

		/** Scroll handler */
		onScroll?: (scrollTop: number, scrollHeight: number, clientHeight: number) => void;
	}

	let {
		messages,
		threadId,
		threads = [],
		emptyContext = 'thread',
		conversationTitle,
		loading = false,
		isTyping = false,
		error,
		messagesLoading = 'idle',
		threadResolved = false,
		onSendMessage,
		onOpenSettings,
		onRetry,
		onEditMessage,
		onDeleteMessage,
		onRetryMessage,
		onThreadClick,
		onScroll
	}: Props = $props();

	// Input state
	let inputValue = $state('');
	let hasInputActivity = $state(false);
	let hasInputFocus = $state(false);

	// Model selection state
	let selectedModelId = $state<string | null>(null);

	// Animation state for input container
	let inputAnimatingIn = $state(false);
	let inputAnimatingOut = $state(false);
	let showInput = $state(true);
	let previousResolved = $state(false);

	// Scroll state
	let messagesWrapperRef: HTMLDivElement | undefined = $state();
	let previousThreadId = $state<string | undefined>(undefined);
	let isUserScrolling = $state(false);

	// Track if we should show "scroll to bottom" button
	let showScrollButton = $state(false);

	// Track previous streaming state to detect completion
	let wasStreaming = $state(false);

	// Track last scroll position to detect user scrolling UP
	let lastScrollTop = $state(0);

	// Threshold for showing scroll button (larger zone for visibility)
	const SCROLL_BUTTON_THRESHOLD = 100;

	// Watch for streaming state changes using the new controller
	$effect(() => {
		const streaming = $isLlmStreaming;
		const currentThreadId = threadId;
		const ref = messagesWrapperRef;

		console.log('[ChatArea] streaming state effect:', {
			streaming,
			wasStreaming,
			currentThreadId,
			hasRef: !!ref
		});

		if (!ref || !currentThreadId) return;

		// Detect transition from NOT streaming TO streaming
		if (!wasStreaming && streaming) {
			// Streaming just started - notify controller
			console.log('[ChatArea] streaming START detected');
			handleStreamingStart(currentThreadId);
		}

		// Detect transition from streaming to NOT streaming (completion)
		if (wasStreaming && !streaming) {
			// Streaming just completed - notify controller
			console.log('[ChatArea] streaming END detected');
			handleStreamingEnd(currentThreadId);

			// Check if user was scrolled up (auto-scroll was disabled)
			const autoScrollWasEnabled = controllerIsAutoScrollEnabled(currentThreadId);

			if (!autoScrollWasEnabled) {
				// User was scrolled up - send notification
				notifyAssistantComplete(
					'Assistant Complete',
					'The assistant has finished responded. Click to see the response.'
				);
			}
		}

		wasStreaming = streaming;
	});

	// Watch for threadResolved changes and animate transitions
	$effect(() => {
		const currentResolved = threadResolved;

		// First run - initialize state without animation
		if (previousResolved === false && !inputAnimatingIn && !inputAnimatingOut) {
			showInput = !currentResolved;
			previousResolved = currentResolved;
			return;
		}

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
			// If !wasResolved && !nowResolved: both unlocked - no animation (stay visible)
		}
	});

	// Handle thread switching - register/unregister with controller
	$effect(() => {
		const currentThreadId = threadId;
		const ref = messagesWrapperRef;

		console.log('[ChatArea] thread registration effect:', {
			currentThreadId,
			previousThreadId,
			hasRef: !!ref
		});

		// Save position for previous thread
		if (previousThreadId && ref) {
			saveScrollPosition(previousThreadId, ref);
		}

		// Unregister previous thread from controller
		if (previousThreadId && previousThreadId !== currentThreadId) {
			unregisterThread(previousThreadId);
			cleanupThreadScroll(previousThreadId);
			// Reset scroll tracking for new thread
			lastScrollTop = 0;
		}

		// Register new thread with controller
		if (ref && currentThreadId && currentThreadId !== previousThreadId) {
			console.log('[ChatArea] registering thread:', currentThreadId);
			registerThreadElement(currentThreadId, ref);
			initializeThreadScroll(currentThreadId);
		}

		previousThreadId = currentThreadId;

		// Restore position or scroll to bottom
		if (ref && currentThreadId) {
			if (hasSavedPosition(currentThreadId)) {
				restoreScrollPosition(currentThreadId, ref);
			} else {
				// New thread - scroll to bottom immediately (no animation, no RAF)
				// This prevents the visual "flash" of seeing top before scrolling
				ref.scrollTop = ref.scrollHeight;
			}
			// Update lastScrollTop after setting position
			lastScrollTop = ref.scrollTop;
		}
	});

	// Auto-scroll when messages change - using new controller
	$effect(() => {
		const ref = messagesWrapperRef;
		const currentThreadId = threadId;

		// Trigger on messages length change
		void messages.length;

		// Also trigger on streaming content changes to scroll during streaming
		// This ensures we scroll when streaming content is being added to existing messages
		void $streamingContent.length;

		console.log('[ChatArea] content update effect:', {
			hasRef: !!ref,
			currentThreadId,
			messagesLength: messages.length,
			streamingContentLength: $streamingContent.length
		});

		if (!ref || !currentThreadId) return;

		// Notify controller of content update
		handleContentUpdate(currentThreadId);

		// Show button when outside the scroll button threshold zone
		const distanceFromBottom = ref.scrollHeight - ref.scrollTop - ref.clientHeight;
		showScrollButton = distanceFromBottom > SCROLL_BUTTON_THRESHOLD;
	});

	// Handle scroll events - using new controller
	function handleScroll() {
		const ref = messagesWrapperRef;
		const currentThreadId = threadId;
		if (!ref || !currentThreadId) return;

		const currentScrollTop = ref.scrollTop;
		const scrolledUp = currentScrollTop < lastScrollTop - 5; // 5px threshold to ignore minor changes
		lastScrollTop = currentScrollTop;

		// Notify parent
		onScroll?.(ref.scrollTop, ref.scrollHeight, ref.clientHeight);

		// Notify controller of user scroll
		controllerHandleUserScroll(currentThreadId);
	}

	// Manual scroll to bottom (when user clicks the button)
	function handleScrollToBottom() {
		const ref = messagesWrapperRef;
		const currentThreadId = threadId;
		if (!ref || !currentThreadId) return;

		controllerEnableAutoScroll(currentThreadId);
		controllerScrollToBottom(currentThreadId, { priority: 'user', animated: true });
		showScrollButton = false;
		isUserScrolling = false;
		lastScrollTop = ref.scrollTop;
	}

	// Track input activity for empty state transitions
	function handleInputChange(value: string) {
		inputValue = value;
		hasInputActivity = value.length > 0 || hasInputFocus;
	}

	function handleInputFocus() {
		hasInputFocus = true;
		hasInputActivity = true;
	}

	function handleInputBlur() {
		hasInputFocus = false;
		hasInputActivity = inputValue.length > 0;
	}

	function handleModelChange(modelId: string) {
		selectedModelId = modelId;
	}

	// Re-enable auto-scroll and scroll to bottom when user sends a message
	function handleSubmit() {
		const ref = messagesWrapperRef;
		const currentThreadId = threadId;

		if (currentThreadId) {
			controllerEnableAutoScroll(currentThreadId);
		}

		if (inputValue.trim() && onSendMessage) {
			onSendMessage(inputValue.trim(), { modelId: selectedModelId ?? undefined });
			inputValue = '';
			// Keep selected model for subsequent messages in this conversation
		}

		// Delay scroll slightly to ensure user's message appears in DOM first
		if (ref && currentThreadId) {
			setTimeout(() => {
				controllerScrollToBottom(currentThreadId, { priority: 'user', animated: true });
				showScrollButton = false;
				isUserScrolling = false;
			}, 150);
		}
	}

	/**
	 * Determines if a message should display a group header.
	 * A header is shown when:
	 * - It's the first message, OR
	 * - The role changes from the previous message, OR
	 * - There's a time gap > 5 minutes from the previous message
	 */
	function shouldShowGroupHeader(index: number): boolean {
		if (index === 0) return true;

		const currentMessage = messages[index];
		const previousMessage = messages[index - 1];

		// Show header if role changed
		if (currentMessage.role !== previousMessage.role) return true;

		// Show header if time gap > 5 minutes
		const timeDiff = currentMessage.timestamp.getTime() - previousMessage.timestamp.getTime();
		if (timeDiff >= 5 * 60 * 1000) return true;

		return false;
	}

	function handleRetry() {
		onRetry?.();
	}

	async function handleCopyMessage(content: string) {
		try {
			await navigator.clipboard.writeText(content);
		} catch (err) {
			console.error('Failed to copy message:', err);
		}
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

	// Ambiguity resolution handlers
	function handleResolveAmbiguity(ambiguityId: string, resolution: string, type: ResolutionType) {
		chatStore.resolveAmbiguity(ambiguityId, resolution, type);
	}

	function handleDismissAmbiguity(ambiguityId: string) {
		chatStore.dismissAmbiguity(ambiguityId);
	}

	// Track messages presence for CSS transitions
	let hasMessages = $derived(messages.length > 0);
</script>

<div class={[styles.chatArea, hasMessages && styles.hasMessages].filter(Boolean).join(' ')}>
	<!-- Messages area -->
	<div
		bind:this={messagesWrapperRef}
		class={[styles.messagesWrapper, !hasMessages && styles.isEmptyState].filter(Boolean).join(' ')}
		onscroll={handleScroll}
	>
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
					<button class={styles.retryButton} onclick={handleRetry} type="button"> Retry </button>
				{/if}
			</div>
		{:else if messages.length === 0}
			<div class={styles.emptyWrapper}>
				<EmptyThreadState {hasInputActivity} context={emptyContext} {conversationTitle} />
			</div>
		{:else}
			<!-- Messages - flat rendering to avoid Svelte 5 nested #each reconciliation issues -->
			{#each messages as message, index (message.id)}
				{#if shouldShowGroupHeader(index)}
					<div
						class={[styles.messageGroup, index === 0 && styles.messageGroupFirst]
							.filter(Boolean)
							.join(' ')}
					>
						<div class={styles.groupHeader}>
							<span class={styles.groupRole}>{message.role}</span>
							<span class={styles.groupTime}>{formatGroupTime(message.timestamp)}</span>
						</div>
					</div>
				{/if}
				<div
					class={[
						styles.messageWrapper,
						message.role === 'user' ? styles.userWrapper : styles.assistantWrapper
					]
						.filter(Boolean)
						.join(' ')}
				>
					<div
						class={[
							styles.message,
							styles[getRoleClass(message.role)],
							getMessageStatusClass(message.status)
						]
							.filter(Boolean)
							.join(' ')}
					>
						<!-- Thread indicator -->
						{#if message.threadId && getThreadTitle(message.threadId)}
							<ThreadIndicator
								thread={{
									id: message.threadId,
									title: getThreadTitle(message.threadId)!
								} as any}
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
									<svg
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
									>
										<path d="M1 4v6h6" />
										<path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
									</svg>
								</button>
							</div>
						{/if}
					</div>
					<!-- Message actions -->
					{#if message.status !== 'sending'}
						<div class={styles.actionsContainer}>
							<MessageActions
								showCopy={true}
								showEdit={message.role === 'user'}
								showDelete={true}
								showRetry={message.status === 'error'}
								showFeedback={message.role === 'assistant'}
								onCopy={() => handleCopyMessage(message.content)}
								onEdit={() => onEditMessage?.(message.id)}
								onDelete={() => onDeleteMessage?.(message.id)}
								onRetry={() => onRetryMessage?.(message.id)}
							/>
						</div>
					{/if}
				</div>
			{/each}

			<!-- Answering indicator -->
			{#if isTyping}
				<AnsweringIndicator />
			{/if}
		{/if}

		<!-- Scroll to bottom button -->
		{#if showScrollButton}
			<button
				class={[styles.scrollToBottom, $isLlmStreaming && styles.pulsing, styles.secondaryElements]
					.filter(Boolean)
					.join(' ')}
				onclick={handleScrollToBottom}
				aria-label="Scroll to latest messages"
				type="button"
			>
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
					<polyline points="6 9 12 15 18 9" />
				</svg>
			</button>
		{/if}
	</div>

	<!-- Ambiguity resolution banner -->
	{#if $nlpState.pendingAmbiguities.length > 0}
		<div class={styles.clarificationWrapper}>
			<ClarificationBanner
				ambiguity={$nlpState.pendingAmbiguities[0]}
				onResolve={handleResolveAmbiguity}
				onDismiss={handleDismissAmbiguity}
			/>
		</div>
	{/if}

	<!-- Input area -->
	{#if showInput}
		<div
			class={[styles.inputContainer, inputAnimatingIn && styles.animateIn]
				.filter(Boolean)
				.join(' ')}
		>
			<ChatInput
				value={inputValue}
				onchange={handleInputChange}
				onsubmit={handleSubmit}
				onfocus={handleInputFocus}
				onblur={handleInputBlur}
				placeholder="Send a message..."
				{loading}
				showIntent={true}
				modelId={selectedModelId}
				onModelChange={handleModelChange}
				{onOpenSettings}
			/>
		</div>
	{/if}
</div>
