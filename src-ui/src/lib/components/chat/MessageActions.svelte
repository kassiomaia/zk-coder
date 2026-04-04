<script lang="ts">
	/**
	 * MessageActions Component
	 * Hover action toolbar for message actions like copy, edit, delete, and feedback.
	 */
	import styles from './MessageActions.module.css';
	import IconButton from '$lib/components/shared/IconButton.svelte';
	import {
		CheckIcon,
		CopyIcon,
		DeleteIcon,
		EditIcon,
		RefreshIcon,
		ThumbsDownIcon,
		ThumbsUpIcon
	} from '$lib/icons';
	import type { MessageFeedback } from '$lib/types/message';

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
		/** Current feedback state (controlled) */
		feedback?: MessageFeedback;
		/** Handler for copy action */
		onCopy?: () => void;
		/** Handler for edit action */
		onEdit?: () => void;
		/** Handler for delete action */
		onDelete?: () => void;
		/** Handler for retry action */
		onRetry?: () => void;
		/** Handler for feedback change - receives new feedback or null to clear */
		onFeedback?: (feedback: MessageFeedback | null) => void;
	}

	let {
		showCopy = true,
		showEdit = true,
		showDelete = false,
		showRetry = false,
		showFeedback = true,
		feedback,
		onCopy,
		onEdit,
		onDelete,
		onRetry,
		onFeedback
	}: Props = $props();

	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | null = null;

	// Derive active states from feedback prop
	let thumbsUpActive = $derived(feedback === 'like');
	let thumbsDownActive = $derived(feedback === 'dislike');

	// Cleanup timer on component destroy
	$effect(() => {
		return () => {
			if (copyTimer) {
				clearTimeout(copyTimer);
				copyTimer = null;
			}
		};
	});

	async function handleCopy() {
		if (onCopy) {
			onCopy();
		} else {
			// Default copy behavior - copy message content
			try {
				// This would typically copy the message text passed as prop
				copied = true;
				// Clear any existing timer before setting a new one
				if (copyTimer) {
					clearTimeout(copyTimer);
				}
				copyTimer = setTimeout(() => {
					copied = false;
					copyTimer = null;
				}, 2000);
			} catch (err) {
				console.error('Failed to copy:', err);
			}
		}
	}

	function handleThumbsUp() {
		// Toggle: if already liked, clear; otherwise set to like
		const newFeedback: MessageFeedback | null = feedback === 'like' ? null : 'like';
		onFeedback?.(newFeedback);
	}

	function handleThumbsDown() {
		// Toggle: if already disliked, clear; otherwise set to dislike
		const newFeedback: MessageFeedback | null = feedback === 'dislike' ? null : 'dislike';
		onFeedback?.(newFeedback);
	}
</script>

<div class={styles.actions} role="toolbar" aria-label="Message actions">
	{#if showCopy}
		<IconButton
			label={copied ? 'Copied' : 'Copy message'}
			tooltip="Copy"
			tooltipPosition="bottom"
			variant="ghost"
			size="sm"
			class={styles.actionBtn}
			onclick={handleCopy}
		>
			{#snippet icon()}
				{#if copied}
					<CheckIcon class={styles.checkIcon} size={16} />
				{:else}
					<CopyIcon class={styles.copyIcon} size={16} />
				{/if}
			{/snippet}
		</IconButton>
	{/if}

	{#if showEdit}
		<IconButton
			label="Edit message"
			tooltip="Edit"
			tooltipPosition="bottom"
			variant="ghost"
			size="sm"
			class={styles.actionBtn}
			onclick={onEdit}
		>
			{#snippet icon()}
				<EditIcon size={16} />
			{/snippet}
		</IconButton>
	{/if}

	{#if showDelete}
		<IconButton
			label="Delete message"
			tooltip="Delete"
			tooltipPosition="bottom"
			variant="ghost"
			intent="danger"
			size="sm"
			class={styles.actionBtn}
			onclick={onDelete}
		>
			{#snippet icon()}
				<DeleteIcon size={16} />
			{/snippet}
		</IconButton>
	{/if}

	{#if showRetry}
		<IconButton
			label="Retry"
			tooltip="Retry"
			tooltipPosition="bottom"
			variant="ghost"
			size="sm"
			class={styles.actionBtn}
			onclick={onRetry}
		>
			{#snippet icon()}
				<RefreshIcon size={16} />
			{/snippet}
		</IconButton>
	{/if}

	{#if showFeedback}
		<div class={styles.feedbackGroup}>
			<IconButton
				label="Good response"
				tooltip="Good response"
				tooltipPosition="bottom"
				variant="ghost"
				intent="success"
				size="sm"
				class={styles.actionBtn}
				active={thumbsUpActive}
				onclick={handleThumbsUp}
			>
				{#snippet icon()}
					<ThumbsUpIcon class={styles.feedbackIcon} size={16} filled={thumbsUpActive} />
				{/snippet}
			</IconButton>
			<IconButton
				label="Bad response"
				tooltip="Bad response"
				tooltipPosition="bottom"
				variant="ghost"
				intent="warning"
				size="sm"
				class={styles.actionBtn}
				active={thumbsDownActive}
				onclick={handleThumbsDown}
			>
				{#snippet icon()}
					<ThumbsDownIcon class={styles.feedbackIcon} size={16} filled={thumbsDownActive} />
				{/snippet}
			</IconButton>
		</div>
	{/if}
</div>
