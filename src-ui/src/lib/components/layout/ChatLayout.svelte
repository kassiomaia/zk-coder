<script lang="ts">
	/**
	 * ChatLayout Component
	 * Main responsive layout for the chat application.
	 * Single unified sidebar with hierarchical navigation (conversations + inline threads).
	 */
	import { onDestroy, onMount } from 'svelte';
	import styles from './ChatLayout.module.css';
	import ConversationSidebar from '../chat/sidebar/ConversationSidebar.svelte';
	import ThreadPanel from '../chat/thread/ThreadPanel.svelte';
	import ChatArea from '../chat/ChatArea.svelte';
	import AgentPanel from '../agent/AgentPanel.svelte';
	import WorkspaceSelector from '../agent/WorkspaceSelector.svelte';
	import { Bot, Layout } from 'lucide-svelte';
	import { tick } from 'svelte';
	import { chatStore } from '$lib/stores/chat';
	import type { Conversation, Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';
	import type { AgentSession, PendingAction } from '$lib/types/agent';
	import { createAppShortcuts, unregisterShortcuts } from '$lib/utils/keyboardShortcuts';
	import { getThreadAgentState } from '$lib/utils/threadAgentState';

	interface Props {
		/** Conversations for the sidebar */
		conversations: Conversation[];
		/** Threads for the current conversation */
		threads: Thread[];
		/** Thread IDs currently receiving streamed responses */
		activeStreamingThreadIds?: string[];
		/** Thread IDs with new unseen assistant responses */
		unreadThreadIds?: string[];
		/** Currently selected conversation */
		selectedConversation?: Conversation;
		/** Currently selected thread ID */
		selectedThreadId?: string;
		/** Messages for the current view */
		messages: Message[];
		/** Handler when conversation is selected */
		onSelectConversation?: (conversation: Conversation) => void;
		/** Handler when thread is selected */
		onSelectThread?: (thread: Thread) => void;
		/** Handler to send a message */
		onSendMessage?: (content: string, options?: { modelId?: string }) => void;
		/** Handler to create new conversation */
		onCreateConversation?: () => void;
		/** Handler to create new thread */
		onCreateThread?: () => void;
		/** Handler to resolve thread */
		onResolveThread?: (thread: Thread) => void;
		/** Handler to delete conversation */
		onDeleteConversation?: (conversation: Conversation) => void;
		/** Handler to rename conversation */
		onRenameConversation?: (conversation: Conversation, newTitle: string) => void;
		/** Handler to archive conversation */
		onArchiveConversation?: (conversation: Conversation) => void;
		/** Handler to archive thread */
		onArchiveThread?: (thread: Thread) => void;
		/** Handler to delete thread */
		onDeleteThread?: (thread: Thread) => void;
		/** Handler to rename thread */
		onRenameThread?: (thread: Thread, newTitle: string) => void;
		/** Handler for search */
		onSearch?: (query: string) => void;
		/** Handler for settings button click */
		onSettings?: () => void;
		/** Handler when model changes for selected conversation */
		onModelChange?: (conversationId: string, modelId: string) => void;
		/** Handler for message feedback (like/dislike) */
		onMessageFeedback?: (messageId: string, feedback: 'like' | 'dislike' | null) => void;
		/** Loading state */
		loading?: boolean;
		/** Threads loading state */
		threadsLoading?: boolean;
		/** Whether any provider is configured */
		hasConfiguredProvider?: boolean;
		/** Search query for filtering conversations and threads */
		searchQuery?: string;
		/** Active agent session (if any) */
		agentSession?: AgentSession | null;
		/** Pending actions awaiting approval */
		agentPendingActions?: PendingAction[];
		/** Whether the agent panel is open */
		agentPanelOpen?: boolean;
		/** Handler to toggle agent panel */
		onToggleAgentPanel?: () => void;
		/** Handler to approve an agent action */
		onApproveAction?: (sessionId: string, actionId: string) => void;
		/** Handler to deny an agent action */
		onDenyAction?: (sessionId: string, actionId: string) => void;
		/** Handler for workspace path change */
		onWorkspaceChange?: (path: string) => void;
		/** All agent sessions for this conversation */
		agentSessions?: AgentSession[];
		/** All loaded agent sessions */
		allAgentSessions?: AgentSession[];
		/** Map of sessionId to thread title for tab display */
		agentSessionTitles?: Map<string, string>;
		/** Handler to select a different agent session */
		onSelectSession?: (sessionId: string) => void;
		/** Handler to create a new agent session */
		onCreateSession?: () => void;
		/** Conversation's workspace root (for agent sessions) */
		conversationWorkspace?: string;
		/** Handler to set conversation workspace */
		onSetConversationWorkspace?: (path: string) => void;
		/** Handler to retry latest request for a recovered session */
		onRetryLastRequest?: (sessionId: string) => void;
	}

	let {
		conversations,
		threads,
		activeStreamingThreadIds = [],
		unreadThreadIds = [],
		selectedConversation,
		selectedThreadId,
		messages,
		onSelectConversation,
		onSelectThread,
		onSendMessage,
		onCreateConversation,
		onCreateThread,
		onResolveThread,
		onDeleteConversation,
		onRenameConversation,
		onArchiveConversation,
		onArchiveThread,
		onDeleteThread,
		onRenameThread,
		onSearch,
		onSettings,
		loading = false,
		threadsLoading = false,
		hasConfiguredProvider = true,
		onModelChange,
		onMessageFeedback,
		searchQuery,
		agentSession = null,
		agentPendingActions = [],
		agentPanelOpen = false,
		onToggleAgentPanel,
		onApproveAction,
		onDenyAction,
		onWorkspaceChange,
		agentSessions = [],
		allAgentSessions = [],
		agentSessionTitles = new Map(),
		onSelectSession,
		onCreateSession,
		conversationWorkspace,
		onSetConversationWorkspace,
		onRetryLastRequest
	}: Props = $props();

	// UI state for responsive behavior
	// Sidebar starts open on desktop, closed on mobile/tablet
	let sidebarOpen = $state(true);

	// Keyboard shortcuts
	let shortcutIds: string[] = [];

	onMount(() => {
		// Initialize sidebar state based on screen width
		const isDesktop = window.innerWidth >= 769;
		sidebarOpen = isDesktop;

		shortcutIds = createAppShortcuts({
			onNewThread: () => {
				if (selectedConversation) {
					onCreateThread?.();
				}
			},
			onNewConversation: () => {
				onCreateConversation?.();
			},
			onToggleSidebar: () => {
				toggleSidebar();
			},
			onEscape: () => {
				if (sidebarOpen) {
					closeSidebar();
				}
			}
		});
	});

	onDestroy(() => {
		unregisterShortcuts(shortcutIds);
	});

	// Derived state
	let hasSelectedConversation = $derived(!!selectedConversation);

	// Derive selected thread from threads array to always get fresh data
	let selectedThread = $derived(
		selectedThreadId ? threads.find((t) => t.id === selectedThreadId) : undefined
	);

	// Derive initial model from conversation settings
	let initialModelId = $derived(selectedConversation?.settings.model ?? null);

	function toggleSidebar() {
		sidebarOpen = !sidebarOpen;
	}

	function closeSidebar() {
		sidebarOpen = false;
	}

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

	// Get breadcrumb path from thread hierarchy
	let breadcrumbPath = $derived(
		selectedThread
			? [{ id: selectedConversation?.id || '', title: selectedConversation?.title || '' }]
			: []
	);

	let currentThreadAgentState = $derived(getThreadAgentState(selectedThread, allAgentSessions));
	async function handleReadyToWork() {
		const thread = await chatStore.createThread();
		if (thread) {
			// Small delay to ensure the DOM is updated before focusing
			await tick();
			const chatInput = document.querySelector('textarea') as HTMLTextAreaElement | null;
			if (chatInput) {
				chatInput.focus();
			}
		}
	}
</script>

<div class={styles.layout} data-sidebar-open={sidebarOpen}>
	<!-- Mobile header -->
	<header class={styles.header}>
		<button
			class={styles.menuButton}
			onclick={toggleSidebar}
			aria-label="Toggle sidebar"
			aria-expanded={sidebarOpen}
			type="button"
		>
			<svg
				width="24"
				height="24"
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

		<h1 class={styles.headerTitle}>
			{#if selectedThread}
				{selectedThread.title}
			{:else if selectedConversation}
				{selectedConversation.title}
			{:else}
				Coder
			{/if}
		</h1>
	</header>

	<!-- Main content area -->
	<div class={styles.main}>
		<!-- Sidebar backdrop (mobile) -->
		{#if sidebarOpen}
			<button
				class={styles.backdrop}
				onclick={closeSidebar}
				aria-label="Close sidebar"
				type="button"
			>
			</button>
		{/if}

		<!-- Unified sidebar with inline threads -->
		<div class={[styles.sidebarWrapper, sidebarOpen && styles.open].filter(Boolean).join(' ')}>
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
		</div>

		<!-- Main chat area -->
		<div class={styles.content}>
			{#if selectedThread}
				<!-- Thread panel with messages -->
				<ThreadPanel
					thread={selectedThread}
					{messages}
					{breadcrumbPath}
					onSendMessage={handleSendMessage}
					onOpenSettings={onSettings}
					onReopenThread={handleReopenThread}
					onToggleSidebar={toggleSidebar}
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
			{:else if hasSelectedConversation}
				<!-- Chat area for conversation (no thread selected) -->
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
				<!-- Empty state -->
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
		</div>

		<!-- Agent Panel (right side) -->
		{#if agentPanelOpen}
			<div class={styles.agentPanelWrapper}>
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
					<div class={styles.agentPanelEmpty}>
						<div class={styles.emptyHeader}>
							<div class={styles.emptyIconContainer}>
								<Bot size={20} strokeWidth={2} />
							</div>
							<div class={styles.emptyContent}>
								{#if conversationWorkspace}
									<p class={styles.emptyTitle}>Ready to Assist</p>
									<p class={styles.emptyHint}>Workspace active. Start a thread to begin.</p>
								{:else}
									<p class={styles.emptyTitle}>Workspace Required</p>
									<p class={styles.emptyHint}>Select a workspace to enable agents.</p>
								{/if}
							</div>
						</div>

						<div class={styles.emptyStateGroup}>
							<div class={styles.workspaceSection}>
								<div class={styles.workspaceHeader}>
									<Layout size={14} />
									<span>Workspace Status</span>
								</div>
								<div class={styles.workspaceSelectorContainer}>
									<WorkspaceSelector
										workspacePath={conversationWorkspace}
										onSelect={(path) => onSetConversationWorkspace?.(path)}
									/>
								</div>
							</div>

							<button
								class={styles.emptyButton}
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
			</div>
		{/if}
	</div>
</div>
