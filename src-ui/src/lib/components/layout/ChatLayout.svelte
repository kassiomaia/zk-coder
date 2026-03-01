<script lang="ts">
	/**
	 * ChatLayout Component
	 * Main responsive layout for the chat application.
	 * Manages sidebar visibility, thread panel, and responsive behavior.
	 */
	import styles from './ChatLayout.module.css';
	import ConversationSidebar from '../chat/sidebar/ConversationSidebar.svelte';
	import ThreadView from '../chat/thread/ThreadView.svelte';
	import ThreadPanel from '../chat/thread/ThreadPanel.svelte';
	import ChatArea from '../chat/ChatArea.svelte';
	import type { Conversation, Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';

	interface Props {
		/** Conversations for the sidebar */
		conversations: Conversation[];
		/** Currently selected conversation */
		selectedConversation?: Conversation;
		/** Currently selected thread */
		selectedThread?: Thread;
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
		/** Handler for search */
		onSearch?: (query: string) => void;
		/** Loading state */
		loading?: boolean;
	}

	let {
		conversations,
		selectedConversation,
		selectedThread,
		messages,
		onSelectConversation,
		onSelectThread,
		onSendMessage,
		onCreateConversation,
		onCreateThread,
		onResolveThread,
		onDeleteConversation,
		onSearch,
		loading = false
	}: Props = $props();

	// UI state for responsive behavior
	let sidebarOpen = $state(false);
	let threadPanelOpen = $state(false);

	// Derived state
	let showThreadPanel = $derived(!!selectedThread);

	function toggleSidebar() {
		sidebarOpen = !sidebarOpen;
	}

	function toggleThreadPanel() {
		threadPanelOpen = !threadPanelOpen;
	}

	function closeSidebar() {
		sidebarOpen = false;
	}

	function closeThreadPanel() {
		threadPanelOpen = false;
	}

	function handleSelectConversation(conversation: Conversation) {
		onSelectConversation?.(conversation);
		closeSidebar();
	}

	function handleSelectThread(thread: Thread) {
		onSelectThread?.(thread);
		threadPanelOpen = true;
	}

	// Handle sending message
	function handleSendMessage(content: string) {
		onSendMessage?.(content);
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

		{#if showThreadPanel}
			<button
				class={styles.menuButton}
				onclick={toggleThreadPanel}
				aria-label="Toggle thread panel"
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
			</button>
		{/if}
	</header>

	<!-- Main content area -->
	<div class={styles.main}>
		<!-- Sidebar backdrop (mobile) -->
		{#if sidebarOpen}
			<button class={styles.backdrop} onclick={closeSidebar} aria-label="Close sidebar" type="button">
			</button>
		{/if}

		<!-- Conversation sidebar -->
		<div class={[styles.sidebarWrapper, sidebarOpen && styles.open].filter(Boolean).join(' ')}>
			<ConversationSidebar
				{conversations}
				selectedId={selectedConversation?.id}
				onSelect={handleSelectConversation}
				onSearch={onSearch}
				onCreateNew={onCreateConversation}
				onDelete={onDeleteConversation}
			/>
		</div>

		<!-- Main chat area -->
		<div class={styles.content}>
			{#if selectedThread}
				<!-- Thread view with panel -->
				<div class={styles.threadLayout}>
					<div class={[styles.threadPanelWrapper, threadPanelOpen && styles.open].filter(Boolean).join(' ')}>
						<ThreadView
							threads={selectedConversation?.threads || []}
							selectedThreadId={selectedThread.id}
							onSelect={handleSelectThread}
							onCreateThread={onCreateThread}
							onResolveThread={onResolveThread}
						/>
					</div>

					<!-- Thread content -->
					<div class={styles.threadContent}>
						<ThreadPanel
							thread={selectedThread}
							{messages}
							breadcrumbPath={breadcrumbPath}
							onSendMessage={handleSendMessage}
							{loading}
						/>
					</div>
				</div>
			{:else}
				<!-- Standard chat area -->
				<ChatArea
					{messages}
					{loading}
					onSendMessage={handleSendMessage}
				/>
			{/if}
		</div>

		<!-- Thread panel backdrop (mobile) -->
		{#if threadPanelOpen && showThreadPanel}
			<button
				class={[styles.backdrop, styles.right].filter(Boolean).join(' ')}
				onclick={closeThreadPanel}
				aria-label="Close thread panel"
				type="button"
			>
			</button>
		{/if}
	</div>
</div>
