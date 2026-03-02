<script lang="ts">
	/**
	 * ChatLayout Component
	 * Main responsive layout for the chat application.
	 * Manages sidebar visibility, thread panel, and responsive behavior.
	 */
	import { onMount } from 'svelte';
	import styles from './ChatLayout.module.css';
	import ConversationSidebar from '../chat/sidebar/ConversationSidebar.svelte';
	import ThreadView from '../chat/thread/ThreadView.svelte';
	import ThreadPanel from '../chat/thread/ThreadPanel.svelte';
	import ChatArea from '../chat/ChatArea.svelte';
	import type { Conversation, Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';

	// localStorage key for thread panel state
	const THREAD_PANEL_STORAGE_KEY = 'coder:threadPanelOpen';

	interface Props {
		/** Conversations for the sidebar */
		conversations: Conversation[];
		/** Threads for the current conversation (loaded separately from backend) */
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
		onSearch,
		onSettings,
		loading = false,
		threadsLoading = false,
		hasConfiguredProvider = true
	}: Props = $props();

	// UI state for responsive behavior
	let sidebarOpen = $state(false);
	let threadPanelOpen = $state(true); // Default to open

	// Load persisted state on mount
	onMount(() => {
		const stored = localStorage.getItem(THREAD_PANEL_STORAGE_KEY);
		if (stored !== null) {
			threadPanelOpen = stored === 'true';
		}
	});

	// Persist state changes
	function setThreadPanelOpen(open: boolean) {
		threadPanelOpen = open;
		try {
			localStorage.setItem(THREAD_PANEL_STORAGE_KEY, String(open));
		} catch {
			// localStorage not available
		}
	}

	// Derived state - show thread panel when a conversation is selected
	let hasThreads = $derived(threads.length > 0);
	let showThreadPanel = $derived(!!selectedConversation);

	// Derive selected thread from threads array to always get fresh data
	let selectedThread = $derived(
		selectedThreadId ? threads.find((t) => t.id === selectedThreadId) : undefined
	);

	function toggleSidebar() {
		sidebarOpen = !sidebarOpen;
	}

	function toggleThreadPanel() {
		setThreadPanelOpen(!threadPanelOpen);
	}

	function closeSidebar() {
		sidebarOpen = false;
	}

	function handleSelectConversation(conversation: Conversation) {
		onSelectConversation?.(conversation);
		closeSidebar();
	}

	function handleOpenChatWithThreads(conversation: Conversation) {
		// Select the conversation first
		onSelectConversation?.(conversation);
		// Open the thread panel
		setThreadPanelOpen(true);
		closeSidebar();
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

	// Handle sending message
	function handleSendMessage(content: string) {
		onSendMessage?.(content);
	}

	// Handle reopening a resolved thread
	function handleReopenThread() {
		if (selectedThread && onResolveThread) {
			// Pass the original thread - the handler will toggle its resolved state
			// Since it's currently resolved (true), toggling will make it unresolved (false)
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

<div class={styles.layout} data-sidebar-open={sidebarOpen} data-thread-open={threadPanelOpen}>
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

		{#if selectedConversation}
			<button
				class={styles.menuButton}
				onclick={toggleThreadPanel}
				aria-label={threadPanelOpen ? 'Hide threads' : 'Show threads'}
				aria-expanded={threadPanelOpen}
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
					<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
				</svg>
				{#if threads.length > 0}
					<span class={styles.badge}>{threads.length}</span>
				{/if}
			</button>
		{/if}
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

		<!-- Conversation sidebar -->
		<div class={[styles.sidebarWrapper, sidebarOpen && styles.open].filter(Boolean).join(' ')}>
			<ConversationSidebar
				{conversations}
				selectedId={selectedConversation?.id}
				onSelect={handleSelectConversation}
				onOpenChat={handleOpenChatWithThreads}
				{onSearch}
				onCreateNew={onCreateConversation}
				onDelete={onDeleteConversation}
				onRename={onRenameConversation}
				{onSettings}
				{hasConfiguredProvider}
			/>
		</div>

		<!-- Main chat area -->
		<div class={styles.content}>
			<!-- Desktop thread toggle button -->
			{#if selectedConversation && !threadPanelOpen}
				<button
					class={styles.threadToggle}
					onclick={toggleThreadPanel}
					aria-label="Show thread panel"
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
						<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
					</svg>
					<span class={styles.toggleLabel}>Threads</span>
					<span class={styles.toggleBadge}>{threads.length}</span>
				</button>
			{/if}

			{#if showThreadPanel}
				<!-- Thread view with panel -->
				<div class={styles.threadLayout}>
					<div
						class={[styles.threadPanelWrapper, threadPanelOpen && styles.open]
							.filter(Boolean)
							.join(' ')}
					>
						<ThreadView
							{threads}
							selectedThreadId={selectedThread?.id}
							onSelect={handleSelectThread}
							{onCreateThread}
							{onResolveThread}
							onCollapse={toggleThreadPanel}
							loading={threadsLoading}
						/>
					</div>

					<!-- Thread content or main chat -->
					<div class={styles.threadContent}>
						{#if selectedThread}
							<ThreadPanel
								thread={selectedThread}
								{messages}
								{breadcrumbPath}
								onSendMessage={handleSendMessage}
								onReopenThread={handleReopenThread}
								{loading}
							/>
						{:else}
							<ChatArea
								{messages}
								{threads}
								{loading}
								onSendMessage={handleSendMessage}
								onThreadClick={handleSelectThreadById}
							/>
						{/if}
					</div>
				</div>
			{:else}
				<!-- Standard chat area (no threads) -->
				<ChatArea {messages} {loading} onSendMessage={handleSendMessage} />
			{/if}
		</div>

		<!-- Thread panel backdrop (mobile) -->
		{#if threadPanelOpen && showThreadPanel}
			<button
				class={[styles.backdrop, styles.right].filter(Boolean).join(' ')}
				onclick={toggleThreadPanel}
				aria-label="Close thread panel"
				type="button"
			>
			</button>
		{/if}
	</div>
</div>
