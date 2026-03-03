<script lang="ts">
	/**
	 * ConversationSidebar Component
	 * Main sidebar showing conversation history with grouping and search.
	 */
	import styles from './ConversationSidebar.module.css';
	import SearchBar from './SearchBar.svelte';
	import ConversationItem from './ConversationItem.svelte';
	import type { Conversation, ConversationGroup } from '$lib/types/thread';

	interface Props {
		/** List of conversations to display */
		conversations: Conversation[];
		/** Currently selected conversation ID */
		selectedId?: string;
		/** Handler when a conversation is selected */
		onSelect: (conversation: Conversation) => void;
		/** Handler when a conversation is double-clicked (typically opens thread panel) */
		onOpenChat?: (conversation: Conversation) => void;
		/** Handler for search input */
		onSearch?: (query: string) => void;
		/** Handler to create new conversation */
		onCreateNew?: () => void;
		/** Handler to delete conversation */
		onDelete?: (conversation: Conversation) => void;
		/** Handler to rename conversation */
		onRename?: (conversation: Conversation, newTitle: string) => void;
		/** Handler to archive conversation */
		onArchive?: (conversation: Conversation) => void;
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
		onSelect,
		onOpenChat,
		onSearch,
		onCreateNew,
		onDelete,
		onRename,
		onArchive,
		onSettings,
		loading = false,
		hasConfiguredProvider = true
	}: Props = $props();

	let searchQuery = $state('');

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
</script>

<aside class={styles.sidebar} aria-label="Conversations">
	<!-- Header -->
	<header class={styles.header}>
		<div class={styles.headerTop}>
			<h2 class={styles.title}>Chats</h2>
			{#if onCreateNew}
				<button
					class={styles.newButton}
					onclick={onCreateNew}
					aria-label="New conversation"
					title="New conversation"
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
						<line x1="12" y1="5" x2="12" y2="19" />
						<line x1="5" y1="12" x2="19" y2="12" />
					</svg>
				</button>
			{/if}
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
			{#each groupedConversations as group (group.label)}
				<section class={styles.group}>
					<h3 class={styles.groupLabel}>{group.label}</h3>
					<ul class={styles.list} role="listbox" aria-label="{group.label} conversations">
						{#each group.conversations as conv (conv.id)}
							<ConversationItem
								conversation={conv}
								selected={conv.id === selectedId}
								onclick={() => onSelect(conv)}
								ondoubleclick={onOpenChat ? () => onOpenChat(conv) : undefined}
								ondelete={onDelete ? () => onDelete(conv) : undefined}
								onrename={onRename ? (title) => onRename(conv, title) : undefined}
								onarchive={onArchive ? () => onArchive(conv) : undefined}
							/>
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
				class={[styles.settingsButton, !hasConfiguredProvider && styles.warning].filter(Boolean).join(' ')}
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
