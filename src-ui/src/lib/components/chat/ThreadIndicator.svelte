<script lang="ts">
	/**
	 * ThreadIndicator Component
	 * Shows a badge indicating which thread a message belongs to.
	 * Clickable to navigate to the thread.
	 */
	import styles from './ThreadIndicator.module.css';
	import type { Thread } from '$lib/types/thread';

	interface Props {
		/** The thread the message belongs to */
		thread: Thread;
		/** Handler to navigate to the thread */
		onclick?: () => void;
		/** Compact mode for inline display */
		compact?: boolean;
		/** Whether the thread is resolved (shows a check badge + muted styling) */
		resolved?: boolean;
	}

	let { thread, onclick, compact = false, resolved = false }: Props = $props();

	function handleClick(e: MouseEvent) {
		e.stopPropagation();
		onclick?.();
	}
</script>

<button
	class={[styles.indicator, compact && styles.compact, resolved && styles.resolved]
		.filter(Boolean)
		.join(' ')}
	onclick={handleClick}
	aria-label={resolved
		? `View resolved thread: ${thread.title}`
		: `View thread: ${thread.title}`}
	type="button"
>
	{#if resolved}
		<svg
			class={styles.icon}
			width="14"
			height="14"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<circle cx="12" cy="12" r="10" />
			<path d="M9 12l2 2 4-4" />
		</svg>
	{:else}
		<svg
			class={styles.icon}
			width="14"
			height="14"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
		</svg>
	{/if}
	{#if !compact}
		<span class={styles.title}>{thread.title}</span>
	{/if}
</button>
