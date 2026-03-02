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

	async function handleDeleteConversation(conversation: Conversation) {
		await chatStore.deleteConversation(conversation.id);
	}

	async function handleRenameConversation(conversation: Conversation, newTitle: string) {
		await chatStore.renameConversation(conversation.id, newTitle);
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
