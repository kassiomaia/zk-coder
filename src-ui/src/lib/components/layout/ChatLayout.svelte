<script lang="ts">
	/**
	 * ChatLayout Component
	 * Main responsive layout for the chat application.
	 * Single unified sidebar with hierarchical navigation (conversations + inline threads).
	 */
	import { onMount, onDestroy } from 'svelte';
	import styles from './ChatLayout.module.css';
	import ConversationSidebar from '../chat/sidebar/ConversationSidebar.svelte';
	import ThreadPanel from '../chat/thread/ThreadPanel.svelte';
	import ChatArea from '../chat/ChatArea.svelte';
	import type { Conversation, Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';
	import { createAppShortcuts, unregisterShortcuts } from '$lib/utils/keyboardShortcuts';

	interface Props {
		/** Conversations for the sidebar */
		conversations: Conversation[];
		/** Threads for the current conversation */
		threads: Thread[];
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
		onSendMessage?: (content: string) => void;
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
		/** Loading state */
		loading?: boolean;
		/** Threads loading state */
		threadsLoading?: boolean;
		/** Whether any provider is configured */
		hasConfiguredProvider?: boolean;
	}

	let {
		conversations,
		threads,
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
		hasConfiguredProvider = true
	}: Props = $props();

	// UI state for responsive behavior (mobile sidebar)
	let sidebarOpen = $state(false);

	// Keyboard shortcuts
	let shortcutIds: string[] = [];

	onMount(() => {
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

	function toggleSidebar() {
		sidebarOpen = !sidebarOpen;
	}

	function closeSidebar() {
		sidebarOpen = false;
	}

	function handleSelectConversation(conversation: Conversation) {
		onSelectConversation?.(conversation);
		closeSidebar();
	}

	function handleSelectThread(thread: Thread) {
		onSelectThread?.(thread);
		closeSidebar();
	}

	function handleSelectThreadById(threadId: string) {
		const thread = threads.find((t) => t.id === threadId);
		if (thread) {
			onSelectThread?.(thread);
		}
	}

	function handleSendMessage(content: string) {
		onSendMessage?.(content);
	}

	function handleReopenThread() {
		if (selectedThread && onResolveThread) {
			onResolveThread(selectedThread);
		}
	}

	// Get breadcrumb path from thread hierarchy
	let breadcrumbPath = $derived(
		selectedThread
			? [{ id: selectedConversation?.id || '', title: selectedConversation?.title || '' }]
			: []
	);
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
				selectedThreadId={selectedThread?.id}
				onSelect={handleSelectConversation}
				onSelectThread={handleSelectThread}
				{onSearch}
				onCreateNew={onCreateConversation}
				onCreateThread={onCreateThread}
				onDelete={onDeleteConversation}
				onRename={onRenameConversation}
				onArchive={onArchiveConversation}
				onResolveThread={onResolveThread}
				onRenameThread={onRenameThread}
				onDeleteThread={onDeleteThread}
				onArchiveThread={onArchiveThread}
				{onSettings}
				{hasConfiguredProvider}
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
					onReopenThread={handleReopenThread}
					onToggleSidebar={toggleSidebar}
					{loading}
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
					onSendMessage={handleSendMessage}
					onThreadClick={handleSelectThreadById}
				/>
			{:else}
				<!-- Empty state -->
				<ChatArea
					{messages}
					threadId={selectedThreadId}
					emptyContext="none"
					{loading}
					onSendMessage={handleSendMessage}
				/>
			{/if}
		</div>
	</div>
</div>
