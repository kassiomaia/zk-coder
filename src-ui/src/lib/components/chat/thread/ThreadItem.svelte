<script lang="ts">
	/**
	 * ThreadItem Component
	 * Individual thread item in the thread list.
	 */
	import styles from './ThreadItem.module.css';
	import type { Thread } from '$lib/types/thread';

	interface Props {
		/** Thread data */
		thread: Thread;
		/** Whether this thread is selected */
		selected?: boolean;
		/** Click handler */
		onclick: () => void;
		/** Resolve handler */
		onresolve?: () => void;
		/** Delete handler */
		ondelete?: () => void;
	}

	let { thread, selected = false, onclick, onresolve, ondelete }: Props = $props();

	let showActions = $state(false);

	function formatTime(date: Date): string {
		const now = new Date();
		const diff = now.getTime() - new Date(date).getTime();
		const minutes = Math.floor(diff / 60000);
		const hours = Math.floor(diff / 3600000);
		const days = Math.floor(diff / 86400000);

		if (minutes < 1) return 'Just now';
		if (minutes < 60) return `${minutes}m ago`;
		if (hours < 24) return `${hours}h ago`;
		if (days < 7) return `${days}d ago`;
		return new Date(date).toLocaleDateString();
	}
</script>

<li
	class={[styles.item, selected && styles.selected].filter(Boolean).join(' ')}
	role="option"
	aria-selected={selected}
	onmouseenter={() => (showActions = true)}
	onmouseleave={() => (showActions = false)}
>
	<button
		class={styles.button}
		onclick={onclick}
		aria-label="Select thread: {thread.title}"
		type="button"
	>
		<div class={styles.content}>
			<span class={styles.title}>{thread.title}</span>
			<span class={styles.meta}>
				<span class={styles.messageCount}>{thread.messageCount} messages</span>
				<span class={styles.dot}>·</span>
				<span class={styles.time}>{formatTime(thread.updatedAt)}</span>
			</span>
		</div>

		{#if thread.metadata.resolved}
			<span class={styles.resolvedBadge} aria-label="Resolved">
				<svg
					width="12"
					height="12"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<polyline points="20 6 9 17 4 12" />
				</svg>
			</span>
		{/if}
	</button>

	{#if showActions && (onresolve || ondelete)}
		<div class={styles.actions}>
			{#if onresolve}
				<button
					class={styles.action}
					onclick={(e) => {
						e.stopPropagation();
						onresolve();
					}}
					aria-label={thread.metadata.resolved ? 'Reopen thread' : 'Resolve thread'}
					title={thread.metadata.resolved ? 'Reopen' : 'Resolve'}
					type="button"
				>
					<svg
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						{#if thread.metadata.resolved}
							<polyline points="23 4 23 10 17 10" />
							<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
						{:else}
							<polyline points="20 6 9 17 4 12" />
						{/if}
					</svg>
				</button>
			{/if}

			{#if ondelete}
				<button
					class={[styles.action, styles.delete].filter(Boolean).join(' ')}
					onclick={(e) => {
						e.stopPropagation();
						ondelete();
					}}
					aria-label="Delete thread"
					title="Delete"
					type="button"
				>
					<svg
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="3 6 5 6 21 6" />
						<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
					</svg>
				</button>
			{/if}
		</div>
	{/if}
</li>
