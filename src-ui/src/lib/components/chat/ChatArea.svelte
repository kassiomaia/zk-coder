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
	// Agent message components
	import ThinkingMessage from './messages/ThinkingMessage.svelte';
	import ToolCallMessage from './messages/ToolCallMessage.svelte';
	import TerminalOutput from './messages/TerminalOutput.svelte';
	import type { Message } from '$lib/types/message';
	import type { Thread } from '$lib/types/thread';
	import styles from './ChatArea.module.css';
	import { isLlmStreaming, streamingContent, nlpState } from '$lib/stores/chat';
	import { notifyAssistantComplete, initNotifications } from '$lib/utils/notifications';
	import { configStore } from '$lib/stores/configStore';
	import { activeSession, pendingActions } from '$lib/stores/agentStore';
	import { getAgentStatus } from '$lib/utils/agentStatus';
	import {
		saveScrollPosition,
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
		handleUserClickScrollDown as controllerHandleUserClickScrollDown,
		handleUserSendMessage as controllerHandleUserSendMessage,
		scrollToBottom as controllerScrollToBottom,
		isAutoScrollEnabled as controllerIsAutoScrollEnabled
	} from '$lib/utils/scrollController';
	import { onMount, tick } from 'svelte';
	import { chatStore } from '$lib/stores';
	import type { ResolutionType } from '$lib/types/nlp';
	import { createLogger } from '$lib/utils/logger';
	import { shouldForceInitialBottomScroll } from './chatAreaScroll';

	const log = createLogger('ChatArea');

	onMount(() => {
		initNotifications();
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

		/** Initial model ID from conversation settings */
		initialModelId?: string | null;

		/** Send message handler - can accept content string or object with content and modelId */
		onSendMessage?: (content: string, options?: { modelId?: string }) => void;

		/** Model change handler - called when user selects a different model */
		onModelChange?: (modelId: string) => void;

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

		/** Message feedback handler - called when user likes/dislikes a message */
		onMessageFeedback?: (messageId: string, feedback: 'like' | 'dislike' | null) => void;

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
		initialModelId = null,
		onSendMessage,
		onModelChange,
		onOpenSettings,
		onRetry,
		onEditMessage,
		onDeleteMessage,
		onRetryMessage,
		onMessageFeedback,
		onThreadClick,
		onScroll
	}: Props = $props();

	// Input state
	let inputValue = $state('');
	let hasInputActivity = $state(false);
	let hasInputFocus = $state(false);

	// Model selection state - initialize from conversation settings or config default
	let selectedModelId = $state<string | null>(null);

	// Initialize model from conversation settings or config default
	$effect(() => {
		if (initialModelId) {
			selectedModelId = initialModelId;
		} else {
			const defaultModel = configStore.getDefault('code');
			if (defaultModel) {
				selectedModelId = defaultModel;
			}
		}
	});

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

	// Track pending one-time bottom positioning when opening a thread
	let pendingInitialBottomScrollThreadId = $state<string | null>(null);

	// Threshold for showing scroll button (larger zone for visibility)
	const SCROLL_BUTTON_THRESHOLD = 100;

	// Watch for streaming state changes using the new controller
	$effect(() => {
		const streaming = $isLlmStreaming;
		const currentThreadId = threadId;
		const ref = messagesWrapperRef;

		log.debug('streaming state effect:', {
			streaming,
			wasStreaming,
			currentThreadId,
			hasRef: !!ref
		});

		if (!ref || !currentThreadId) return;

		// Detect transition from NOT streaming TO streaming
		if (!wasStreaming && streaming) {
			// Streaming just started - notify controller
			log.debug('streaming START detected');
			handleStreamingStart(currentThreadId);
		}

		// Detect transition from streaming to NOT streaming (completion)
		if (wasStreaming && !streaming) {
			// Streaming just completed - notify controller
			log.debug('streaming END detected');
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

		log.debug('thread registration effect:', {
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
			log.debug('registering thread:', currentThreadId);
			registerThreadElement(currentThreadId, ref);
			initializeThreadScroll(currentThreadId);
		}

		previousThreadId = currentThreadId;

		// For thread selection UX, always open at bottom.
		// We defer the final jump until messages are loaded/rendered.
		if (ref && currentThreadId) {
			// Prevent initial visual flash while loading
			ref.scrollTop = ref.scrollHeight;
			pendingInitialBottomScrollThreadId = currentThreadId;
			lastScrollTop = ref.scrollTop;
			showScrollButton = false;
		}
	});

	// After selected thread messages are loaded and rendered, force instant bottom positioning once.
	$effect(() => {
		const ref = messagesWrapperRef;
		const currentThreadId = threadId;
		const loadingState = messagesLoading;
		const pendingThreadId = pendingInitialBottomScrollThreadId;

		if (!ref || !currentThreadId) return;
		if (
			!shouldForceInitialBottomScroll({
				currentThreadId,
				pendingThreadId,
				messagesLoading: loadingState
			})
		)
			return;

		let cancelled = false;

		void (async () => {
			await tick();
			if (cancelled || pendingInitialBottomScrollThreadId !== currentThreadId) return;

			controllerScrollToBottom(currentThreadId, { priority: 'programmatic', animated: false });
			lastScrollTop = ref.scrollTop;
			showScrollButton = false;
			pendingInitialBottomScrollThreadId = null;
		})();

		return () => {
			cancelled = true;
		};
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

		log.debug('content update effect:', {
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

		controllerHandleUserClickScrollDown(currentThreadId);
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
		// Notify parent to persist the change
		onModelChange?.(modelId);
	}

	// Re-enable auto-scroll and scroll to bottom when user sends a message
	function handleSubmit() {
		const ref = messagesWrapperRef;
		const currentThreadId = threadId;
		const content = inputValue.trim();
		const canSubmit = Boolean(content && onSendMessage);

		if (currentThreadId && canSubmit) {
			controllerHandleUserSendMessage(currentThreadId);
		}

		if (canSubmit && onSendMessage) {
			onSendMessage(content, { modelId: selectedModelId ?? undefined });
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

	function handleRetry() {
		onRetry?.();
	}

	async function handleCopyMessage(content: string) {
		try {
			await navigator.clipboard.writeText(content);
		} catch (err) {
			log.error('Failed to copy message:', err);
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

	/**
	 * Check if a message should use specialized rendering based on its kind.
	 */
	function getMessageKind(message: Message): 'thinking' | 'tool_call' | 'terminal' | null {
		return message.kind === 'thinking' || message.kind === 'tool_call' || message.kind === 'terminal'
			? message.kind
			: null;
	}

	/**
	 * Check if a message should show a group header.
	 * Agent messages (thinking, tool_call, terminal) don't show group headers.
	 */
	function shouldShowGroupHeader(index: number): boolean {
		if (index === 0) return true;

		const currentMessage = messages[index];
		const previousMessage = messages[index - 1];

		// Skip group header for agent-specific messages
		if (getMessageKind(currentMessage)) return false;

		// Show header if role changed
		if (currentMessage.role !== previousMessage.role) return true;

		// Show header if time gap > 5 minutes
		const timeDiff = currentMessage.timestamp.getTime() - previousMessage.timestamp.getTime();
		if (timeDiff >= 5 * 60 * 1000) return true;

		return false;
	}

	/**
	 * Check if an agent message should show an ASSISTANT group header.
	 * Shows header when starting a new agent message sequence or after a time gap.
	 */
	function shouldShowAgentHeader(index: number): boolean {
		const currentMessage = messages[index];

		// Only for agent-specific messages
		if (!getMessageKind(currentMessage)) return false;

		// First message always shows header
		if (index === 0) return true;

		const previousMessage = messages[index - 1];

		// Show if previous message wasn't an agent message
		const prevIsAgentMessage = getMessageKind(previousMessage) !== null;
		if (!prevIsAgentMessage) {
			// Only show if coming from a different role (not assistant) or first in sequence
			return previousMessage.role !== 'assistant';
		}

		// Show if time gap > 5 minutes between agent messages
		const timeDiff = currentMessage.timestamp.getTime() - previousMessage.timestamp.getTime();
		if (timeDiff >= 5 * 60 * 1000) return true;

		return false;
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

	// Check if current messages contain agent activity (thinking, tool_call, terminal)
	// When present, we suppress the generic AnsweringIndicator to avoid duplicates
	let hasAgentMessages = $derived(
		messages.some((m) => m.kind === 'thinking' || m.kind === 'tool_call' || m.kind === 'terminal')
	);

	// Derive agent status from active session
	let agentStatus = $derived(
		(() => {
			const session = $activeSession;
			if (!session) return null;

			const hasPendingApproval = $pendingActions.length > 0;

			// Get latest execution step
			const latestStep = session.executionSteps?.[session.executionSteps.length - 1] ?? null;

			// Get current in-progress subtask
			const currentSubtask =
				session.tasks
					?.flatMap((t) => t.subtasks ?? [])
					.find((st) => st.status === 'in_progress') ?? null;

			// Get context usage from session state
			const contextUsage = session.state.type === 'running' ? session.state.contextUsage : 0;

			// Get last activity timestamp for stall detection
			const lastActivityAt = session.updatedAt ? new Date(session.updatedAt) : null;

			return getAgentStatus(
				session.state,
				latestStep,
				currentSubtask,
				hasPendingApproval,
				session.statusPhrases,
				contextUsage,
				lastActivityAt
			);
		})()
	);
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
				<!-- Agent-specific messages (thinking, tool_call, terminal) - rendered with assistant styling -->
				{#if message.kind === 'thinking' || message.kind === 'tool_call' || message.kind === 'terminal'}
					<!-- Show ASSISTANT group header for agent messages -->
					{#if shouldShowAgentHeader(index)}
						<div class={styles.messageGroup}>
							<div class={styles.groupHeader}>
								<span class={styles.groupRole}>ASSISTANT</span>
								<span class={styles.groupTime}>{formatGroupTime(message.timestamp)}</span>
							</div>
						</div>
					{/if}
					<div
						class={[styles.messageWrapper, styles.assistantWrapper, styles.agentMessageWrapper]
							.filter(Boolean)
							.join(' ')}
					>
						<div
							class={[styles.message, styles.roleAssistant]
								.filter(Boolean)
								.join(' ')}
						>
							{#if message.kind === 'thinking'}
								<ThinkingMessage {message} />
							{:else if message.kind === 'tool_call'}
								<ToolCallMessage {message} />
							{:else if message.kind === 'terminal'}
								<TerminalOutput {message} />
							{/if}
						</div>
					</div>
				{:else}
					<!-- Normal messages (user, assistant, system) -->
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
							{#if message.status === 'sending' && message.content.trim() === '' && !hasAgentMessages}
								<AnsweringIndicator status={agentStatus} />
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
									feedback={message.metadata?.feedback}
									onCopy={() => handleCopyMessage(message.content)}
									onEdit={() => onEditMessage?.(message.id)}
									onDelete={() => onDeleteMessage?.(message.id)}
									onRetry={() => onRetryMessage?.(message.id)}
									onFeedback={(feedback) => onMessageFeedback?.(message.id, feedback)}
								/>
							</div>
						{/if}
					</div>
				{/if}
			{/each}

			<!-- Answering indicator (only show when no specialized agent messages are present) -->
			{#if isTyping && !hasAgentMessages}
				<AnsweringIndicator status={agentStatus} />
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
