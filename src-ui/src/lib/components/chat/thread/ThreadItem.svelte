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
		/** Archive handler */
		onarchive?: () => void;
	}

	let { thread, selected = false, onclick, onresolve, ondelete, onarchive }: Props = $props();

	let showActions = $state(false);

	function formatTime(date: Date | string): string {
		// Handle both Date objects and ISO strings from backend
		const d = date instanceof Date ? date : new Date(date);

		// Check for invalid date
		if (isNaN(d.getTime())) {
			return '';
		}

		const now = new Date();
		const diff = now.getTime() - d.getTime();
		const minutes = Math.floor(diff / 60000);
		const hours = Math.floor(diff / 3600000);
		const days = Math.floor(diff / 86400000);

		if (minutes < 1) return 'Just now';
		if (minutes < 60) return `${minutes}m ago`;
		if (hours < 24) return `${hours}h ago`;
		if (days < 7) return `${days}d ago`;
		return d.toLocaleDateString();
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
			<span class={styles.title}>
				{thread.title}
				{#if thread.metadata.resolved}
					<span class={styles.resolvedIcon} title="Resolved">
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
							<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
							<polyline points="22 4 12 14.01 9 11.01" />
						</svg>
					</span>
				{/if}
			</span>
			<span class={styles.meta}>
				<span class={styles.messageCount}>{thread.messageCount} messages</span>
				<span class={styles.dot}>·</span>
				<span class={styles.time}>{formatTime(thread.updatedAt)}</span>
			</span>
		</div>
	</button>

	{#if showActions && (onresolve || ondelete || onarchive)}
		<div class={styles.actions}>
			{#if onresolve}
				<button
					class={[styles.action, thread.metadata.resolved && styles.reopen].filter(Boolean).join(' ')}
					onclick={(e) => {
						e.stopPropagation();
						onresolve();
					}}
					aria-label={thread.metadata.resolved ? 'Reopen thread' : 'Resolve thread'}
					title={thread.metadata.resolved ? 'Reopen' : 'Resolve'}
					type="button"
				>
					{#if thread.metadata.resolved}
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
							<path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
							<path d="M3 3v5h5" />
							<path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
							<path d="M16 21h5v-5" />
						</svg>
					{:else}
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
							<polyline points="20 6 9 17 4 12" />
						</svg>
					{/if}
				</button>
			{/if}

			{#if onarchive}
				<button
					class={[styles.action, styles.archive].filter(Boolean).join(' ')}
					onclick={(e) => {
						e.stopPropagation();
						onarchive();
					}}
					aria-label="Archive thread"
					title="Archive"
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
						<polyline points="21 8 21 21 3 21 3 8" />
						<rect x="1" y="3" width="22" height="5" />
						<line x1="10" y1="12" x2="14" y2="12" />
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
						width="16"
						height="16"
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
