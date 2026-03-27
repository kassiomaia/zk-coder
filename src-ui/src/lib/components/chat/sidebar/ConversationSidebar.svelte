<script lang="ts">
    /**
     * ConversationSidebar Component
     * Main sidebar showing conversation history with inline thread navigation.
     *
     * Design Features:
     * - Unified header that adapts contextually (New Conversation vs New Thread)
     * - Inline thread rendering under selected conversation
     * - Hierarchical navigation with indentation
     * - Controlled reveal animation for thread expansion
     */
    import styles from './ConversationSidebar.module.css';
    import SearchBar from './SearchBar.svelte';
    import ConversationItem from './ConversationItem.svelte';
    import InlineThreadList from './InlineThreadList.svelte';
    import Popover from '$lib/components/shared/Popover.svelte';
    import type {PopoverItem} from '$lib/components/shared/popover.types';
    import type {Conversation, ConversationGroup, Thread} from '$lib/types/thread';

    interface Props {
		/** List of conversations to display */
		conversations: Conversation[];
		/** Currently selected conversation ID */
		selectedId?: string;
		/** Threads for the selected conversation (for inline rendering) */
		threadsForSelected?: Thread[];
		/** Thread IDs currently receiving streamed responses */
		activeStreamingThreadIds?: string[];
		/** Thread IDs with unseen assistant responses */
		unreadThreadIds?: string[];
		/** Currently selected thread ID */
		selectedThreadId?: string;
		/** Handler when a conversation is selected */
		onSelect: (conversation: Conversation) => void;
		/** Handler when a conversation is double-clicked */
		onOpenChat?: (conversation: Conversation) => void;
		/** Handler when a thread is selected */
		onSelectThread?: (thread: Thread) => void;
		/** Handler for search input */
		onSearch?: (query: string) => void;
		/** Handler to create new conversation */
		onCreateNew?: () => void;
		/** Handler to create new thread */
		onCreateThread?: () => void;
		/** Handler to delete conversation */
		onDelete?: (conversation: Conversation) => void;
		/** Handler to rename conversation */
		onRename?: (conversation: Conversation, newTitle: string) => void;
		/** Handler to archive conversation */
		onArchive?: (conversation: Conversation) => void;
		/** Handler to resolve thread */
		onResolveThread?: (thread: Thread) => void;
		/** Handler to rename thread */
		onRenameThread?: (thread: Thread, newTitle: string) => void;
		/** Handler to delete thread */
		onDeleteThread?: (thread: Thread) => void;
		/** Handler to archive thread */
		onArchiveThread?: (thread: Thread) => void;
		/** Handler for settings button click */
		onSettings?: () => void;
		/** Loading state */
		loading?: boolean;
		/** Whether any provider is configured */
		hasConfiguredProvider?: boolean;
		/** Search query for filtering */
		searchQuery?: string;
	}

	let {
		conversations,
		selectedId,
		threadsForSelected = [],
		activeStreamingThreadIds = [],
		unreadThreadIds = [],
		selectedThreadId,
		onSelect,
		onOpenChat,
		onSelectThread,
		onSearch,
		onCreateNew,
		onCreateThread,
		onDelete,
		onRename,
		onArchive,
		onResolveThread,
		onRenameThread,
		onDeleteThread,
		onArchiveThread,
		onSettings,
		loading = false,
		hasConfiguredProvider = true,
		searchQuery = ''
	}: Props = $props();

	let isNewMenuOpen = $state(false);
	let newButtonRef: HTMLButtonElement | undefined = $state(undefined);

	let hasSelectedConversation = $derived(!!selectedId);
	let hasSelectedThread = $derived(!!selectedThreadId);
	let hasThreads = $derived(threadsForSelected.length > 0);

	// Filter conversations based on search query
	function getFilteredConversations() {
		if (!searchQuery?.trim()) {
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
	}

	// Group filtered conversations by date
	function getGroupedFilteredConversations() {
		return groupConversations(getFilteredConversations());
	}

	let statusText = $derived(() => {
		if (hasSelectedThread) return 'Active thread';
		if (hasSelectedConversation) return 'Active conversation';
		return 'Select a chat to begin';
	});

	const ICONS = {
		newChat:
			'<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" /></svg>',
		newThread:
			'<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" /></svg>'
	};

	let newMenuItems = $derived<PopoverItem[]>([
		{
			id: 'new-chat',
			label: 'New chat',
			iconHtml: ICONS.newChat,
			disabled: !onCreateNew
		},
		{
			id: 'new-thread',
			label: 'New thread',
			iconHtml: ICONS.newThread,
			disabled: !hasSelectedConversation || !onCreateThread
		}
	]);

	// Group conversations by date
	function groupConversations(convList: Conversation[]): ConversationGroup[] {
		const now = new Date();
		const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		const yesterday = new Date(today.getTime() - 86400000);
		const weekAgo = new Date(today.getTime() - 7 * 86400000);

		const groups: ConversationGroup[] = [
			{ label: 'Today', conversations: [] },
			{ label: 'Yesterday', conversations: [] },
			{ label: 'This Week', conversations: [] },
			{ label: 'Older', conversations: [] }
		];

		for (const conv of convList) {
			if (!conv || !conv.updatedAt) continue;
			const date = new Date(conv.updatedAt);

			if (date >= today) {
				groups[0].conversations.push(conv);
			} else if (date >= yesterday) {
				groups[1].conversations.push(conv);
			} else if (date >= weekAgo) {
				groups[2].conversations.push(conv);
			} else {
				groups[3].conversations.push(conv);
			}
		}

		return groups.filter((g) => g.conversations.length > 0);
	}

	function handleSearch(query: string) {
		searchQuery = query;
		onSearch?.(query);
	}

	function toggleNewMenu(e: MouseEvent) {
		e.stopPropagation();
		isNewMenuOpen = !isNewMenuOpen;
	}

	function closeNewMenu() {
		isNewMenuOpen = false;
	}

	function handleNewMenuSelect(item: PopoverItem) {
		switch (item.id) {
			case 'new-chat':
				onCreateNew?.();
				break;
			case 'new-thread':
				onCreateThread?.();
				break;
		}
	}
</script>

<aside class={styles.sidebar} aria-label="Conversations">
	<!-- Header -->
	<header class={styles.header}>
		<div class={styles.headerTop}>
			<!-- Search -->
			{#if onSearch}
				<div class={styles.searchContainer}>
					<SearchBar
						value={searchQuery}
						oninput={handleSearch}
						placeholder="Search conversations and threads..."
					/>
					<div class={styles.headerActions}>
						<div class={styles.menuWrapper}>
							<button
								bind:this={newButtonRef}
								class={[styles.newButton, isNewMenuOpen && styles.active].filter(Boolean).join(' ')}
								onclick={toggleNewMenu}
								aria-label="Create new..."
								aria-expanded={isNewMenuOpen}
								aria-haspopup="true"
								type="button"
							>
								<svg
									width="16"
									height="16"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
								>
									<path d="M12 5v14M5 12h14" />
								</svg>
							</button>

							<Popover
								open={isNewMenuOpen}
								onClose={closeNewMenu}
								items={newMenuItems}
								onSelect={handleNewMenuSelect}
								anchorEl={newButtonRef}
								origin="bottom-right"
							/>
						</div>
					</div>
				</div>
			{:else}
				<div class={styles.statusRow}>
					<p class={styles.statusText}>{statusText()}</p>
				</div>
			{/if}
		</div>
	</header>

	<!-- Content -->
	<div class={styles.content}>
		{#if loading}
			<div class={styles.loading} aria-label="Loading conversations">
				<div class={styles.spinner}></div>
			</div>
		{:else if getFilteredConversations().length === 0}
			<div class={styles.empty}>
				<svg
					class={styles.emptyIcon}
					width="48"
					height="48"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
				</svg>
				<p class={styles.emptyText}>No conversations yet</p>
				{#if onCreateNew}
					<button class={styles.emptyButton} onclick={onCreateNew} type="button">
						Start a new chat
					</button>
				{/if}
			</div>
		{:else}
			{#each getGroupedFilteredConversations() as group, groupIndex (group.label)}
				{@const groupOffset = getGroupedFilteredConversations()
					.slice(0, groupIndex)
					.reduce((acc: number, g: ConversationGroup) => acc + g.conversations.length, 0)}
				<section class={styles.group}>
					<h3 class={styles.groupLabel}>{group.label}</h3>
					<ul class={styles.list} role="listbox" aria-label="{group.label} conversations">
						{#each group.conversations as conv, index (conv.id)}
							{@const isExpanded = conv.id === selectedId && hasThreads}
							<li
								class={[styles.conversationWrapper, isExpanded && styles.expanded]
									.filter(Boolean)
									.join(' ')}
							>
								<ConversationItem
									conversation={conv}
									selected={conv.id === selectedId}
									onclick={() => onSelect(conv)}
									ondoubleclick={onOpenChat ? () => onOpenChat(conv) : undefined}
									ondelete={onDelete ? () => onDelete(conv) : undefined}
									onrename={onRename ? (title) => onRename(conv, title) : undefined}
									onarchive={onArchive ? () => onArchive(conv) : undefined}
									style="animation-delay: calc({groupOffset + index} * var(--stagger-normal))"
									ariaExpanded={isExpanded}
									ariaControls={isExpanded ? 'inline-threads' : undefined}
								/>
								{#if conv.id === selectedId && hasThreads && onSelectThread}
									<InlineThreadList
										threads={threadsForSelected}
										{activeStreamingThreadIds}
										{unreadThreadIds}
										{selectedThreadId}
										expanded={true}
										{onSelectThread}
										{onResolveThread}
										{onRenameThread}
										{onDeleteThread}
										{onArchiveThread}
									/>
								{/if}
							</li>
						{/each}
					</ul>
				</section>
			{/each}
		{/if}
	</div>

	<!-- Footer with Settings -->
	{#if onSettings}
		<footer class={styles.footer}>
			<button
				class={[styles.settingsButton, !hasConfiguredProvider && styles.warning]
					.filter(Boolean)
					.join(' ')}
				onclick={onSettings}
				aria-label="Open settings"
				title={!hasConfiguredProvider ? 'Configure LLM providers' : 'Settings'}
				type="button"
			>
				<svg
					width="18"
					height="18"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<circle cx="12" cy="12" r="3" />
					<path
						d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
					/>
				</svg>
				<span class={styles.settingsLabel}>Settings</span>
				{#if !hasConfiguredProvider}
					<span class={styles.warningBadge}>!</span>
				{/if}
			</button>
		</footer>
	{/if}
</aside>
