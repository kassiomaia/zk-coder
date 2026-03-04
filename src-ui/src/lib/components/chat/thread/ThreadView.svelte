<script lang="ts">
	/**
	 * ThreadView Component
	 * Sidebar panel showing thread list with open/resolved grouping.
	 */
	import styles from './ThreadView.module.css';
	import ThreadItem from './ThreadItem.svelte';
	import NewThreadButton from './NewThreadButton.svelte';
	import type { Thread } from '$lib/types/thread';

	interface Props {
		/** List of threads to display */
		threads: Thread[];
		/** Currently selected thread ID */
		selectedThreadId?: string;
		/** Handler when a thread is selected */
		onSelect: (thread: Thread) => void;
		/** Handler to create a new thread */
		onCreateThread?: () => void;
		/** Handler to resolve a thread */
		onResolveThread?: (thread: Thread) => void;
		/** Handler to rename a thread */
		onRenameThread?: (thread: Thread, newTitle: string) => void;
		/** Handler to delete a thread */
		onDeleteThread?: (thread: Thread) => void;
		/** Handler to archive a thread */
		onArchiveThread?: (thread: Thread) => void;
		/** Handler to collapse the panel (desktop) */
		onCollapse?: () => void;
		/** Loading state */
		loading?: boolean;
	}

	let {
		threads,
		selectedThreadId,
		onSelect,
		onCreateThread,
		onResolveThread,
		onRenameThread,
		onDeleteThread,
		onArchiveThread,
		onCollapse,
		loading = false
	}: Props = $props();

	// Group threads by status
	let openThreads = $derived(threads.filter((t) => !t.metadata.resolved));
	let resolvedThreads = $derived(threads.filter((t) => t.metadata.resolved));
</script>

<aside class={styles.sidebar} aria-label="Threads">
	<div class={styles.header}>
		<div>
			<h2 class={styles.title}>THREADS</h2>
			<p class={styles.statusText}>
				{selectedThreadId ? 'Active thread' : 'Select a thread'}
			</p>
		</div>
		<div class={styles.headerActions}>
			{#if onCreateThread}
				<NewThreadButton onclick={onCreateThread} />
			{/if}
			{#if onCollapse}
				<button
					class={styles.collapseBtn}
					onclick={onCollapse}
					aria-label="Collapse thread panel"
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
						<polyline points="15 18 9 12 15 6" />
					</svg>
				</button>
			{/if}
		</div>
	</div>

	<div class={styles.content}>
		{#if loading}
			<div class={styles.loading} aria-label="Loading threads">
				<div class={styles.spinner}></div>
				<span>Loading threads...</span>
			</div>
		{:else if threads.length === 0}
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
				<p class={styles.emptyText}>No threads in this session</p>
				<p class={styles.emptyHint}>Create a thread to organize your conversation</p>
			</div>
		{:else}
			<!-- Open threads -->
			{#if openThreads.length > 0}
				<section class={styles.section}>
					<div class={styles.sectionHeader}>
						<h3 class={styles.sectionTitle}>Open</h3>
						<span class={styles.count}>{openThreads.length}</span>
					</div>
					<ul class={styles.list} role="listbox" aria-label="Open threads">
						{#each openThreads as thread, index (thread.id)}
							<ThreadItem
								{thread}
								selected={thread.id === selectedThreadId}
								onclick={() => onSelect(thread)}
								onresolve={onResolveThread ? () => onResolveThread(thread) : undefined}
								onrename={onRenameThread
									? (newTitle) => onRenameThread(thread, newTitle)
									: undefined}
								ondelete={onDeleteThread ? () => onDeleteThread(thread) : undefined}
								onarchive={onArchiveThread ? () => onArchiveThread(thread) : undefined}
								style="animation-delay: {index * 50}ms"
							/>
						{/each}
					</ul>
				</section>
			{/if}

			<!-- Resolved threads -->
			{#if resolvedThreads.length > 0}
				<section class={styles.section}>
					<div class={styles.sectionHeader}>
						<h3 class={styles.sectionTitle}>Resolved</h3>
						<span class={styles.count}>{resolvedThreads.length}</span>
					</div>
					<ul class={styles.list} role="listbox" aria-label="Resolved threads">
						{#each resolvedThreads as thread, index (thread.id)}
							<ThreadItem
								{thread}
								selected={thread.id === selectedThreadId}
								onclick={() => onSelect(thread)}
								onresolve={onResolveThread ? () => onResolveThread(thread) : undefined}
								onrename={onRenameThread
									? (newTitle) => onRenameThread(thread, newTitle)
									: undefined}
								ondelete={onDeleteThread ? () => onDeleteThread(thread) : undefined}
								onarchive={onArchiveThread ? () => onArchiveThread(thread) : undefined}
								style="animation-delay: {(openThreads.length + index) * 50}ms"
							/>
						{/each}
					</ul>
				</section>
			{/if}
		{/if}
	</div>

	<div class={styles.footer}>
		<div class={styles.footerInfo}>
			<span>{threads.length} {threads.length === 1 ? 'thread' : 'threads'}</span>
		</div>
	</div>
</aside>
