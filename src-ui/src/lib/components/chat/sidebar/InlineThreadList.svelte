<script lang="ts">
	/**
	 * InlineThreadList Component
	 * Renders threads inline under the selected conversation in the sidebar.
	 *
	 * Design Features:
	 * - Groups threads by status (Open/Resolved) with subtle section labels
	 * - Uses `variant="inline"` on ThreadItem for reduced visual weight
	 * - Controlled reveal animation with max-height and opacity
	 * - Stagger animation on thread items (50ms delay per item)
	 * - Scroll stability with overflow-anchor: none
	 */
	import styles from './InlineThreadList.module.css';
	import ThreadItem from '../thread/ThreadItem.svelte';
	import type { Thread } from '$lib/types/thread';

	interface Props {
		/** List of threads to display */
		threads: Thread[];
		/** Currently selected thread ID */
		selectedThreadId?: string;
		/** Whether the thread list is expanded */
		expanded?: boolean;
		/** Handler when a thread is selected */
		onSelectThread: (thread: Thread) => void;
		/** Handler to resolve a thread */
		onResolveThread?: (thread: Thread) => void;
		/** Handler to rename a thread */
		onRenameThread?: (thread: Thread, newTitle: string) => void;
		/** Handler to delete a thread */
		onDeleteThread?: (thread: Thread) => void;
		/** Handler to archive a thread */
		onArchiveThread?: (thread: Thread) => void;
	}

	let {
		threads,
		selectedThreadId,
		expanded = true,
		onSelectThread,
		onResolveThread,
		onRenameThread,
		onDeleteThread,
		onArchiveThread
	}: Props = $props();

	// Group threads by status (matches existing ThreadView pattern)
	let openThreads = $derived(threads.filter((t) => !t.metadata.resolved));
	let resolvedThreads = $derived(threads.filter((t) => t.metadata.resolved));
	let hasThreads = $derived(threads.length > 0);
</script>

{#if hasThreads}
	<div
		class={[styles.container, expanded && styles.expanded].filter(Boolean).join(' ')}
		id="inline-threads"
		aria-label="Conversation threads"
	>
		<!-- Open threads -->
		{#if openThreads.length > 0}
			<section class={styles.section}>
				{#if resolvedThreads.length > 0}
					<div class={styles.sectionHeader}>
						<span class={styles.sectionLabel}>Open</span>
						<span class={styles.count}>{openThreads.length}</span>
					</div>
				{/if}
				<ul class={styles.list} role="listbox" aria-label="Open threads">
					{#each openThreads as thread, index (thread.id)}
						<li class={styles.itemWrapper}>
							<ThreadItem
								{thread}
								variant="inline"
								selected={thread.id === selectedThreadId}
								onclick={() => onSelectThread(thread)}
								onresolve={onResolveThread ? () => onResolveThread(thread) : undefined}
								onrename={onRenameThread ? (newTitle) => onRenameThread(thread, newTitle) : undefined}
								ondelete={onDeleteThread ? () => onDeleteThread(thread) : undefined}
								onarchive={onArchiveThread ? () => onArchiveThread(thread) : undefined}
								style="animation-delay: {index * 50}ms"
							/>
						</li>
					{/each}
				</ul>
			</section>
		{/if}

		<!-- Resolved threads -->
		{#if resolvedThreads.length > 0}
			<section class={styles.section}>
				<div class={styles.sectionHeader}>
					<span class={styles.sectionLabel}>Resolved</span>
					<span class={styles.count}>{resolvedThreads.length}</span>
				</div>
				<ul class={styles.list} role="listbox" aria-label="Resolved threads">
					{#each resolvedThreads as thread, index (thread.id)}
						<li class={styles.itemWrapper}>
							<ThreadItem
								{thread}
								variant="inline"
								selected={thread.id === selectedThreadId}
								onclick={() => onSelectThread(thread)}
								onresolve={onResolveThread ? () => onResolveThread(thread) : undefined}
								onrename={onRenameThread ? (newTitle) => onRenameThread(thread, newTitle) : undefined}
								ondelete={onDeleteThread ? () => onDeleteThread(thread) : undefined}
								onarchive={onArchiveThread ? () => onArchiveThread(thread) : undefined}
								style="animation-delay: {(openThreads.length + index) * 50}ms"
							/>
						</li>
					{/each}
				</ul>
			</section>
		{/if}
	</div>
{/if}
