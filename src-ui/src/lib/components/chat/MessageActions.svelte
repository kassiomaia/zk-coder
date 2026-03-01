<script lang="ts">
	/**
	 * MessageActions Component
	 * Hover action toolbar for message actions like copy, edit, delete, and feedback.
	 */
	import styles from './MessageActions.module.css';

	interface Props {
		/** Whether to show copy action */
		showCopy?: boolean;
		/** Whether to show edit action */
		showEdit?: boolean;
		/** Whether to show delete action */
		showDelete?: boolean;
		/** Whether to show retry action */
		showRetry?: boolean;
		/** Whether to show feedback actions (thumbs up/down) */
		showFeedback?: boolean;
		/** Handler for copy action */
		onCopy?: () => void;
		/** Handler for edit action */
		onEdit?: () => void;
		/** Handler for delete action */
		onDelete?: () => void;
		/** Handler for retry action */
		onRetry?: () => void;
		/** Handler for positive feedback */
		onThumbsUp?: () => void;
		/** Handler for negative feedback */
		onThumbsDown?: () => void;
	}

	let {
		showCopy = true,
		showEdit = true,
		showDelete = false,
		showRetry = false,
		showFeedback = true,
		onCopy,
		onEdit,
		onDelete,
		onRetry,
		onThumbsUp,
		onThumbsDown
	}: Props = $props();

	let copied = $state(false);
	let thumbsUpActive = $state(false);
	let thumbsDownActive = $state(false);

	async function handleCopy() {
		if (onCopy) {
			onCopy();
		} else {
			// Default copy behavior - copy message content
			try {
				// This would typically copy the message text passed as prop
				copied = true;
				setTimeout(() => {
					copied = false;
				}, 2000);
			} catch (err) {
				console.error('Failed to copy:', err);
			}
		}
	}

	function handleThumbsUp() {
		thumbsUpActive = !thumbsUpActive;
		if (thumbsUpActive) {
			thumbsDownActive = false;
		}
		onThumbsUp?.();
	}

	function handleThumbsDown() {
		thumbsDownActive = !thumbsDownActive;
		if (thumbsDownActive) {
			thumbsUpActive = false;
		}
		onThumbsDown?.();
	}
</script>

<div class={styles.actions} role="toolbar" aria-label="Message actions">
	{#if showCopy}
		<button
			class={[styles.action, copied && styles.copied].filter(Boolean).join(' ')}
			onclick={handleCopy}
			aria-label={copied ? 'Copied' : 'Copy message'}
			type="button"
			title="Copy"
		>
			{#if copied}
				<svg
					class={styles.checkIcon}
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
			{:else}
				<svg
					class={styles.copyIcon}
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
					<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
				</svg>
			{/if}
		</button>
	{/if}

	{#if showEdit}
		<button
			class={styles.action}
			onclick={onEdit}
			aria-label="Edit message"
			type="button"
			title="Edit"
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
				<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
				<path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
			</svg>
		</button>
	{/if}

	{#if showDelete}
		<button
			class={styles.action}
			onclick={onDelete}
			aria-label="Delete message"
			type="button"
			title="Delete"
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

	{#if showRetry}
		<button class={styles.action} onclick={onRetry} aria-label="Retry" type="button" title="Retry">
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
				<polyline points="23 4 23 10 17 10" />
				<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
			</svg>
		</button>
	{/if}

	{#if showFeedback}
		<div class={styles.feedbackGroup}>
			<button
				class={[styles.action, thumbsUpActive && styles.active].filter(Boolean).join(' ')}
				onclick={handleThumbsUp}
				aria-label="Good response"
				aria-pressed={thumbsUpActive}
				type="button"
				title="Good response"
			>
				<svg
					class={styles.feedbackIcon}
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill={thumbsUpActive ? 'currentColor' : 'none'}
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path
						d="M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3zM7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3"
					/>
				</svg>
			</button>
			<button
				class={[styles.action, thumbsDownActive && styles.active].filter(Boolean).join(' ')}
				onclick={handleThumbsDown}
				aria-label="Bad response"
				aria-pressed={thumbsDownActive}
				type="button"
				title="Bad response"
			>
				<svg
					class={styles.feedbackIcon}
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill={thumbsDownActive ? 'currentColor' : 'none'}
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path
						d="M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3zm7-13h2.67A2.31 2.31 0 0 1 22 4v7a2.31 2.31 0 0 1-2.33 2H17"
					/>
				</svg>
			</button>
		</div>
	{/if}
</div>
