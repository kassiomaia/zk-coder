<script lang="ts">
	/**
	 * InlineThreadList Component
	 * Renders threads inline under the selected conversation in the sidebar.
	 *
	 * Design Features:
	 * - Continuous cluster without section dividers
	 * - Resolved threads differentiated by tonal reduction (opacity)
	 * - Open threads appear first, resolved threads follow
	 * - Controlled reveal animation with max-height and opacity
	 * - Stagger animation on thread items using design system tokens
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

	// Sort threads: open first, then resolved (no grouping labels)
	let sortedThreads = $derived([
		...threads.filter((t) => !t.metadata.resolved),
		...threads.filter((t) => t.metadata.resolved)
	]);

	let hasThreads = $derived(threads.length > 0);
</script>

{#if hasThreads}
	<div
		class={[styles.container, expanded && styles.expanded].filter(Boolean).join(' ')}
		id="inline-threads"
		data-threads-expanded={expanded || undefined}
		aria-label="Conversation threads"
	>
		<ul class={styles.list} role="listbox" aria-label="Threads">
			{#each sortedThreads as thread, index (thread.id)}
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
						style="animation-delay: calc({index} * var(--stagger-normal))"
					/>
				</li>
			{/each}
		</ul>
	</div>
{/if}
