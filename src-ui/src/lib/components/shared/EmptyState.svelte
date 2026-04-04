<script lang="ts">
	/**
	 * EmptyState Component
	 * Simple inline empty state for lists and panels.
	 * Supports icon + text layout with proper hierarchy.
	 */
	import type { Snippet } from 'svelte';

	interface Props {
		/** Icon component to display */
		icon?: any;
		/** Message to display */
		message: string;
		/** Child content (alternative to message) */
		children?: Snippet;
		/** Additional CSS class for custom styling */
		class?: string;
	}

	let { icon, message, children, class: className = '' }: Props = $props();
</script>

<div class={className}>
	{#if icon}
		<span class="empty-icon"><icon size={14} strokeWidth={1.5}></icon></span>
	{/if}
	<span class="empty-text">{message}</span>
</div>

<style>
	div {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-4);
	}

	.empty-icon {
		color: var(--color-text-muted);
		opacity: 0.6;
		display: flex;
		align-items: center;
	}

	.empty-text {
		font-size: var(--text-xs);
		color: var(--color-text-tertiary);
	}
</style>
