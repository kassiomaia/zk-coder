<script lang="ts">
	/**
	 * MessageActions Component
	 * Hover action toolbar for message actions like copy, edit, delete, and feedback.
	 */
	import styles from './MessageActions.module.css';
	import IconButton from '$lib/components/shared/IconButton.svelte';

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
		<IconButton
			label={copied ? 'Copied' : 'Copy message'}
			tooltip="Copy"
			variant="ghost"
			size="sm"
			class={styles.actionBtn}
			onclick={handleCopy}
		>
			{#snippet icon()}
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
			{/snippet}
		</IconButton>
	{/if}

	{#if showEdit}
		<IconButton
			label="Edit message"
			tooltip="Edit"
			variant="ghost"
			size="sm"
			class={styles.actionBtn}
			onclick={onEdit}
		>
			{#snippet icon()}
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
			{/snippet}
		</IconButton>
	{/if}

	{#if showDelete}
		<IconButton
			label="Delete message"
			tooltip="Delete"
			variant="ghost"
			intent="danger"
			size="sm"
			class={styles.actionBtn}
			onclick={onDelete}
		>
			{#snippet icon()}
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
			{/snippet}
		</IconButton>
	{/if}

	{#if showRetry}
		<IconButton
			label="Retry"
			tooltip="Retry"
			variant="ghost"
			size="sm"
			class={styles.actionBtn}
			onclick={onRetry}
		>
			{#snippet icon()}
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
			{/snippet}
		</IconButton>
	{/if}

	{#if showFeedback}
		<div class={styles.feedbackGroup}>
			<IconButton
				label="Good response"
				tooltip="Good response"
				variant="ghost"
				intent="success"
				size="sm"
				class={styles.actionBtn}
				active={thumbsUpActive}
				onclick={handleThumbsUp}
			>
				{#snippet icon()}
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
				{/snippet}
			</IconButton>
			<IconButton
				label="Bad response"
				tooltip="Bad response"
				variant="ghost"
				intent="warning"
				size="sm"
				class={styles.actionBtn}
				active={thumbsDownActive}
				onclick={handleThumbsDown}
			>
				{#snippet icon()}
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
				{/snippet}
			</IconButton>
		</div>
	{/if}
</div>
