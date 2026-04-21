<script lang="ts">
	/**
	 * Main Page - Chat Interface
	 * Connected to the real backend via chat store.
	 */
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { invoke } from '@tauri-apps/api/core';
	import AppShell from '$lib/components/shell/AppShell.svelte';
	import ProvidersModal from '$lib/components/config/ProvidersModal.svelte';
	import ConfigModal from '$lib/components/config/ConfigModal.svelte';
	import CustomProviderModal from '$lib/components/config/CustomProviderModal.svelte';
	import SettingsModal from '$lib/components/config/SettingsModal.svelte';
	import ConfirmDialog from '$lib/components/shared/ConfirmDialog.svelte';
	import {
		chatStore,
		selectedConversation,
		activeStreamingThreadIds,
		chatError,
		isLoading,
		currentThreads,
		threadsLoading,
		unreadThreadIds,
		configStore,
		hasConfiguredProvider
	} from '$lib/stores';
	import type { Conversation, Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';
	import type { AgentSession, PendingAction } from '$lib/types/agent';
	import { resolvePersistedAgentContent } from '$lib/utils/agentStreaming';
	import {
		agentStore,
		activeSession,
		allSessions,
		conversationSessions,
		sessionThreadTitles,
		pendingActions as agentPendingActionsStore,
		agentPanelOpen as agentPanelOpenStore
	} from '$lib/stores';
	import { setThreadAgentSession } from '$lib/api/chat';
	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('MainPage');

	// App state
	let appInfo = $state<{ name: string; version: string } | null>(null);
	let backendStatus = $state('Connecting...');

	// Get messages and threads directly from chat store via subscription
	let messages = $state<Message[]>([]);
	let threads = $state<Thread[]>([]);
	let conversations = $state<Conversation[]>([]);
	let searchQuery = $state<string>('');
	let selectedThreadId = $state<string | null>(null);
	let processingThreadIds = $state<string[]>([]);
	let unreadThreads = $state<string[]>([]);
	let currentError = $state<string | null>(null);
	let currentThreadsLoading = $state<LoadingState>('idle');

	type LoadingState = 'idle' | 'loading' | 'success' | 'error';

	// Agent panel state (derived from agent store)
	let agentPanelOpen = $state(false);
	let agentSession = $state<AgentSession | null>(null);
	let agentPendingActions = $state<PendingAction[]>([]);
	let agentSessions = $state<AgentSession[]>([]);
	let allAgentSessions = $state<AgentSession[]>([]);
	let agentSessionTitles = $state<Map<string, string>>(new Map());

	// Subscribe to agent store
	$effect(() => {
		const unsubPanel = agentPanelOpenStore.subscribe((open) => {
			agentPanelOpen = open;
		});
		const unsubSession = activeSession.subscribe((session) => {
			agentSession = session;
		});
		const unsubActions = agentPendingActionsStore.subscribe((actions) => {
			agentPendingActions = actions;
		});
		const unsubSessions = conversationSessions.subscribe((sessions) => {
			agentSessions = sessions;
		});
		const unsubAllSessions = allSessions.subscribe((sessions) => {
			allAgentSessions = sessions;
		});
		const unsubTitles = sessionThreadTitles.subscribe((titles) => {
			agentSessionTitles = titles;
		});
		return () => {
			unsubPanel();
			unsubSession();
			unsubActions();
			unsubSessions();
			unsubAllSessions();
			unsubTitles();
		};
	});

	function handleToggleAgentPanel() {
		agentStore.togglePanel();
	}

	async function handleApproveAction(sessionId: string, actionId: string) {
		await agentStore.approveAction(sessionId, actionId);
	}

	async function handleDenyAction(sessionId: string, actionId: string) {
		await agentStore.denyAction(sessionId, actionId);
	}

	function handleWorkspaceChange(path: string) {
		if (agentSession) {
			agentStore.updateSessionWorkspace(agentSession.id, path);
		}
		// Also update the conversation workspace
		const conversation = $selectedConversation;
		if (conversation) {
			chatStore.setConversationWorkspace(conversation.id, path);
		}
	}

	async function handleRetryLastRequest(sessionId: string) {
		await agentStore.retryLastRequest(sessionId);
	}

	async function handleContinueExecution(sessionId: string) {
		await agentStore.continueExecution(sessionId);
	}

	async function handleReplanExecution(sessionId: string) {
		await agentStore.replanExecution(sessionId);
	}

	/**
	 * Reply to a paused clarification question (Workstream E.4). Re-uses
	 * `agentStore.sendRequest`, which wraps the same `send_agent_request`
	 * IPC the main chat input uses — the reply appears in the transcript
	 * and the backend clears the pending-clarification registry via the
	 * standard send pipeline.
	 */
	async function handleReplyToClarification(sessionId: string, content: string) {
		// agent-clarification-resume plan: the dedicated IPC persists
		// the reply to the transcript and wakes the parked coordinator
		// without the overhead of re-spawning a new execution.
		await agentStore.replyToClarification(sessionId, content);
	}

	async function handleSetConversationWorkspace(path: string) {
		log.debug('handleSetConversationWorkspace called:', { path });
		const conversation = $selectedConversation;
		if (conversation) {
			await chatStore.setConversationWorkspace(conversation.id, path);
		}
		// Also update the existing agent session's workspace if one exists
		// Check both the active session and any session linked to the current thread
		if (agentSession) {
			log.debug('Updating active agent session workspace:', agentSession.id);
			agentStore.updateSessionWorkspace(agentSession.id, path);
		} else {
			// Check if the current thread has an agentSessionId
			const currentThread = threads.find((t) => t.id === selectedThreadId);
			log.debug('Checking thread for agentSessionId:', {
				selectedThreadId,
				currentThread,
				agentSessionId: currentThread?.metadata.agentSessionId
			});
			if (currentThread?.metadata.agentSessionId) {
				log.debug(
					'Updating thread agent session workspace:',
					currentThread.metadata.agentSessionId
				);
				agentStore.updateSessionWorkspace(currentThread.metadata.agentSessionId, path);
			}
		}
	}

	function handleSelectSession(sessionId: string) {
		agentStore.setActiveSession(sessionId);
		// Sync thread to match the selected session
		const session = agentSessions.find((s) => s.id === sessionId);
		if (session?.threadId) {
			chatStore.selectThread(session.threadId);
		}
	}

	function createArtifactMetadata(artifacts: import('$lib/types/agent').ArtifactReference[]) {
		if (artifacts.length === 0) {
			return undefined;
		}

		return {
			artifacts: artifacts.map((artifact) => ({
				id: artifact.id,
				sessionId: artifact.sessionId,
				artifactType: artifact.artifactType,
				filename: artifact.filename,
				sizeBytes: artifact.sizeBytes,
				createdAt: artifact.createdAt
			}))
		};
	}

	async function handleCreateSession() {
		// Get the current conversation
		const currentConversation = $selectedConversation;
		log.debug('handleCreateSession called:', {
			conversationId: currentConversation?.id,
			workspaceRoot: currentConversation?.settings.workspaceRoot,
			settings: currentConversation?.settings
		});
		if (!currentConversation?.id) {
			log.warn('No conversation selected. Start a conversation first.');
			return;
		}

		// Get the current thread ID
		const currentThread = selectedThreadId ?? currentConversation.threads?.[0]?.id;
		if (!currentThread) {
			log.warn('No thread available in this conversation.');
			return;
		}

		// Check if thread already has an agent session
		const thread = threads.find((t) => t.id === currentThread);
		const existingSessionId = thread?.metadata.agentSessionId;
		log.debug('Checking for existing session:', {
			threadId: currentThread,
			existingSessionId
		});

		if (existingSessionId) {
			// Thread already has a session - just activate it and update workspace
			log.debug('Using existing session:', existingSessionId);
			agentStore.setActiveSession(existingSessionId);

			// Update the session's workspace if conversation has one set
			const workspace = currentConversation.settings.workspaceRoot;
			if (workspace) {
				log.debug('Updating existing session workspace:', workspace);
				await agentStore.updateSessionWorkspace(existingSessionId, workspace);
			}
			return;
		}

		// No existing session - create a new one
		const workspace = currentConversation.settings.workspaceRoot ?? agentSession?.workspaceRoot;
		log.debug('Creating new session with workspace:', { workspace });

		const newSession = await agentStore.createSession(
			'code-agent',
			currentConversation.id,
			currentThread,
			workspace
		);

		if (newSession) {
			// Update thread metadata with the new session ID
			await setThreadAgentSession(currentThread, newSession.id);
		}
	}

	// Filtered conversations based on search query
	let filteredConversations = $derived(() => {
		if (!searchQuery.trim()) {
			return conversations;
		}

		const query = searchQuery.toLowerCase().trim();

		return conversations.filter((conversation) => {
			// Search conversation title
			if (conversation.title.toLowerCase().includes(query)) {
				return true;
			}

			// Search threads within this conversation (only if available)
			if (conversation.threads && conversation.threads.length > 0) {
				return conversation.threads.some((thread) => thread.title.toLowerCase().includes(query));
			}

			return false;
		});
	});

	// Config modal state
	type ModalState = 'closed' | 'providers' | 'config' | 'custom-provider' | 'settings';
	let activeModal = $state<ModalState>('closed');
	let previousModal = $state<ModalState>('closed'); // Track previous modal to return after config
	let selectedProviderId = $state<string | null>(null);
	let configInitialized = $state(false);

	// Archive confirmation state
	let archiveConfirmOpen = $state(false);
	let archiveTarget = $state<{
		type: 'conversation' | 'thread';
		item: Conversation | Thread;
	} | null>(null);
	let archiveLoading = $state(false);
	let archiveTargetIsArchived = $derived(
		archiveTarget?.type === 'conversation'
			? ((archiveTarget.item as Conversation | undefined)?.archived ?? false)
			: ((archiveTarget?.item as Thread | undefined)?.metadata.archived ?? false)
	);

	// Delete confirmation state
	let deleteConfirmOpen = $state(false);
	let deleteTarget = $state<Conversation | null>(null);
	let deleteLoading = $state(false);

	// Subscribe to chat store changes
	$effect(() => {
		const unsubMsg = chatStore.subscribe((state) => {
			messages = state.messages;
			selectedThreadId = state.selectedThreadId;
		});
		const unsubConv = chatStore.subscribe((state) => {
			conversations = state.conversations;
		});
		const unsubThreads = currentThreads.subscribe((t) => {
			threads = t;
		});
		const unsubThreadsLoading = threadsLoading.subscribe((l) => {
			currentThreadsLoading = l;
		});
		const unsubProcessingThreads = activeStreamingThreadIds.subscribe((ids) => {
			processingThreadIds = ids;
		});
		const unsubUnreadThreads = unreadThreadIds.subscribe((ids) => {
			unreadThreads = ids;
		});
		const unsubErr = chatError.subscribe((err) => {
			currentError = err;
		});
		const unsubConfigErr = configStore.subscribe((state) => {
			if (state.error) {
				currentError = state.error;
			}
		});
		return () => {
			unsubMsg();
			unsubConv();
			unsubThreads();
			unsubThreadsLoading();
			unsubProcessingThreads();
			unsubUnreadThreads();
			unsubErr();
			unsubConfigErr();
		};
	});

	// Agent panel visibility logic based on conversation and thread selection
	$effect(() => {
		const conversation = $selectedConversation;
		const threadId = selectedThreadId;

		// Rule 1: No conversation or thread selected - close agent panel
		if (!conversation) {
			if (agentPanelOpen) {
				agentStore.togglePanel();
			}
			return;
		}

		// Rule 2: Thread is selected - show agent panel automatically
		if (threadId) {
			if (!agentPanelOpen) {
				agentStore.togglePanel();
			}
			return;
		}

		// Rule 3: Conversation selected but no thread (empty chat) - open agent panel for workspace setup
		// This allows user to set workspace before starting a thread
		if (!agentPanelOpen) {
			agentStore.togglePanel();
		}
	});

	// Initialize
	onMount(async () => {
		try {
			const health = await invoke<string>('health_check');
			backendStatus = health;
			const info = await invoke<{ name: string; version: string }>('get_app_info');
			appInfo = info;
		} catch (error) {
			backendStatus = 'Backend unavailable';
			log.warn('Backend not available:', error);
		}

		// Initialize chat store (loads conversations from backend)
		await chatStore.initialize();

		// Initialize agent store (listens for backend events)
		await agentStore.initialize();

		const pendingPersistedAssistantMessages = new Set<string>();

		async function persistAssistantMessage(
			streamingMessageId: string,
			conversationId: string,
			threadId: string,
			content: string,
			metadata?: Message['metadata']
		) {
			if (pendingPersistedAssistantMessages.has(streamingMessageId)) {
				return;
			}

			pendingPersistedAssistantMessages.add(streamingMessageId);

			try {
				const chatState = get(chatStore);
				const streamingMessage = chatState.messages.find((m) => m.id === streamingMessageId);
				const persistedContent = resolvePersistedAgentContent(streamingMessage?.content, content);
				const { sendMessage } = await import('$lib/api/chat');
				const savedMessage = await sendMessage({
					conversationId,
					threadId,
					role: 'assistant',
					content: persistedContent
				});
				const latestState = get(chatStore);
				const hasStreamingMessage = latestState.messages.some((m) => m.id === streamingMessageId);

				if (hasStreamingMessage) {
					chatStore.updateMessage(streamingMessageId, {
						id: savedMessage.id,
						content: savedMessage.content,
						status: 'sent',
						metadata: {
							...streamingMessage?.metadata,
							...savedMessage.metadata,
							...(metadata ?? {})
						}
					});
				} else {
					chatStore.addMessage({
						...savedMessage,
						metadata: {
							...savedMessage.metadata,
							...(metadata ?? {})
						}
					});
				}
			} catch (error) {
				log.error('Failed to persist assistant message:', error);
				chatStore.updateMessage(streamingMessageId, {
					status: 'error',
					metadata
				});
			} finally {
				pendingPersistedAssistantMessages.delete(streamingMessageId);
			}
		}

		// Set callback to add assistant message when agent completes execution
		// Import the API function to persist messages to backend
		agentStore.setExecutionCompleteCallback(
			async (sessionId, conversationId, threadId, content, artifacts) => {
				log.debug('Execution complete callback invoked:', {
					sessionId,
					conversationId,
					threadId,
					contentLength: content?.length ?? 0,
					content,
					artifactCount: artifacts.length
				});

				try {
					const streamingMessageId = `streaming-assistant-${sessionId}`;
					const artifactMetadata = createArtifactMetadata(artifacts);
					await persistAssistantMessage(
						streamingMessageId,
						conversationId,
						threadId,
						content,
						artifactMetadata
					);
					log.debug('Agent message persisted and added to chatStore:', streamingMessageId);
				} catch (error) {
					log.error('Failed to persist agent message:', error);
					// Still add locally even if backend save fails
					chatStore.addMessage({
						id: crypto.randomUUID(),
						conversationId,
						threadId,
						role: 'assistant',
						content,
						timestamp: new Date(),
						status: 'error',
						metadata: createArtifactMetadata(artifacts)
					});
				}
			}
		);
		log.debug('Execution complete callback registered');

		agentStore.setPersistAssistantMessageCallback(
			async (sessionId, conversationId, threadId, messageId, content) => {
				log.debug('Persist assistant streaming message callback invoked:', {
					sessionId,
					conversationId,
					threadId,
					messageId,
					contentLength: content.length
				});

				await persistAssistantMessage(messageId, conversationId, threadId, content);
			}
		);

		// Set callback to add system messages from agent (e.g., context compaction)
		agentStore.setSystemMessageCallback(async (sessionId, conversationId, threadId, message) => {
			log.debug('System message callback invoked:', {
				sessionId,
				conversationId,
				threadId,
				message
			});

			try {
				const { sendMessage } = await import('$lib/api/chat');
				const savedMessage = await sendMessage({
					conversationId,
					threadId,
					role: message.role,
					content: message.content
				});

				// Add to local state
				chatStore.addMessage({
					...savedMessage,
					metadata: {
						...savedMessage.metadata,
						...message.metadata
					}
				});
				log.debug('System message persisted and added to chatStore:', savedMessage.id);
			} catch (error) {
				log.error('Failed to persist system message:', error);
				// Still add locally even if backend save fails
				chatStore.addMessage({
					id: crypto.randomUUID(),
					conversationId,
					threadId,
					role: message.role,
					content: message.content,
					timestamp: new Date(),
					status: 'error',
					metadata: message.metadata
				});
			}
		});
		log.debug('System message callback registered');

		// Set callback to add agent activity messages (thinking, tool_call, terminal, streaming) to chat
		// These messages show agent progress and the final response

		// Track known message IDs for O(1) lookup instead of O(n) array find
		const knownMessageIds = new Set<string>();
		let messagesLength = 0; // Track length to detect when to rebuild cache

		agentStore.setAgentActivityCallback((conversationId, threadId, messageData) => {
			const messageId = messageData.id ?? crypto.randomUUID();

			// Check if message with this ID already exists (for thinking, tool_call, and streaming assistant messages)
			// These should update in-place rather than creating duplicates
			if (messageData.id) {
				// Fast path: check Set cache first
				let exists = knownMessageIds.has(messageData.id);

				// Slow path: rebuild cache if messages changed (thread switch, etc.)
				if (!exists || messagesLength !== get(chatStore).messages.length) {
					const chatState = get(chatStore);
					messagesLength = chatState.messages.length;
					// Rebuild cache
					knownMessageIds.clear();
					for (const m of chatState.messages) {
						knownMessageIds.add(m.id);
					}
					exists = knownMessageIds.has(messageData.id);
				}

				if (exists) {
					const chatState = get(chatStore);
					const existingMessage = chatState.messages.find((m: Message) => m.id === messageData.id);
					if (existingMessage) {
						// Update existing message
						// For agentData, merge with existing to preserve fields like toolArgs
						const updates: Partial<Message> = {};
						if (messageData.content !== undefined) updates.content = messageData.content;
						if (messageData.status !== undefined) updates.status = messageData.status as any;
						if (messageData.reasoning !== undefined)
							updates.reasoning = messageData.reasoning as string;
						if (messageData.agentData) {
							// Deep merge agentData to preserve existing fields
							updates.agentData = {
								...existingMessage.agentData,
								...messageData.agentData
							};
						}
						chatStore.updateMessage(messageData.id, updates);
						return;
					}
				}

				// Add to cache when creating new message
				knownMessageIds.add(messageData.id);
			}

			// Create new message if it doesn't exist yet
			chatStore.addMessage({
				...messageData,
				id: messageId,
				conversationId,
				threadId,
				timestamp: messageData.timestamp ?? new Date()
			} as Message);
		});

		// Remove transient tool/thinking bubbles after the persistent assistant message lands.
		agentStore.setCleanupTransientCallback((_conversationId, _threadId, sessionId) => {
			log.debug('Cleaning up transient messages for session:', sessionId);
			chatStore.removeTransientMessages(sessionId);
		});

		// Initialize config store and show providers modal if no providers configured
		await configStore.initialize();
		configInitialized = true;

		// Show providers modal if no providers are configured
		if (!$hasConfiguredProvider) {
			activeModal = 'providers';
		}
	});

	// Handlers - connected to chat store
	async function handleSelectConversation(conversation: Conversation) {
		await chatStore.selectConversation(conversation.id);
	}

	async function handleSelectThread(thread: Thread) {
		await chatStore.selectThread(thread.id);
	}

	async function handleSendMessage(content: string, options?: { modelId?: string }) {
		// Use LLM streaming - backend will use 'code' operation default model if modelId not provided
		await chatStore.sendMessageWithLlm(content, options);
	}

	async function handleCreateConversation() {
		const conversation = await chatStore.createConversation('New Conversation');
		if (conversation) {
			await chatStore.selectConversation(conversation.id);
		}
	}

	async function handleCreateThread() {
		const thread = await chatStore.createThread('New Thread');
		if (thread) {
			await chatStore.selectThread(thread.id);
			// Inherit workspace from conversation if available
			const conversation = $selectedConversation;
			if (conversation?.settings.workspaceRoot) {
				log.debug('Inheriting workspace for new thread:', conversation.settings.workspaceRoot);
				await chatStore.setThreadWorkspace(thread.id, conversation.settings.workspaceRoot);
			}
		}
	}

	async function handleResolveThread(thread: Thread) {
		await chatStore.resolveThread(thread.id, !thread.metadata.resolved);
	}

	function handleDeleteConversation(conversation: Conversation) {
		deleteTarget = conversation;
		deleteConfirmOpen = true;
	}

	function handleDeleteCancel() {
		deleteConfirmOpen = false;
		deleteTarget = null;
	}

	async function handleDeleteConfirm() {
		if (!deleteTarget) return;

		deleteLoading = true;
		try {
			await chatStore.deleteConversation(deleteTarget.id);
			deleteConfirmOpen = false;
			deleteTarget = null;
		} finally {
			deleteLoading = false;
		}
	}

	async function handleRenameConversation(conversation: Conversation, newTitle: string) {
		await chatStore.renameConversation(conversation.id, newTitle);
	}

	async function handleRenameThread(thread: Thread, newTitle: string) {
		await chatStore.renameThread(thread.id, newTitle);
	}

	// Model change handler - persists model selection to conversation settings
	// and syncs to any active agent session on the current thread
	async function handleModelChange(conversationId: string, modelId: string) {
		const conversation = $selectedConversation;
		if (conversation && conversation.settings.model !== modelId) {
			await chatStore.updateConversationSettings(conversationId, {
				model: modelId,
				temperature: conversation.settings.temperature,
				maxTokens: conversation.settings.maxTokens,
				systemPrompt: conversation.settings.systemPrompt
			});
		}
		// Sync agent session model if the current thread has an active agent session
		const currentThread = threads.find((t) => t.id === selectedThreadId);
		if (currentThread?.metadata.agentSessionId) {
			await agentStore.updateSessionModel(currentThread.metadata.agentSessionId, modelId);
		}
	}

	// Message feedback handler - handles like/dislike on messages
	async function handleMessageFeedback(
		messageId: string,
		feedback: 'like' | 'dislike' | null
	): Promise<void> {
		await chatStore.updateMessageFeedback(messageId, feedback);
	}

	// Archive handlers - shows confirmation dialog before archiving
	function handleArchiveConversation(conversation: Conversation) {
		archiveTarget = { type: 'conversation', item: conversation };
		archiveConfirmOpen = true;
	}

	function handleArchiveThread(thread: Thread) {
		archiveTarget = { type: 'thread', item: thread };
		archiveConfirmOpen = true;
	}

	function handleArchiveCancel() {
		archiveConfirmOpen = false;
		archiveTarget = null;
	}

	async function handleArchiveConfirm() {
		if (!archiveTarget) return;

		archiveLoading = true;
		try {
			if (archiveTarget.type === 'conversation') {
				const conversation = archiveTarget.item as Conversation;
				await chatStore.archiveConversation(conversation.id, !conversation.archived);
			} else {
				const thread = archiveTarget.item as Thread;
				await chatStore.archiveThread(thread.id, !(thread.metadata.archived ?? false));
			}
			archiveConfirmOpen = false;
			archiveTarget = null;
		} finally {
			archiveLoading = false;
		}
	}

	function handleSearch(query: string) {
		searchQuery = query;
	}

	function handleErrorRetry() {
		chatStore.clearError();
		configStore.clearError();
		currentError = null;
	}

	// Config modal handlers
	function handleOpenSettings() {
		activeModal = 'settings';
	}

	function handleCloseModal() {
		activeModal = 'closed';
		previousModal = 'closed';
		selectedProviderId = null;
	}

	function handleConfigureProvider(providerId: string) {
		selectedProviderId = providerId;
		previousModal = activeModal;
		activeModal = 'config';
	}

	function handleConfigSaved() {
		activeModal = previousModal;
		previousModal = 'closed';
		selectedProviderId = null;
	}

	function handleAddCustomProvider() {
		previousModal = activeModal;
		activeModal = 'custom-provider';
	}

	async function handleRemoveProvider(providerId: string) {
		try {
			await configStore.removeProvider(providerId);
		} catch (error) {
			log.error('Failed to remove provider:', error);
		}
	}
</script>

<svelte:head>
	<title>{appInfo?.name || 'Coder'} - AI Assistant</title>
</svelte:head>

<div class="page-container">
	<AppShell
		{conversations}
		{threads}
		activeStreamingThreadIds={processingThreadIds}
		unreadThreadIds={unreadThreads}
		selectedConversation={$selectedConversation ?? undefined}
		selectedThreadId={selectedThreadId ?? undefined}
		{messages}
		onSelectConversation={handleSelectConversation}
		onSelectThread={handleSelectThread}
		onSendMessage={handleSendMessage}
		onCreateConversation={handleCreateConversation}
		onCreateThread={handleCreateThread}
		onResolveThread={handleResolveThread}
		onDeleteConversation={handleDeleteConversation}
		onRenameConversation={handleRenameConversation}
		onArchiveConversation={handleArchiveConversation}
		onArchiveThread={handleArchiveThread}
		onRenameThread={handleRenameThread}
		onSearch={handleSearch}
		onSettings={handleOpenSettings}
		onModelChange={handleModelChange}
		onMessageFeedback={handleMessageFeedback}
		loading={$isLoading}
		threadsLoading={currentThreadsLoading === 'loading'}
		hasConfiguredProvider={$hasConfiguredProvider}
		{searchQuery}
		{agentSession}
		{agentPendingActions}
		{agentPanelOpen}
		onToggleAgentPanel={handleToggleAgentPanel}
		onApproveAction={handleApproveAction}
		onDenyAction={handleDenyAction}
		onWorkspaceChange={handleWorkspaceChange}
		{agentSessions}
		{allAgentSessions}
		{agentSessionTitles}
		onSelectSession={handleSelectSession}
		onCreateSession={handleCreateSession}
		onRetryLastRequest={handleRetryLastRequest}
		onContinueExecution={handleContinueExecution}
		onReplanExecution={handleReplanExecution}
		onReplyToClarification={handleReplyToClarification}
		conversationWorkspace={$selectedConversation?.settings.workspaceRoot}
		onSetConversationWorkspace={handleSetConversationWorkspace}
	/>

	<!-- Status indicator -->
	<div class="status-bar">
		<span class="status-actions">
			{#if currentError}
				<span class="error">Error: {currentError}</span>
				<button class="retry-btn" onclick={handleErrorRetry} type="button">Dismiss</button>
			{/if}
		</span>
	</div>
</div>

<!-- Configuration Modals -->
{#if configInitialized}
	<!-- Parent modals first so child modals (declared after) win on both z-index and DOM stacking order -->

	<!-- Providers Modal (shown when no providers configured) -->
	<ProvidersModal
		open={activeModal === 'providers' || previousModal === 'providers'}
		onclose={handleCloseModal}
		oncontinue={handleCloseModal}
		onconfigure={handleConfigureProvider}
		onaddcustom={handleAddCustomProvider}
	/>

	<!-- Settings Modal (full settings) -->
	<SettingsModal
		open={activeModal === 'settings' || previousModal === 'settings'}
		onclose={handleCloseModal}
		onconfigure={handleConfigureProvider}
		onaddcustom={handleAddCustomProvider}
		onremoveprovider={handleRemoveProvider}
	/>

	<!-- Child modals declared after parents: stacked={true} gives z-index+10, and later DOM position is a further guarantee -->

	<!-- Config Modal (for entering API key) -->
	<ConfigModal
		open={activeModal === 'config'}
		onclose={() => { activeModal = previousModal; previousModal = 'closed'; selectedProviderId = null; }}
		providerId={selectedProviderId}
		onsaved={() => {
			const returnModal = previousModal;
			handleConfigSaved();
			previousModal = returnModal;
		}}
	/>

	<!-- Custom Provider Modal -->
	<CustomProviderModal
		open={activeModal === 'custom-provider'}
		onclose={() => { activeModal = previousModal; previousModal = 'closed'; }}
		onsaved={(providerId: string) => {
			selectedProviderId = providerId;
			activeModal = 'config';
		}}
	/>
{/if}

<!-- Archive Confirmation Dialog -->
<ConfirmDialog
	open={archiveConfirmOpen}
	title={archiveTarget?.type === 'conversation'
		? archiveTargetIsArchived
			? 'Restore Conversation'
			: 'Archive Conversation'
		: archiveTargetIsArchived
			? 'Restore Thread'
			: 'Archive Thread'}
	message={archiveTarget?.type === 'conversation'
		? archiveTargetIsArchived
			? 'Restore this conversation to the active list?'
			: 'Are you sure you want to archive this conversation? It will be moved to the archived section.'
		: archiveTargetIsArchived
			? 'Restore this thread to the active list?'
			: 'Are you sure you want to archive this thread? It will be removed from the thread list.'}
	confirmLabel={archiveTargetIsArchived ? 'Restore' : 'Archive'}
	cancelLabel="Cancel"
	onconfirm={handleArchiveConfirm}
	oncancel={handleArchiveCancel}
	loading={archiveLoading}
/>

<!-- Delete Confirmation Dialog -->
<ConfirmDialog
	open={deleteConfirmOpen}
	title="Delete Conversation"
	message="Are you sure you want to delete this conversation? This action cannot be undone."
	confirmLabel="Delete"
	cancelLabel="Cancel"
	variant="danger"
	onconfirm={handleDeleteConfirm}
	oncancel={handleDeleteCancel}
	loading={deleteLoading}
/>

<style>
	.page-container {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: 100%;
		overflow: hidden;
	}

	/* AppShell fills available space above status bar */
	.page-container:global(> :first-child) {
		flex: 1;
		min-height: 0;
	}

	.status-bar {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		padding: var(--space-1) var(--space-4);
		background: var(--color-bg-secondary);
		border-top: 1px solid var(--color-border-subtle);
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		flex-shrink: 0;
		min-height: 20px;
	}

	.status-actions {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		font-family: var(--font-mono);
	}

	.error {
		color: var(--color-error);
	}

	.retry-btn {
		padding: 2px 6px;
		font-size: var(--text-2xs);
		font-family: var(--font-mono);
		background: var(--color-bg-tertiary);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.retry-btn:hover {
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
	}
</style>
