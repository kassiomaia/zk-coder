<script lang="ts">
	/**
	 * ConversationItem Component
	 * Individual conversation item in the sidebar list.
	 */
	import styles from './ConversationItem.module.css';
	import ListItemActions from '$lib/components/shared/ListItemActions.svelte';
	import InlineRenameInput from '$lib/components/shared/InlineRenameInput.svelte';
	import { formatRelativeTime } from '$lib/utils/formatRelativeTime';
	import type { Conversation } from '$lib/types/thread';

	interface Props {
		/** Conversation data */
		conversation: Conversation;
		/** Whether this conversation is selected */
		selected?: boolean;
		/** Whether threads are expanded (for aria-expanded) */
		ariaExpanded?: boolean;
		/** ID of the controlled element (for aria-controls) */
		ariaControls?: string;
		/** Click handler */
		onclick: () => void;
		/** Double-click handler (typically opens thread panel if threads exist) */
		ondoubleclick?: () => void;
		/** Delete handler */
		ondelete?: () => void;
		/** Rename handler */
		onrename?: (newTitle: string) => void;
		/** Archive handler */
		onarchive?: () => void;
		/** Optional inline styles (used for animation delays) */
		style?: string;
	}

	let {
		conversation,
		selected = false,
		ariaExpanded,
		ariaControls,
		onclick,
		ondoubleclick,
		ondelete,
		onrename,
		onarchive,
		style = ''
	}: Props = $props();

	let isHovered = $state(false);
	let isMenuOpen = $state(false);
	let isEditing = $state(false);

	function startEdit() {
		isEditing = true;
	}

	function handleRename(newTitle: string) {
		onrename?.(newTitle);
		isEditing = false;
	}
</script>

<li
	class={[
		styles.item,
		selected && styles.selected,
		ariaExpanded && styles.expanded,
		isMenuOpen && styles.menuOpen
	]
		.filter(Boolean)
		.join(' ')}
	role="option"
	aria-selected={selected}
	{style}
	onmouseenter={() => (isHovered = true)}
	onmouseleave={() => (isHovered = false)}
>
	{#if isEditing}
		<InlineRenameInput
			value={conversation.title}
			onSave={handleRename}
			onCancel={() => (isEditing = false)}
			maxLength={100}
		>
			{#snippet icon()}
				<div class={styles.icon}>
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
			{/snippet}
		</InlineRenameInput>
	{/if}

	<button
		class={[styles.button, isEditing && styles.hidden].filter(Boolean).join(' ')}
		{onclick}
		ondblclick={() => {
			// Only trigger double-click action if conversation has threads
			if (conversation.threads && conversation.threads.length > 0) {
				ondoubleclick?.();
			}
		}}
		aria-label="Select conversation: {conversation.title}"
		aria-expanded={ariaExpanded}
		aria-controls={ariaControls}
		type="button"
		tabindex={isEditing ? -1 : 0}
	>
		<div class={styles.icon}>
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
		<div class={styles.content}>
			<span class={styles.title}>{conversation.title}</span>
			<span class={styles.meta}>
				<span class={styles.time}>{formatRelativeTime(conversation.updatedAt)}</span>
			</span>
		</div>
	</button>

	{#if !isEditing && (isHovered || isMenuOpen) && (ondelete || onrename || onarchive)}
		<ListItemActions
			{ondelete}
			onrename={onrename ? startEdit : undefined}
			{onarchive}
			isArchived={conversation.archived ?? false}
			iconSize={14}
			onMenuOpen={() => (isMenuOpen = true)}
			onMenuClose={() => (isMenuOpen = false)}
		/>
	{/if}
</li>
