<script lang="ts">
	/**
	 * ChatMessagesList Component
	 * Renders the list of chat messages with grouping and specialized rendering.
	 *
	 * Document Generation Integration:
	 * - Detects document generation tool calls and aggregates them
	 * - Renders DocumentGenerationCard instead of individual tool_call messages
	 * - Hides transient execution narration during doc generation
	 */
	import ThreadIndicator from './ThreadIndicator.svelte';
	import MarkdownRenderer from '../markdown/MarkdownRenderer.svelte';
	import MessageActions from './MessageActions.svelte';
	import EmptyThreadState from './EmptyThreadState.svelte';
	import AnsweringIndicator from './AnsweringIndicator.svelte';
	// Agent message components
	import ThinkingMessage from './messages/ThinkingMessage.svelte';
	import SystemNoticeMessage from './messages/SystemNoticeMessage.svelte';
	import ToolCallMessage from './messages/ToolCallMessage.svelte';
	import TerminalOutput from './messages/TerminalOutput.svelte';
	import NarrationMessage from './messages/NarrationMessage.svelte';
	import ApprovalMessage from './messages/ApprovalMessage.svelte';
	import RecoveryMessage from './messages/RecoveryMessage.svelte';
	import DocumentGenerationCard from './messages/DocumentGenerationCard.svelte';
	import GroupedToolCallMessage from './messages/GroupedToolCallMessage.svelte';
	import { MessageSquareIcon, UserIcon } from '$lib/icons';
	import type { Message } from '$lib/types/message';
	import type { Thread } from '$lib/types/thread';
	import type { AgentStatus } from '$lib/utils/agentStatus';
	import type { DocumentGenerationJob } from '$lib/utils/documentGenerationTracker';
	import type { DocumentGenerationState } from '$lib/stores/documentGenerationStore';
	import { shouldHideMessageInDocGen } from '$lib/utils/documentGenerationTracker';
	import { documentGenerationStore } from '$lib/stores/documentGenerationStore';
	import { groupToolMessages, isToolGroup } from '$lib/utils/groupToolMessages';
	import type { DisplayItem } from '$lib/utils/groupToolMessages';
	import messageStyles from './MessageBubbles.module.css';
	import inputStyles from './ChatInputContainer.module.css';
	import { createLogger } from '$lib/utils/logger';
	import { saveArtifactToPath } from '$lib/api/artifacts';
	import { save } from '@tauri-apps/plugin-dialog';
	import { tick } from 'svelte';

	const log = createLogger('ChatMessagesList');

	// Progressive rendering budget. Opening a long thread used to mount every
	// message on one frame, blocking the main thread for seconds. Instead we
	// render the most recent `INITIAL_VISIBLE_COUNT` synchronously (the user
	// starts at the bottom, so these are what they actually see), then prepend
	// older messages in `PROGRESSIVE_CHUNK_SIZE` batches across subsequent
	// animation frames. Scroll preservation is handled by the parent via the
	// `onBeforeProgressiveChunk` / `onAfterProgressiveChunk` callbacks.
	const INITIAL_VISIBLE_COUNT = 50;
	const PROGRESSIVE_CHUNK_SIZE = 50;

	interface Props {
		/** Array of messages to display */
		messages: Message[];

		/** Available threads for looking up thread info */
		threads?: Thread[];

		/** Loading state for messages */
		messagesLoading?: 'idle' | 'loading' | 'success' | 'error';

		/** Empty state context */
		emptyContext?: 'none' | 'conversation' | 'thread';

		/** Optional conversation title for empty state context */
		conversationTitle?: string;

		/** Is assistant typing */
		isTyping?: boolean;

		/** Current thread agent status */
		agentStatus?: AgentStatus | null;

		/** Error message to display */
		error?: string;

		/** Has input activity (for empty state) */
		hasInputActivity?: boolean;

		/** Edit message handler */
		onEditMessage?: (id: string) => void;

		/** Delete message handler */
		onDeleteMessage?: (id: string) => void;

		/** Retry message handler */
		onRetryMessage?: (id: string) => void;

		/** Message feedback handler */
		onMessageFeedback?: (messageId: string, feedback: 'like' | 'dislike' | null) => void;

		/** Thread click handler */
		onThreadClick?: (threadId: string) => void;

		/** Retry handler */
		onRetry?: () => void;

		/** Copy message handler */
		onCopyMessage?: (content: string) => Promise<void>;

		/** Approve action handler */
		onApproveAction?: (sessionId: string, actionId: string) => void;

		/** Deny action handler */
		onDenyAction?: (sessionId: string, actionId: string) => void;

		/** Continue execution handler */
		onContinueExecution?: (sessionId: string) => void;

		/** Replan execution handler */
		onReplanExecution?: (sessionId: string) => void;

		/** Create conversation handler — wired to the empty-state CTA when no conversation is selected */
		onCreateConversation?: () => void;

		/** Create thread handler — wired to the empty-state CTA when a conversation has no thread */
		onCreateThread?: () => void;

		/**
		 * Called immediately before `visibleCount` grows and older messages get
		 * prepended to the DOM. The parent should capture scroll state here so
		 * it can restore the anchor after Svelte reconciles.
		 */
		onBeforeProgressiveChunk?: () => void;

		/**
		 * Called after the next chunk has been reconciled into the DOM
		 * (i.e. after `await tick()` inside the growth loop). The parent should
		 * restore the captured scroll anchor here.
		 */
		onAfterProgressiveChunk?: () => void;
	}

	let {
		messages,
		threads = [],
		messagesLoading = 'idle',
		emptyContext = 'thread',
		conversationTitle,
		isTyping = false,
		agentStatus = null,
		error,
		hasInputActivity = false,
		onEditMessage,
		onDeleteMessage,
		onRetryMessage,
		onMessageFeedback,
		onThreadClick,
		onRetry,
		onCopyMessage,
		onApproveAction,
		onDenyAction,
		onContinueExecution,
		onReplanExecution,
		onCreateConversation,
		onCreateThread,
		onBeforeProgressiveChunk,
		onAfterProgressiveChunk
	}: Props = $props();

	// Document generation state - use centralized store
	// We need to subscribe to the store to get reactive updates
	let docGenState: DocumentGenerationState = $state({
		jobs: [],
		activeJob: null,
		hasActiveJob: false
	});

	// Subscribe to the store for reactive updates
	const unsubscribe = documentGenerationStore.subscribe((state) => {
		docGenState.jobs = state.jobs;
		docGenState.activeJob = state.activeJob;
		docGenState.hasActiveJob = state.hasActiveJob;
	});

	// Cleanup on unmount
	$effect(() => {
		return () => unsubscribe();
	});

	// Track which messages should be hidden due to doc gen aggregation,
	// then group consecutive tool_call messages by stepNumber.
	// Doc-gen tracking is fed from the message append/update path in
	// chat/message-methods.ts and from selectThread's one-shot replay, so
	// this component does not need a per-render scan.
	const displayItems = $derived.by<DisplayItem[]>(() => {
		const visible = messages.filter((msg) => !shouldHideMessageInDocGen(msg));
		return groupToolMessages(visible);
	});

	// Progressive rendering state. `visibleCount` is how many trailing items we
	// want to render; `effectiveCount` clamps it to `[0, displayItems.length]`
	// so the template and `firstVisibleIndex` are always consistent even when
	// the `$effect` below hasn't yet reacted to a `displayItems` change on the
	// same tick. `lastKnownTotal` is the last `displayItems.length` we reacted
	// to, used to distinguish thread switches from simple appends.
	let visibleCount = $state(INITIAL_VISIBLE_COUNT);
	let lastKnownTotal = $state(0);
	// Per-run cancel token so that a thread switch mid-growth-loop doesn't keep
	// advancing the previous thread's counter into the new thread's items.
	let currentRun: { cancelled: boolean } = $state({ cancelled: true });

	const effectiveCount = $derived(
		Math.min(displayItems.length, Math.max(0, visibleCount))
	);
	const firstVisibleIndex = $derived(displayItems.length - effectiveCount);
	const visibleItems = $derived.by<DisplayItem[]>(() => {
		if (effectiveCount === 0) return [];
		if (effectiveCount >= displayItems.length) return displayItems;
		return displayItems.slice(-effectiveCount);
	});

	$effect(() => {
		const total = displayItems.length;

		// No content: cancel any in-flight growth; let `visibleItems` naturally
		// return []. We intentionally do NOT reset `visibleCount` here — keeping
		// it at its last value avoids an inconsistent single-frame window on
		// the following thread load where `visibleCount` would be 0 while
		// `displayItems.length` was already the new total.
		if (total === 0) {
			currentRun.cancelled = true;
			lastKnownTotal = 0;
			return;
		}

		// Simple append while already showing everything: reveal the new tail.
		if (lastKnownTotal > 0 && total > lastKnownTotal && visibleCount >= lastKnownTotal) {
			visibleCount = total;
			lastKnownTotal = total;
			return;
		}

		// New or replaced content (thread switch / bulk load). Start over.
		if (total !== lastKnownTotal) {
			currentRun.cancelled = true;
			lastKnownTotal = total;

			const initial = Math.min(total, INITIAL_VISIBLE_COUNT);
			visibleCount = initial;

			if (initial < total) {
				const run = { cancelled: false };
				currentRun = run;
				void growVisibleCount(run);
			}
		}
	});

	async function growVisibleCount(run: { cancelled: boolean }) {
		// Let the initial chunk paint before we start prepending older items.
		await nextFrame();
		while (!run.cancelled && visibleCount < displayItems.length) {
			onBeforeProgressiveChunk?.();
			visibleCount = Math.min(visibleCount + PROGRESSIVE_CHUNK_SIZE, displayItems.length);
			await tick();
			if (run.cancelled) break;
			onAfterProgressiveChunk?.();
			if (visibleCount >= displayItems.length) break;
			await nextFrame();
		}
	}

	function nextFrame(): Promise<void> {
		return new Promise((resolve) => requestAnimationFrame(() => resolve()));
	}

	// Get the current doc gen job for display (most recent active)
	let currentDocGenJob = $derived(docGenState.activeJob);

	/**
	 * Format time for group headers.
	 */
	function formatGroupTime(date: Date): string {
		return new Date(date).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	/**
	 * Get CSS class for message role.
	 */
	function getRoleClass(role: string): string {
		return `role-${role}`;
	}

	/**
	 * Get CSS class for message status.
	 */
	function getMessageStatusClass(status: string | undefined): string {
		if (status === 'error') return messageStyles.messageError;
		if (status === 'sending') return messageStyles.messageSending;
		return '';
	}

	/**
	 * Get thread title by ID.
	 */
	function getThreadTitle(threadId: string | undefined): string | undefined {
		if (!threadId) return undefined;
		return threads.find((t) => t.id === threadId)?.title;
	}

	function isThreadResolved(threadId: string | undefined): boolean {
		if (!threadId) return false;
		return threads.find((t) => t.id === threadId)?.metadata.resolved ?? false;
	}

	/**
	 * Check if a message should use specialized rendering based on its kind.
	 * ToolGroup items are always treated as agent messages.
	 */
	function getItemKind(
		item: DisplayItem
	): 'thinking' | 'tool_call' | 'tool_group' | 'terminal' | 'narration' | 'approval' | null {
		if (isToolGroup(item)) return 'tool_group';
		const message = item as Message;
		if (
			message.kind === 'thinking' ||
			message.kind === 'tool_call' ||
			message.kind === 'terminal' ||
			message.kind === 'narration' ||
			message.kind === 'approval'
		) {
			return message.kind;
		}
		return null;
	}

	function isSystemNotice(item: DisplayItem): boolean {
		if (isToolGroup(item)) return false;
		const message = item as Message;
		return message.role === 'system' && message.metadata?.systemNotice?.type === 'context_update';
	}

	/** Returns the timestamp of a display item for header gap calculations. */
	function getItemTimestamp(item: DisplayItem): Date {
		if (isToolGroup(item)) return item.messages[0].timestamp;
		return (item as Message).timestamp;
	}

	/** Returns the role of a display item ('assistant' for tool groups). */
	function getItemRole(item: DisplayItem): string {
		if (isToolGroup(item)) return 'assistant';
		return (item as Message).role;
	}

	/**
	 * Check if a display item should show a group header.
	 * Agent messages (thinking, tool_call, tool_group, terminal) don't show group headers.
	 */
	function shouldShowGroupHeader(index: number): boolean {
		if (index === 0) return true;

		const current = displayItems[index];
		const previous = displayItems[index - 1];

		// Skip group header for agent-specific and structured system notices
		if (getItemKind(current) || isSystemNotice(current)) return false;

		// Show header if role changed
		if (getItemRole(current) !== getItemRole(previous)) return true;

		// Show header if time gap > 5 minutes
		const timeDiff = getItemTimestamp(current).getTime() - getItemTimestamp(previous).getTime();
		if (timeDiff >= 5 * 60 * 1000) return true;

		return false;
	}

	/**
	 * Check if an agent display item should show an ASSISTANT group header.
	 * Shows header when starting a new agent message sequence or after a time gap.
	 */
	function shouldShowAgentHeader(index: number): boolean {
		const current = displayItems[index];

		// Only for agent-specific items
		if (!getItemKind(current)) return false;

		// First item always shows header
		if (index === 0) return true;

		const previous = displayItems[index - 1];

		// Show if previous item wasn't an agent item
		const prevIsAgent = getItemKind(previous) !== null;
		if (!prevIsAgent) {
			// Only show if coming from a different role (not assistant) or first in sequence
			return getItemRole(previous) !== 'assistant';
		}

		// Show if time gap > 5 minutes between agent items
		const timeDiff = getItemTimestamp(current).getTime() - getItemTimestamp(previous).getTime();
		if (timeDiff >= 5 * 60 * 1000) return true;

		return false;
	}

	// Default copy handler if not provided
	async function handleCopyMessage(content: string) {
		if (onCopyMessage) {
			await onCopyMessage(content);
		} else {
			try {
				await navigator.clipboard.writeText(content);
			} catch (err) {
				log.error('Failed to copy message:', err);
			}
		}
	}

	// Document generation handlers
	async function handleDownloadDoc(job: DocumentGenerationJob) {
		if (!job.artifactId) {
			log.warn('DocGen Save: job has no artifactId', job.id);
			return;
		}

		const extension = job.docType.toLowerCase();
		const defaultPath = job.filename || `document.${extension}`;

		const destination = await save({
			defaultPath,
			filters: [{ name: job.docType, extensions: [extension] }]
		});
		if (!destination) return; // user cancelled

		try {
			await saveArtifactToPath(job.artifactId, destination);
		} catch (err) {
			log.error('Failed to save artifact:', err);
		}
	}

	async function handleCopyDocLink(job: DocumentGenerationJob) {
		if (job.filePath) {
			try {
				await navigator.clipboard.writeText(job.filePath);
			} catch (err) {
				log.error('Failed to copy path:', err);
			}
		}
	}

	function handleRetryDocGen(job: DocumentGenerationJob) {
		documentGenerationStore.retryJob(job.id);
	}

	function handleDismissDocGen(job: DocumentGenerationJob) {
		documentGenerationStore.removeJob(job.id);
	}
</script>

{#if messagesLoading === 'loading'}
	<div class={inputStyles.loading}>
		<div class={inputStyles.spinner}></div>
		<span>Loading messages...</span>
	</div>
{:else if messagesLoading === 'error' || error}
	<div class={inputStyles.error}>
		<svg
			class={inputStyles.errorIcon}
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
		<p class={inputStyles.errorMessage}>{error || 'Failed to load messages'}</p>
		{#if onRetry}
			<button class={inputStyles.retryButton} onclick={onRetry} type="button"> Retry </button>
		{/if}
	</div>
{:else if messages.length === 0}
	<div class={inputStyles.emptyWrapper}>
		<EmptyThreadState
			{hasInputActivity}
			context={emptyContext}
			{conversationTitle}
			{onCreateConversation}
			{onCreateThread}
		/>
	</div>
{:else}
	<!-- Messages - flat rendering to avoid Svelte 5 nested #each reconciliation issues.
	     displayItems is derived from messages: doc-gen-hidden items are filtered out
	     and consecutive same-step tool_call messages are collapsed into ToolGroup nodes.
	     For long threads we render `visibleItems` (the trailing slice) and grow
	     progressively — see `growVisibleCount` above. `index` below is the
	     absolute index into `displayItems`, so header-gap helpers still work. -->
	{#each visibleItems as item, i (isToolGroup(item) ? `group_${item.stepNumber}` : item.id)}
		{@const index = firstVisibleIndex + i}
		{#if isToolGroup(item)}
			<!-- ---- Grouped tool call (2+ consecutive tool_calls sharing a stepNumber) ---- -->
			{#if shouldShowAgentHeader(index)}
				<div class={messageStyles.messageGroup}>
					<div class={messageStyles.groupHeader}>
						<span class={messageStyles.groupRole}>ASSISTANT</span>
						<span class={messageStyles.groupTime}
							>{formatGroupTime(item.messages[0].timestamp)}</span
						>
					</div>
				</div>
			{/if}
			<div
				class={[
					messageStyles.messageWrapper,
					messageStyles.assistantWrapper,
					messageStyles.agentMessageWrapper
				]
					.filter(Boolean)
					.join(' ')}
			>
				<div
					class={[messageStyles.message, messageStyles.roleAssistant].filter(Boolean).join(' ')}
				>
					<GroupedToolCallMessage group={item} />
				</div>
			</div>
		{:else}
			{@const message = item}
			<!-- Insert DocumentGenerationCard after last user message when active -->
			{@const isLastUserMessage =
				message.role === 'user' &&
				(index === displayItems.length - 1 ||
					(isToolGroup(displayItems[index + 1])
						? true
						: (displayItems[index + 1] as Message).role !== 'user'))}
			{#if currentDocGenJob && isLastUserMessage}
				<div class={messageStyles.messageWrapper}>
					<div class={messageStyles.message}>
						<DocumentGenerationCard
							docType={currentDocGenJob.docType}
							filename={currentDocGenJob.filename}
							phase={currentDocGenJob.phase}
							statusText={currentDocGenJob.statusText}
							progress={currentDocGenJob.progress}
							steps={currentDocGenJob.steps}
							error={currentDocGenJob.error}
							sessionId={currentDocGenJob.sessionId}
							artifactId={currentDocGenJob.artifactId}
							filePath={currentDocGenJob.filePath}
							onDownload={() => handleDownloadDoc(currentDocGenJob)}
							onCopy={() => handleCopyDocLink(currentDocGenJob)}
							onRetry={() => handleRetryDocGen(currentDocGenJob)}
							onDismiss={() => handleDismissDocGen(currentDocGenJob)}
						/>
					</div>
				</div>
			{/if}

			<!-- Agent-specific messages (thinking, tool_call, terminal) - rendered with assistant styling -->
			{#if message.kind === 'thinking' || message.kind === 'tool_call' || message.kind === 'terminal' || message.kind === 'narration' || message.kind === 'approval' || message.kind === 'recovery'}
				<!-- Show ASSISTANT group header for agent messages -->
				{#if shouldShowAgentHeader(index)}
					<div class={messageStyles.messageGroup}>
						<div class={messageStyles.groupHeader}>
							<span class={messageStyles.groupRole}>ASSISTANT</span>
							<span class={messageStyles.groupTime}>{formatGroupTime(message.timestamp)}</span>
						</div>
					</div>
				{/if}
				<div
					class={[
						messageStyles.messageWrapper,
						messageStyles.assistantWrapper,
						messageStyles.agentMessageWrapper
					]
						.filter(Boolean)
						.join(' ')}
				>
					<div
						class={[messageStyles.message, messageStyles.roleAssistant].filter(Boolean).join(' ')}
					>
						{#if message.kind === 'thinking'}
							<ThinkingMessage {message} />
						{:else if message.kind === 'tool_call'}
							<ToolCallMessage {message} />
						{:else if message.kind === 'terminal'}
							<TerminalOutput {message} />
						{:else if message.kind === 'narration'}
							<NarrationMessage {message} />
						{:else if message.kind === 'approval'}
							<ApprovalMessage {message} onApprove={onApproveAction} onDeny={onDenyAction} />
						{:else if message.kind === 'recovery'}
							<RecoveryMessage
								{message}
								onContinue={onContinueExecution}
								onReplan={onReplanExecution}
							/>
						{/if}
					</div>
				</div>
			{:else if isSystemNotice(item)}
				<div
					class={[messageStyles.messageWrapper, messageStyles.systemNoticeWrapper]
						.filter(Boolean)
						.join(' ')}
				>
					<div
						class={[messageStyles.message, messageStyles.systemNoticeMessage]
							.filter(Boolean)
							.join(' ')}
					>
						<SystemNoticeMessage {message} />
					</div>
				</div>
			{:else}
				<!-- Normal messages (user, assistant, system) -->
				{#if shouldShowGroupHeader(index)}
					<div
						class={[messageStyles.messageGroup, index === 0 && messageStyles.messageGroupFirst]
							.filter(Boolean)
							.join(' ')}
					>
						<div class={messageStyles.groupHeader}>
							<div class={messageStyles.avatarWrapper}>
								{#if message.role === 'user'}
									<UserIcon size={12} strokeWidth={2.5} />
								{:else}
									<MessageSquareIcon size={12} strokeWidth={2.5} />
								{/if}
							</div>
							<span class={messageStyles.groupRole}>{message.role}</span>
							<span class={messageStyles.groupTime}>{formatGroupTime(message.timestamp)}</span>
						</div>
					</div>
				{/if}
				<div
					class={[
						messageStyles.messageWrapper,
						message.role === 'user' ? messageStyles.userWrapper : messageStyles.assistantWrapper
					]
						.filter(Boolean)
						.join(' ')}
				>
					<div
						class={[
							messageStyles.message,
							messageStyles[getRoleClass(message.role)],
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
								resolved={isThreadResolved(message.threadId)}
								onclick={() => onThreadClick?.(message.threadId!)}
								compact
							/>
						{/if}
						<div class={messageStyles.messageContent}>
							<MarkdownRenderer content={message.content} />
						</div>
						{#if message.role === 'assistant' && message.reasoning && message.reasoning.trim().length > 0}
							<div class={messageStyles.reasoningBlock}>
								<div class={messageStyles.reasoningLabel}>Reasoning</div>
								<div class={messageStyles.reasoningContent}>
									<MarkdownRenderer content={message.reasoning} />
								</div>
							</div>
						{/if}
						<!-- Message status indicator -->
						{#if message.status === 'sending' && message.content.trim() === '' && !isTyping}
							<AnsweringIndicator status={agentStatus} />
						{:else if message.status === 'error'}
							<div class={messageStyles.statusIndicator} aria-label="Failed to send">
								<button
									class={messageStyles.retryIconButton}
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
						<div class={messageStyles.actionsContainer}>
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
		{/if}
	{/each}

	<!-- Persistent bottom status strip for the thread-linked agent session -->
	{#if isTyping && agentStatus?.active}
		<div class={messageStyles.agentStatusWrapper}>
			<AnsweringIndicator status={agentStatus} />
		</div>
	{/if}
{/if}
