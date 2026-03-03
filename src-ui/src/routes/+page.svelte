<script lang="ts">
	/**
	 * Main Page - Chat Interface
	 * Connected to the real backend via chat store.
	 */
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import ChatLayout from '$lib/components/layout/ChatLayout.svelte';
	import ProvidersModal from '$lib/components/config/ProvidersModal.svelte';
	import ConfigModal from '$lib/components/config/ConfigModal.svelte';
	import CustomProviderModal from '$lib/components/config/CustomProviderModal.svelte';
	import SettingsModal from '$lib/components/config/SettingsModal.svelte';
	import ConfirmDialog from '$lib/components/shared/ConfirmDialog.svelte';
	import {
		chatStore,
		selectedConversation,
		selectedThread,
		conversationGroups,
		messagesLoading,
		chatError,
		isLoading,
		currentThreads,
		threadsLoading,
		configStore,
		hasConfiguredProvider
	} from '$lib/stores';
	import type { Conversation, Thread, ConversationGroup } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';

	// App state
	let appInfo = $state<{ name: string; version: string } | null>(null);
	let backendStatus = $state('Connecting...');

	// Get messages and threads directly from chat store via subscription
	let messages = $state<Message[]>([]);
	let threads = $state<Thread[]>([]);
	let conversations = $state<Conversation[]>([]);
	let currentError = $state<string | null>(null);
	let currentThreadsLoading = $state<LoadingState>('idle');

	type LoadingState = 'idle' | 'loading' | 'success' | 'error';

	// Config modal state
	type ModalState = 'closed' | 'providers' | 'config' | 'custom-provider' | 'settings';
	let activeModal = $state<ModalState>('closed');
	let previousModal = $state<ModalState>('closed'); // Track previous modal to return after config
	let selectedProviderId = $state<string | null>(null);
	let configInitialized = $state(false);

	// Archive confirmation state
	let archiveConfirmOpen = $state(false);
	let archiveTarget = $state<{ type: 'conversation' | 'thread'; item: Conversation | Thread } | null>(null);
	let archiveLoading = $state(false);

	// Delete confirmation state
	let deleteConfirmOpen = $state(false);
	let deleteTarget = $state<Conversation | null>(null);
	let deleteLoading = $state(false);

	// Subscribe to chat store changes
	$effect(() => {
		const unsubMsg = chatStore.subscribe((state) => {
			messages = state.messages;
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
		const unsubErr = chatError.subscribe((err) => {
			currentError = err;
		});
		return () => {
			unsubMsg();
			unsubConv();
			unsubThreads();
			unsubThreadsLoading();
			unsubErr();
		};
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
			console.log('Backend not available:', error);
		}

		// Initialize chat store (loads conversations from backend)
		await chatStore.initialize();

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

	async function handleSendMessage(content: string) {
		// Use LLM streaming - backend will use 'code' operation default model
		await chatStore.sendMessageWithLlm(content);
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
				// TODO: Implement proper archive in backend
				// For now, this behaves the same as delete
				await chatStore.deleteConversation((archiveTarget.item as Conversation).id);
			} else {
				await chatStore.deleteThread((archiveTarget.item as Thread).id);
			}
			archiveConfirmOpen = false;
			archiveTarget = null;
		} finally {
			archiveLoading = false;
		}
	}

	function handleSearch(query: string) {
		// TODO: Implement search via backend
		console.log('Search:', query);
	}

	function handleErrorRetry() {
		chatStore.clearError();
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
		// Track the previous modal so we can return to it after configuration
		previousModal = activeModal;
		activeModal = 'config';
	}

	function handleConfigSaved() {
		// Return to the previous modal (settings or providers)
		activeModal = previousModal === 'closed' ? 'settings' : previousModal;
		previousModal = 'closed';
	}

	function handleAddCustomProvider() {
		activeModal = 'custom-provider';
	}

	async function handleRemoveProvider(providerId: string) {
		try {
			await configStore.removeProvider(providerId);
		} catch (error) {
			console.error('Failed to remove provider:', error);
		}
	}

	// Flatten conversation groups for ChatLayout
	function flattenGroups(groups: ConversationGroup[]): Conversation[] {
		return groups.flatMap((group) => group.conversations);
	}
</script>

<svelte:head>
	<title>{appInfo?.name || 'Coder'} - AI Assistant</title>
</svelte:head>

<div class="page-container">
	<ChatLayout
		conversations={conversations}
		threads={threads}
		selectedConversation={$selectedConversation ?? undefined}
		selectedThreadId={$selectedThread?.id}
		{messages}
		onSelectConversation={handleSelectConversation}
		onSelectThread={handleSelectThread}
		onSendMessage={handleSendMessage}
		onCreateConversation={handleCreateConversation}
		onCreateThread={handleCreateThread}
		onResolveThread={handleResolveThread}
		onDeleteConversation={handleDeleteConversation}
		onRenameConversation={handleRenameConversation}
		onSearch={handleSearch}
		onSettings={handleOpenSettings}
		loading={$isLoading}
		threadsLoading={currentThreadsLoading === 'loading'}
		hasConfiguredProvider={$hasConfiguredProvider}
	/>

	<!-- Status indicator -->
	<div class="status-bar">
		<span class="status">
			{#if currentError}
				<span class="error">Error: {currentError}</span>
				<button class="retry-btn" onclick={handleErrorRetry} type="button">Dismiss</button>
			{:else}
				{backendStatus}
			{/if}
		</span>
		{#if appInfo}
			<span class="version">{appInfo.name} v{appInfo.version}</span>
		{/if}
	</div>
</div>

<!-- Configuration Modals -->
{#if configInitialized}
	<!-- Providers Modal (shown when no providers configured) -->
	<ProvidersModal
		open={activeModal === 'providers'}
		onclose={handleCloseModal}
		oncontinue={handleCloseModal}
		onconfigure={handleConfigureProvider}
		onaddcustom={handleAddCustomProvider}
	/>

	<!-- Config Modal (for entering API key) -->
	<ConfigModal
		open={activeModal === 'config'}
		onclose={handleCloseModal}
		providerId={selectedProviderId}
		onsaved={handleConfigSaved}
	/>

	<!-- Custom Provider Modal -->
	<CustomProviderModal
		open={activeModal === 'custom-provider'}
		onclose={() => activeModal = 'providers'}
		onsaved={(providerId: string) => {
			selectedProviderId = providerId;
			activeModal = 'config';
		}}
	/>

	<!-- Settings Modal (full settings) -->
	<SettingsModal
		open={activeModal === 'settings'}
		onclose={handleCloseModal}
		onconfigure={handleConfigureProvider}
		onaddcustom={handleAddCustomProvider}
		onremoveprovider={handleRemoveProvider}
	/>
{/if}

<!-- Archive Confirmation Dialog -->
<ConfirmDialog
	open={archiveConfirmOpen}
	title={archiveTarget?.type === 'conversation' ? 'Archive Conversation' : 'Archive Thread'}
	message={archiveTarget?.type === 'conversation'
		? 'Are you sure you want to archive this conversation? It will be moved to the archived section.'
		: 'Are you sure you want to archive this thread? It will be moved to the archived section.'}
	confirmLabel="Archive"
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
		height: 100vh;
		width: 100%;
		overflow: hidden;
	}

	.status-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-1) var(--space-4);
		background: var(--color-bg-secondary);
		border-top: 1px solid var(--color-border-primary);
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		flex-shrink: 0;
	}

	.status {
		font-family: var(--font-mono);
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.error {
		color: var(--color-text-error);
	}

	.retry-btn {
		padding: 2px 8px;
		font-size: var(--text-2xs);
		background: var(--color-bg-tertiary);
		border: 1px solid var(--color-border-primary);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		cursor: pointer;
	}

	.retry-btn:hover {
		background: var(--color-bg-hover);
	}

	.version {
		font-family: var(--font-mono);
		opacity: 0.7;
	}

	@media (max-width: 600px) {
		.status-bar {
			padding: var(--space-1) var(--space-2);
			font-size: var(--text-2xs);
		}
	}
</style>
