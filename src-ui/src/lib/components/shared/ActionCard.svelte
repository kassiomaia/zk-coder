<script lang="ts">
	/**
	 * ActionCard Component
	 * Reusable card with icon, title, description, and action buttons.
	 * Used for ApprovalMessage, RecoveryMessage, and similar in-chat action cards.
	 */
	import type { Snippet } from 'svelte';
	import { fade } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import styles from './ActionCard.module.css';

	interface Props {
		/** Icon snippet (required) */
		icon: Snippet;
		/** Primary title text */
		title: string;
		/** Secondary description text */
		description?: string;
		/** Additional content (e.g., highlight spans) */
		titleContent?: Snippet;
		/** Action buttons snippet (shown when not resolved) */
		actions?: Snippet;
		/** Whether the card is in resolved state */
		resolved?: boolean;
		/** Resolved state content snippet */
		resolvedContent?: Snippet;
		/** Additional CSS class */
		class?: string;
	}

	let {
		icon,
		title,
		description = '',
		titleContent,
		actions,
		resolved = false,
		resolvedContent,
		class: className = ''
	}: Props = $props();
</script>

<div
	class={[styles.container, className].filter(Boolean).join(' ')}
	in:fade={{ duration: 200, easing: cubicOut }}
>
	<div class={styles.contentRow}>
		<div class={styles.iconWrapper}>
			{@render icon()}
		</div>
		<div class={styles.textWrapper}>
			<div class={styles.primaryText}>
				{title}
				{#if titleContent}
					{@render titleContent()}
				{/if}
			</div>
			{#if description}
				<div class={styles.secondaryText}>{description}</div>
			{/if}
		</div>
		<div class={styles.actionZone}>
			{#if !resolved && actions}
				{@render actions()}
			{:else if resolved && resolvedContent}
				<div class={styles.resolvedStatus} in:fade>
					{@render resolvedContent()}
				</div>
			{/if}
		</div>
	</div>
</div>
