<script lang="ts">
	import Spinner from './Spinner.svelte';

	interface Props {
		/** Loading message text */
		message?: string;
		/** Spinner size */
		size?: number;
		/** Additional class */
		class?: string;
	}

	let { message = 'Loading...', size = 16, class: className = '' }: Props = $props();
</script>

<div class="loading-state {className}">
	<Spinner {size} />
	<p>{message}</p>
</div>

<style>
	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: var(--space-12) var(--space-4);
		color: var(--color-text-muted);
		font-size: 11px;
		text-align: center;
		gap: var(--space-3);
	}

	.loading-state p {
		margin: 0;
		font-weight: 500;
		opacity: 0.8;
	}
</style>
