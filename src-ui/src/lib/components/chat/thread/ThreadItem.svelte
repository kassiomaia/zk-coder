<script lang="ts">
    /**
     * ThreadItem Component
     * Individual thread item in the thread list.
     */
    import styles from './ThreadItem.module.css';
    import ListItemActions from '$lib/components/shared/ListItemActions.svelte';
    import InlineRenameInput from '$lib/components/shared/InlineRenameInput.svelte';
    import {CheckCircleIcon, MessageSquareIcon} from '$lib/icons';
    import {formatRelativeTime} from '$lib/utils/formatRelativeTime';
    import type {Thread} from '$lib/types/thread';

    interface Props {
		/** Thread data */
		thread: Thread;
		/** Whether this thread is selected */
		selected?: boolean;
		/** Thread currently receiving assistant response */
		isStreaming?: boolean;
		/** Thread has unseen assistant response */
		hasUnread?: boolean;
		/** Visual variant: 'default' for full size, 'inline' for reduced size with indentation */
		variant?: 'default' | 'inline';
		/** Click handler */
		onclick: () => void;
		/** Resolve handler */
		onresolve?: () => void;
		/** Delete handler */
		ondelete?: () => void;
		/** Archive handler */
		onarchive?: () => void;
		/** Rename handler */
		onrename?: (newTitle: string) => void;
		/** Optional inline styles (used for animation delays) */
		style?: string;
	}

	let {
		thread,
		selected = false,
		isStreaming = false,
		hasUnread = false,
		variant = 'default',
		onclick,
		onresolve,
		ondelete,
		onarchive,
		onrename,
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
		hasUnread && styles.hasUnread,
		isStreaming && styles.streaming,
		isMenuOpen && styles.menuOpen,
		thread.metadata.resolved && styles.resolved
	]
		.filter(Boolean)
		.join(' ')}
	role="option"
	aria-selected={selected}
	data-variant={variant}
	{style}
	onmouseenter={() => (isHovered = true)}
	onmouseleave={() => (isHovered = false)}
>
	{#if isEditing}
		<InlineRenameInput
			value={thread.title}
			onSave={handleRename}
			onCancel={() => (isEditing = false)}
			maxLength={100}
		>
			{#snippet icon()}
				<div class={styles.icon}>
					{#if thread.metadata.resolved}
						<CheckCircleIcon size={16} />
					{:else}
						<MessageSquareIcon size={16} />
					{/if}
				</div>
			{/snippet}
		</InlineRenameInput>
	{/if}

	<button
		class={[styles.button, isEditing && styles.hidden].filter(Boolean).join(' ')}
		{onclick}
		aria-label="Select thread: {thread.title}"
		type="button"
		tabindex={isEditing ? -1 : 0}
	>
		<div class={styles.icon}>
			{#if thread.metadata.resolved}
				<CheckCircleIcon size={16} />
			{:else}
				<MessageSquareIcon size={16} />
			{/if}
		</div>
		<div class={styles.content}>
			<span class={styles.title}>{thread.title}</span>
			<span class={styles.meta}>
				<span class={styles.messageCount}>{thread.messageCount} messages</span>
				<span class={styles.dot}>·</span>
				<span class={styles.time}>{formatRelativeTime(thread.updatedAt)}</span>
				{#if isStreaming || hasUnread}
					<span class={styles.indicators} aria-hidden="true">
						{#if isStreaming}
							<span class={[styles.indicatorDot, styles.streamingDot].join(' ')}></span>
						{/if}
						{#if hasUnread}
							<span class={[styles.indicatorDot, styles.unreadDot].join(' ')}></span>
						{/if}
					</span>
				{/if}
			</span>
		</div>
	</button>

	{#if !isEditing && (isHovered || isMenuOpen) && (onresolve || onrename || ondelete || onarchive)}
		<ListItemActions
			{onresolve}
			isResolved={thread.metadata.resolved}
			isArchived={thread.metadata.archived ?? false}
			onrename={onrename ? startEdit : undefined}
			{onarchive}
			{ondelete}
			iconSize={16}
			onMenuOpen={() => (isMenuOpen = true)}
			onMenuClose={() => (isMenuOpen = false)}
		/>
	{/if}
</li>
