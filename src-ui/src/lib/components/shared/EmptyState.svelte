<script lang="ts">
	/**
	 * EmptyState Component
	 * Simple text-based empty state for lists and panels.
	 * Accepts a class prop to inherit parent styling.
	 */
	import type { Snippet } from 'svelte';

	interface Props {
		/** Message to display */
		message?: string;
		/** Child content (alternative to message) */
		children?: Snippet;
		/** Additional CSS class for custom styling */
		class?: string;
	}

	let { message = '', children, class: className = '' }: Props = $props();
</script>

<div class={className}>
	{#if children}
		{@render children()}
	{:else}
		<p>{message}</p>
	{/if}
</div>
