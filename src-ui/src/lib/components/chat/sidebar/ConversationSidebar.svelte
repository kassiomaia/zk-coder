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
	import type { Conversation, ConversationGroup, Thread } from '$lib/types/thread';

	interface Props {
		/** List of conversations to display */
		conversations: Conversation[];
		/** Currently selected conversation ID */
		selectedId?: string;
		/** Threads for the selected conversation (for inline rendering) */
		threadsForSelected?: Thread[];
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
	}

	let {
		conversations,
		selectedId,
		threadsForSelected = [],
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
		hasConfiguredProvider = true
	}: Props = $props();

	let searchQuery = $state('');
	let isNewMenuOpen = $state(false);
	let newButtonRef: HTMLButtonElement | undefined = $state(undefined);
	let newMenuRef: HTMLDivElement | undefined = $state(undefined);

	// Derived state for contextual UI
	let hasSelectedConversation = $derived(!!selectedId);
	let hasSelectedThread = $derived(!!selectedThreadId);
	let hasThreads = $derived(threadsForSelected.length > 0);

	// Thread status counts for selected conversation
	let openThreadCount = $derived(threadsForSelected.filter((t) => !t.metadata.resolved).length);
	let resolvedThreadCount = $derived(threadsForSelected.filter((t) => t.metadata.resolved).length);

	// Status text based on selection state
	let statusText = $derived(() => {
		if (hasSelectedThread) return 'Active thread';
		if (hasSelectedConversation) return 'Active conversation';
		return 'Select a chat to begin';
	});

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

	let groupedConversations = $derived(groupConversations(conversations));

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

	function handleWindowClick(e: MouseEvent) {
		if (
			isNewMenuOpen &&
			newMenuRef &&
			!newMenuRef.contains(e.target as Node) &&
			!newButtonRef?.contains(e.target as Node)
		) {
			closeNewMenu();
		}
	}

	function handleCreateChat() {
		onCreateNew?.();
		closeNewMenu();
	}

	function handleCreateThread() {
		onCreateThread?.();
		closeNewMenu();
	}
</script>

	<svelte:window onclick={handleWindowClick} />

	<aside class={styles.sidebar} aria-label="Conversations">
		<!-- Header -->
		<header class={styles.header}>
			<div class={styles.headerTop}>
				<div class={styles.headerInfo}>
					<h2 class={styles.title}>CHATS</h2>
				</div>
				<div class={styles.headerActions}>
					<!-- Single "New" button with dropdown -->
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

						{#if isNewMenuOpen}
							<div
								bind:this={newMenuRef}
								class={styles.menu}
								role="menu"
							>
								{#if onCreateNew}
									<button
										class={styles.menuItem}
										onclick={handleCreateChat}
										role="menuitem"
										type="button"
									>
										<div class={styles.menuIcon}>
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
												<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
											</svg>
										</div>
										<span>New chat</span>
									</button>
								{/if}

								{#if onCreateThread}
									<button
										class={styles.menuItem}
										onclick={handleCreateThread}
										role="menuitem"
										type="button"
										disabled={!hasSelectedConversation}
									>
										<div class={styles.menuIcon}>
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
												<path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" />
											</svg>
										</div>
										<span>New thread</span>
									</button>
								{/if}
							</div>
						{/if}
					</div>
				</div>
			</div>
			<!-- Status row below title/buttons -->

		<div class={styles.statusRow}>
			<p class={styles.statusText}>{statusText()}</p>
		</div>

		<!-- Search -->
		{#if onSearch}
			<SearchBar
				value={searchQuery}
				oninput={handleSearch}
				placeholder="Search conversations..."
			/>
		{/if}
	</header>


	<!-- Content -->
	<div class={styles.content}>
		{#if loading}
			<div class={styles.loading} aria-label="Loading conversations">
				<div class={styles.spinner}></div>
			</div>
		{:else if conversations.length === 0}
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
					<button
						class={styles.emptyButton}
						onclick={onCreateNew}
						type="button"
					>
						Start a new chat
					</button>
				{/if}
			</div>
		{:else}
			{#each groupedConversations as group, groupIndex (group.label)}
				{@const groupOffset = groupedConversations
					.slice(0, groupIndex)
					.reduce((acc, g) => acc + g.conversations.length, 0)}
				<section class={styles.group}>
					<h3 class={styles.groupLabel}>{group.label}</h3>
					<ul class={styles.list} role="listbox" aria-label="{group.label} conversations">
						{#each group.conversations as conv, index (conv.id)}
							{@const isExpanded = conv.id === selectedId && hasThreads}
							<li class={[styles.conversationWrapper, isExpanded && styles.expanded].filter(Boolean).join(' ')}>
								<ConversationItem
									conversation={conv}
									selected={conv.id === selectedId}
									onclick={() => onSelect(conv)}
									ondoubleclick={onOpenChat ? () => onOpenChat(conv) : undefined}
									ondelete={onDelete ? () => onDelete(conv) : undefined}
									onrename={onRename ? (title) => onRename(conv, title) : undefined}
									onarchive={onArchive ? () => onArchive(conv) : undefined}
									style="animation-delay: {(groupOffset + index) * 50}ms"
									ariaExpanded={isExpanded}
									ariaControls={isExpanded ? 'inline-threads' : undefined}
									openThreadCount={isExpanded ? openThreadCount : 0}
									resolvedThreadCount={isExpanded ? resolvedThreadCount : 0}
								/>
								{#if conv.id === selectedId && hasThreads && onSelectThread}
									<InlineThreadList
										threads={threadsForSelected}
										{selectedThreadId}
										expanded={true}
										onSelectThread={onSelectThread}
										onResolveThread={onResolveThread}
										onRenameThread={onRenameThread}
										onDeleteThread={onDeleteThread}
										onArchiveThread={onArchiveThread}
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
					<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
				</svg>
				<span class={styles.settingsLabel}>Settings</span>
				{#if !hasConfiguredProvider}
					<span class={styles.warningBadge}>!</span>
				{/if}
			</button>
		</footer>
	{/if}
</aside>
