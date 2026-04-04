<script lang="ts">
	/**
	 * AppShell Component
	 * Desktop-grade application shell with custom header and resizable panels.
	 * Three-zone layout: left sidebar (conversations), center (threads), right sidebar (agent).
	 */
	import { onMount } from 'svelte';
	import AppHeader from './AppHeader.svelte';
	import ConversationSidebar from '$lib/components/chat/sidebar/ConversationSidebar.svelte';
	import ThreadPanel from '$lib/components/chat/thread/ThreadPanel.svelte';
	import ChatArea from '$lib/components/chat/ChatArea.svelte';
	import AgentPanel from '$lib/components/agent/AgentPanel.svelte';
	import AgentPanelEmpty from '$lib/components/agent/AgentPanelEmpty.svelte';
	import { tick } from 'svelte';
	import { chatStore } from '$lib/stores/chat';
	import type { Conversation, Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';
	import type { AgentSession, PendingAction } from '$lib/types/agent';
	import { createLogger } from '$lib/utils/logger';
	import { getThreadAgentState } from '$lib/utils/threadAgentState';
	import type { Snippet } from 'svelte';

	const log = createLogger('AppShell');

	interface Props {
		/* Conversation sidebar props */
		conversations: Conversation[];
		threads: Thread[];
		activeStreamingThreadIds?: string[];
		unreadThreadIds?: string[];
		selectedConversation?: Conversation;
		selectedThreadId?: string;
		onSelectConversation?: (conversation: Conversation) => void;
		onSelectThread?: (thread: Thread) => void;
		onCreateConversation?: () => void;
		onCreateThread?: () => void;
		onResolveThread?: (thread: Thread) => void;
		onDeleteConversation?: (conversation: Conversation) => void;
		onRenameConversation?: (conversation: Conversation, newTitle: string) => void;
		onArchiveConversation?: (conversation: Conversation) => void;
		onArchiveThread?: (thread: Thread) => void;
		onRenameThread?: (thread: Thread, newTitle: string) => void;
		onDeleteThread?: (thread: Thread) => void;
		onSearch?: (query: string) => void;
		onSettings?: () => void;
		onModelChange?: (conversationId: string, modelId: string) => void;
		onMessageFeedback?: (messageId: string, feedback: 'like' | 'dislike' | null) => void;
		loading?: boolean;
		threadsLoading?: boolean;
		hasConfiguredProvider?: boolean;
		searchQuery?: string;

		/* Agent panel props */
		agentSession?: AgentSession | null;
		agentPendingActions?: PendingAction[];
		agentPanelOpen?: boolean;
		agentSessions?: AgentSession[];
		allAgentSessions?: AgentSession[];
		agentSessionTitles?: Map<string, string>;
		onToggleAgentPanel?: () => void;
		onApproveAction?: (sessionId: string, actionId: string) => void;
		onDenyAction?: (sessionId: string, actionId: string) => void;
		onWorkspaceChange?: (path: string) => void;
		onSelectSession?: (sessionId: string) => void;
		onCreateSession?: () => void;
		onRetryLastRequest?: (sessionId: string) => void;
		onContinueExecution?: (sessionId: string) => void;
		onReplanExecution?: (sessionId: string) => void;
		conversationWorkspace?: string;
		onSetConversationWorkspace?: (path: string) => void;

		/* Chat area props */
		messages: Message[];
		onSendMessage?: (content: string, options?: { modelId?: string }) => void;

		/* Command palette */
		onCommandPalette?: () => void;
	}

	let {
		conversations,
		threads,
		activeStreamingThreadIds = [],
		unreadThreadIds = [],
		selectedConversation,
		selectedThreadId,
		onSelectConversation,
		onSelectThread,
		onCreateConversation,
		onCreateThread,
		onResolveThread,
		onDeleteConversation,
		onRenameConversation,
		onArchiveConversation,
		onArchiveThread,
		onRenameThread,
		onDeleteThread,
		onSearch,
		onSettings,
		onModelChange,
		onMessageFeedback,
		loading = false,
		threadsLoading = false,
		hasConfiguredProvider = true,
		searchQuery,
		agentSession = null,
		agentPendingActions = [],
		agentPanelOpen,
		agentSessions = [],
		allAgentSessions = [],
		agentSessionTitles = new Map(),
		onToggleAgentPanel,
		onApproveAction,
		onDenyAction,
		onWorkspaceChange,
		onSelectSession,
		onCreateSession,
		onRetryLastRequest,
		onContinueExecution,
		onReplanExecution,
		conversationWorkspace,
		onSetConversationWorkspace,
		messages,
		onSendMessage,
		onCommandPalette
	}: Props = $props();

	// Panel widths - load from localStorage on mount
	let leftSidebarWidth = $state(280);
	let rightSidebarWidth = $state(320);
	let leftSidebarOpen = $state(true);

	// Resize state
	let isResizingLeft = $state(false);
	let isResizingRight = $state(false);

	// Sync right sidebar open state with agent panel prop
	let rightSidebarOpen = $derived(agentPanelOpen ?? false);

	// Derived state
	let selectedThread = $derived(
		selectedThreadId ? threads.find((t) => t.id === selectedThreadId) : undefined
	);

	let initialModelId = $derived(selectedConversation?.settings.model ?? null);
	let currentThreadAgentState = $derived(getThreadAgentState(selectedThread, allAgentSessions));

	// Load persisted widths on mount
	onMount(() => {
		try {
			const savedLeft = localStorage.getItem('app-left-sidebar-width');
			const savedRight = localStorage.getItem('app-right-sidebar-width');
			if (savedLeft) leftSidebarWidth = Math.max(200, Math.min(480, parseInt(savedLeft, 10)));
			if (savedRight) rightSidebarWidth = Math.max(280, Math.min(500, parseInt(savedRight, 10)));
		} catch (e) {
			log.warn('Failed to load panel widths:', e);
		}
	});

	// Persist widths on change
	$effect(() => {
		try {
			localStorage.setItem('app-left-sidebar-width', String(leftSidebarWidth));
			localStorage.setItem('app-right-sidebar-width', String(rightSidebarWidth));
		} catch (e) {
			log.warn('Failed to save panel widths:', e);
		}
	});

	// Keyboard shortcuts
	onMount(() => {
		const handleKeydown = (e: KeyboardEvent) => {
			// Cmd/Ctrl + B to toggle left sidebar
			if ((e.metaKey || e.ctrlKey) && e.key === 'b') {
				e.preventDefault();
				leftSidebarOpen = !leftSidebarOpen;
			}
			// Cmd/Ctrl + J to toggle right sidebar
			if ((e.metaKey || e.ctrlKey) && e.key === 'j') {
				e.preventDefault();
				onToggleAgentPanel?.();
			}
		};

		window.addEventListener('keydown', handleKeydown);
		return () => window.removeEventListener('keydown', handleKeydown);
	});

	// Handlers
	function handleSelectConversation(conversation: Conversation) {
		onSelectConversation?.(conversation);
	}

	function handleSelectThread(thread: Thread) {
		onSelectThread?.(thread);
	}

	function handleSelectThreadById(threadId: string) {
		const thread = threads.find((t) => t.id === threadId);
		if (thread) {
			onSelectThread?.(thread);
		}
	}

	function handleSendMessage(content: string, options?: { modelId?: string }) {
		onSendMessage?.(content, options);
	}

	function handleReopenThread() {
		if (selectedThread && onResolveThread) {
			onResolveThread(selectedThread);
		}
	}

	function handleModelChange(modelId: string) {
		if (selectedConversation) {
			onModelChange?.(selectedConversation.id, modelId);
		}
	}

	let breadcrumbPath = $derived(
		selectedThread
			? [{ id: selectedConversation?.id || '', title: selectedConversation?.title || '' }]
			: []
	);

	async function handleReadyToWork() {
		const thread = await chatStore.createThread();
		if (thread) {
			await tick();
			const chatInput = document.querySelector('textarea') as HTMLTextAreaElement | null;
			if (chatInput) {
				chatInput.focus();
			}
		}
	}

	// Resize handlers - using the same approach as agent trace window
	function handleLeftResizerDown() {
		isResizingLeft = true;
		document.body.style.cursor = 'col-resize';
		document.body.style.userSelect = 'none';
		document.addEventListener('mousemove', handleLeftResizerMove);
		document.addEventListener('mouseup', handleLeftResizerUp);
	}

	function handleLeftResizerMove(event: MouseEvent) {
		if (!isResizingLeft) return;
		// Left sidebar: drag left to increase
		leftSidebarWidth = Math.max(200, Math.min(480, event.clientX));
	}

	function handleLeftResizerUp() {
		isResizingLeft = false;
		document.body.style.cursor = '';
		document.body.style.userSelect = '';
		document.removeEventListener('mousemove', handleLeftResizerMove);
		document.removeEventListener('mouseup', handleLeftResizerUp);
	}

	function handleRightResizerDown() {
		isResizingRight = true;
		document.body.style.cursor = 'col-resize';
		document.body.style.userSelect = 'none';
		document.addEventListener('mousemove', handleRightResizerMove);
		document.addEventListener('mouseup', handleRightResizerUp);
	}

	function handleRightResizerMove(event: MouseEvent) {
		if (!isResizingRight) return;
		// Right sidebar: drag right to increase (distance from right edge)
		rightSidebarWidth = Math.max(280, Math.min(500, window.innerWidth - event.clientX));
	}

	function handleRightResizerUp() {
		isResizingRight = false;
		document.body.style.cursor = '';
		document.body.style.userSelect = '';
		document.removeEventListener('mousemove', handleRightResizerMove);
		document.removeEventListener('mouseup', handleRightResizerUp);
	}

	// Get header subtitle from current context
	let headerSubtitle = $derived(
		selectedThread
			? selectedThread.title
			: selectedConversation
				? selectedConversation.title
				: undefined
	);
</script>

<div class="app-shell">
	<!-- Custom header -->
	<AppHeader subtitle={headerSubtitle} {onCommandPalette} showControls={true} />

	<!-- Main layout -->
	<div class="shell-main">
		<!-- Left sidebar - conversations -->
		{#if leftSidebarOpen}
			<aside class="sidebar-left" style:width="{leftSidebarWidth}px">
				<ConversationSidebar
					{conversations}
					selectedId={selectedConversation?.id}
					threadsForSelected={threads}
					{activeStreamingThreadIds}
					{unreadThreadIds}
					selectedThreadId={selectedThread?.id}
					onSelect={handleSelectConversation}
					onSelectThread={handleSelectThread}
					{onSearch}
					onCreateNew={onCreateConversation}
					{onCreateThread}
					onDelete={onDeleteConversation}
					onRename={onRenameConversation}
					onArchive={onArchiveConversation}
					{onResolveThread}
					{onRenameThread}
					{onDeleteThread}
					{onArchiveThread}
					{onSettings}
					{hasConfiguredProvider}
					{searchQuery}
				/>
			</aside>

			<!-- Left resizer -->
			<!-- svelte-ignore a11y_mouse_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<button
				class="resizer resizer-left"
				class:resizer-active={isResizingLeft}
				type="button"
				aria-label="Resize left sidebar"
				onmousedown={handleLeftResizerDown}
			></button>
		{/if}

		<!-- Center - thread area -->
		<main class="shell-center">
			{#if selectedThread}
				<ThreadPanel
					thread={selectedThread}
					{messages}
					{breadcrumbPath}
					onSendMessage={handleSendMessage}
					onOpenSettings={onSettings}
					onReopenThread={handleReopenThread}
					onToggleSidebar={() => (leftSidebarOpen = !leftSidebarOpen)}
					{loading}
					isTyping={currentThreadAgentState.isWorking}
					agentStatus={currentThreadAgentState.status}
					agentSessionId={currentThreadAgentState.sessionId}
					{initialModelId}
					onModelChange={handleModelChange}
					{onMessageFeedback}
					onApproveAction={(sessionId, actionId) => onApproveAction?.(sessionId, actionId)}
					onDenyAction={(sessionId, actionId) => onDenyAction?.(sessionId, actionId)}
					onContinueExecution={(sessionId) => onContinueExecution?.(sessionId)}
					onReplanExecution={(sessionId) => onReplanExecution?.(sessionId)}
				/>
			{:else if selectedConversation}
				<ChatArea
					{messages}
					threadId={selectedThreadId}
					{threads}
					emptyContext="conversation"
					conversationTitle={selectedConversation?.title}
					{loading}
					isTyping={currentThreadAgentState.isWorking}
					agentStatus={currentThreadAgentState.status}
					agentSessionId={currentThreadAgentState.sessionId}
					onSendMessage={handleSendMessage}
					onThreadClick={handleSelectThreadById}
					onOpenSettings={onSettings}
					{initialModelId}
					onModelChange={handleModelChange}
					{onMessageFeedback}
					onApproveAction={(sessionId, actionId) => onApproveAction?.(sessionId, actionId)}
					onDenyAction={(sessionId, actionId) => onDenyAction?.(sessionId, actionId)}
					onContinueExecution={(sessionId) => onContinueExecution?.(sessionId)}
					onReplanExecution={(sessionId) => onReplanExecution?.(sessionId)}
				/>
			{:else}
				<ChatArea
					{messages}
					threadId={selectedThreadId}
					emptyContext="none"
					{loading}
					isTyping={currentThreadAgentState.isWorking}
					agentStatus={currentThreadAgentState.status}
					agentSessionId={currentThreadAgentState.sessionId}
					onSendMessage={handleSendMessage}
					onOpenSettings={onSettings}
					onApproveAction={(sessionId, actionId) => onApproveAction?.(sessionId, actionId)}
					onDenyAction={(sessionId, actionId) => onDenyAction?.(sessionId, actionId)}
					onContinueExecution={(sessionId) => onContinueExecution?.(sessionId)}
					onReplanExecution={(sessionId) => onReplanExecution?.(sessionId)}
				/>
			{/if}
		</main>

		<!-- Right sidebar - agent panel -->
		{#if rightSidebarOpen}
			<!-- Right resizer -->
			<!-- svelte-ignore a11y_mouse_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<button
				class="resizer resizer-right"
				class:resizer-active={isResizingRight}
				type="button"
				aria-label="Resize right sidebar"
				onmousedown={handleRightResizerDown}
			></button>

			<aside class="sidebar-right" style:width="{rightSidebarWidth}px">
				{#if agentSession}
					<AgentPanel
						session={agentSession}
						allSessions={agentSessions}
						sessionTitles={agentSessionTitles}
						pendingActions={agentPendingActions}
						onApprove={(id) => onApproveAction?.(agentSession.id, id)}
						onDeny={(id) => onDenyAction?.(agentSession.id, id)}
						onWorkspaceChange={(path) => onWorkspaceChange?.(path)}
						onSelectSession={(id) => onSelectSession?.(id)}
						onCreateSession={() => onCreateSession?.()}
						onRetryLastRequest={(id) => onRetryLastRequest?.(id)}
						onclose={() => onToggleAgentPanel?.()}
					/>
				{:else}
					<AgentPanelEmpty
						workspacePath={conversationWorkspace}
						onSelectWorkspace={(path) => onSetConversationWorkspace?.(path)}
						onReadyToWork={handleReadyToWork}
					/>
				{/if}
			</aside>
		{/if}
	</div>
</div>

<style>
	.app-shell {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: 100%;
		overflow: hidden;
		background: var(--depth-bg-1);
	}

	.shell-main {
		flex: 1;
		display: flex;
		min-height: 0;
		position: relative;
	}

	/* Sidebars */
	.sidebar-left,
	.sidebar-right {
		display: flex;
		flex-direction: column;
		min-height: 100%;
		background: var(--depth-bg-2);
		flex-shrink: 0;
		overflow: hidden;
	}

	/* Center content area */
	.shell-center {
		flex: 1;
		min-width: 0;
		min-height: 100%;
		display: flex;
		flex-direction: column;
		position: relative;
		background: var(--depth-bg-1);
		border-left: 1px solid var(--color-border-subtle);
		border-right: 1px solid var(--color-border-subtle);
	}

	/* Resizers - Enhanced visual feedback */
	.resizer {
		position: relative;
		width: 4px;
		padding: 0;
		border: none;
		appearance: none;
		background: transparent;
		cursor: col-resize;
		z-index: 10;
		flex-shrink: 0;
		margin: 0 -2px;
		transition: all var(--transition-fast) var(--ease-out);
		-webkit-app-region: no-drag;
	}

	/* Visual indicator area */
	.resizer::before {
		content: '';
		position: absolute;
		top: 0;
		left: 50%;
		transform: translateX(-50%);
		width: 2px;
		height: 100%;
		background: var(--color-border-subtle);
		border-radius: var(--radius-full);
		transition: all var(--transition-fast) var(--ease-out);
	}

	/* Hover state - expand hit area and show indicator */
	.resizer:hover::before {
		width: 3px;
		background: var(--color-primary-400);
		box-shadow: 0 0 8px var(--color-primary-glow);
	}

	/* Active state during resize */
	.resizer.resizer-active::before {
		width: 3px;
		background: var(--color-primary-500);
		box-shadow: 0 0 12px var(--color-primary-glow-strong);
	}

	.resizer:hover,
	.resizer-active {
		background: transparent;
	}

	/* Reduced motion */
	@media (prefers-reduced-motion: reduce) {
		.resizer {
			animation: none;
			transition: none;
		}
	}

	/* Touch support */
	@media (pointer: coarse) {
		.resizer {
			width: 8px;
			margin: 0 -4px;
		}

		.resizer::before {
			width: 4px;
		}
	}
</style>
