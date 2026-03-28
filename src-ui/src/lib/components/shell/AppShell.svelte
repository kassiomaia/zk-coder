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
	import WorkspaceSelector from '$lib/components/agent/WorkspaceSelector.svelte';
	import { Bot, Layout } from 'lucide-svelte';
	import { tick } from 'svelte';
	import { chatStore } from '$lib/stores/chat';
	import type { Conversation, Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';
	import type { AgentSession, PendingAction } from '$lib/types/agent';
	import { getThreadAgentState } from '$lib/utils/threadAgentState';
	import type { Snippet } from 'svelte';

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
		conversationWorkspace,
		onSetConversationWorkspace,
		messages,
		onSendMessage,
		onCommandPalette,
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
			console.warn('Failed to load panel widths:', e);
		}
	});

	// Persist widths on change
	$effect(() => {
		try {
			localStorage.setItem('app-left-sidebar-width', String(leftSidebarWidth));
			localStorage.setItem('app-right-sidebar-width', String(rightSidebarWidth));
		} catch (e) {
			console.warn('Failed to save panel widths:', e);
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
	<AppHeader
		subtitle={headerSubtitle}
		onCommandPalette={onCommandPalette}
		showControls={true}
	/>

	<!-- Main layout -->
	<div class="shell-main">
		<!-- Left sidebar - conversations -->
		{#if leftSidebarOpen}
			<aside class="sidebar-left" style:width="{leftSidebarWidth}px">
				<ConversationSidebar
					conversations={conversations}
					selectedId={selectedConversation?.id}
					threadsForSelected={threads}
					activeStreamingThreadIds={activeStreamingThreadIds}
					unreadThreadIds={unreadThreadIds}
					selectedThreadId={selectedThread?.id}
					onSelect={handleSelectConversation}
					onSelectThread={handleSelectThread}
					onSearch={onSearch}
					onCreateNew={onCreateConversation}
					onCreateThread={onCreateThread}
					onDelete={onDeleteConversation}
					onRename={onRenameConversation}
					onArchive={onArchiveConversation}
					onResolveThread={onResolveThread}
					onRenameThread={onRenameThread}
					onDeleteThread={onDeleteThread}
					onArchiveThread={onArchiveThread}
					onSettings={onSettings}
					hasConfiguredProvider={hasConfiguredProvider}
					searchQuery={searchQuery}
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
					<div class="agent-panel-empty">
						<div class="empty-header">
							<div class="empty-icon-container">
								<Bot size={20} strokeWidth={2} />
							</div>
							<div class="empty-content">
								{#if conversationWorkspace}
									<p class="empty-title">Ready to Assist</p>
									<p class="empty-hint">Workspace active. Start a thread to begin.</p>
								{:else}
									<p class="empty-title">Workspace Required</p>
									<p class="empty-hint">Select a workspace to enable agents.</p>
								{/if}
							</div>
						</div>

						<div class="empty-state-group">
							<div class="workspace-section">
								<div class="workspace-header">
									<Layout size={14} />
									<span>Workspace Status</span>
								</div>
								<div class="workspace-selector-container">
									<WorkspaceSelector
										workspacePath={conversationWorkspace}
										onSelect={(path) => onSetConversationWorkspace?.(path)}
									/>
								</div>
							</div>

							<button
								class="empty-button"
								onclick={handleReadyToWork}
								type="button"
								disabled={!conversationWorkspace}
								title={!conversationWorkspace ? 'Select a workspace first' : 'Create a new thread'}
							>
								Ready to Work
							</button>
						</div>
					</div>
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
		border-color: var(--color-border-subtle);
		border-style: solid;
		flex-shrink: 0;
		overflow: hidden;
	}

	.sidebar-left {
		border-right-width: 1px;
	}

	.sidebar-right {
		border-left-width: 1px;
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

	/* Resizers - simple buttons between panels */
	.resizer {
		position: relative;
		width: 2px;
		padding: 0;
		border: 0;
		appearance: none;
		background: transparent;
		cursor: col-resize;
		z-index: 10;
		flex-shrink: 0;
		margin-left: -1px;
		margin-right: -1px;
		transition: background-color 0.15s;
		-webkit-app-region: no-drag;
	}

	.resizer:hover,
	.resizer-active {
		background: var(--color-primary-500);
		transition-delay: 80ms;
	}

	/* Agent panel empty state */
	.agent-panel-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: flex-start;
		padding: var(--space-8) var(--space-4);
		height: 100%;
		text-align: left;
		animation: emptyStateFadeIn var(--transition-slow) forwards;
	}

	@keyframes emptyStateFadeIn {
		from {
			opacity: 0;
			transform: translateY(var(--space-1));
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.empty-header {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		width: 100%;
		margin-bottom: var(--space-6);
		padding: 0 var(--space-2);
	}

	.empty-icon-container {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		width: 40px;
		height: 40px;
		background: var(--color-interactive-hover);
		color: var(--color-text-secondary);
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border-subtle);
	}

	.empty-content {
		flex: 1;
		min-width: 0;
	}

	.empty-title {
		font-size: var(--text-base);
		font-weight: var(--font-semibold);
		color: var(--color-text-primary);
		margin: 0 0 var(--space-0-5);
		letter-spacing: var(--tracking-tight);
	}

	.empty-hint {
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
		margin: 0;
		line-height: var(--leading-tight);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.empty-state-group {
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-4);
		background: var(--color-interactive-hover);
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border-subtle);
	}

	.workspace-section {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		width: 100%;
		gap: var(--space-2);
	}

	.workspace-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wider);
	}

	.workspace-selector-container {
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: var(--depth-bg-3);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		transition: border-color var(--transition-fast);
	}

	.workspace-selector-container:hover {
		border-color: var(--color-border-primary);
	}

	.empty-button {
		width: 100%;
		padding: var(--space-2-5) var(--space-4);
		font-size: var(--text-sm);
		font-weight: var(--font-semibold);
		color: var(--color-neutral-0);
		background: var(--color-primary-600);
		border: 1px solid var(--color-primary-500);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition: all var(--transition-fast);
		box-shadow: var(--shadow-sm);
	}

	.empty-button:hover:not(:disabled) {
		background: var(--color-primary-500);
		border-color: var(--color-primary-400);
		transform: translateY(-1px);
		box-shadow: var(--shadow-md);
	}

	.empty-button:active:not(:disabled) {
		transform: translateY(0);
	}

	.empty-button:disabled {
		opacity: var(--opacity-disabled);
		cursor: not-allowed;
		background: var(--color-neutral-800);
		border-color: var(--color-neutral-700);
		color: var(--color-text-muted);
	}

	/* Reduced motion */
	@media (prefers-reduced-motion: reduce) {
		.agent-panel-empty,
		.empty-button {
			animation: none;
			transition: none;
		}

		.empty-button:active:not(:disabled) {
			transform: none;
		}
	}

	/* Touch support */
	@media (pointer: coarse) {
		.resizer {
			width: 4px;
		}
	}
</style>
