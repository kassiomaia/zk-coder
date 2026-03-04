<script lang="ts">
	/**
	 * ProviderCard Component
	 * Clickable card displaying provider info and status.
	 */
	import styles from './ProviderCard.module.css';
	import ProviderIcon from './ProviderIcon.svelte';
	import type { Provider } from '$lib/types/config';
	import { getProviderDefaultModels } from '$lib/types/config';

	interface Props {
		provider: Provider;
		onclick: () => void;
	}

	let { provider, onclick }: Props = $props();

	let cardClasses = $derived(
		[styles.card, provider.isConfigured && styles.configured].filter(Boolean).join(' ')
	);
</script>

<button class={cardClasses} {onclick} type="button" aria-label="Configure {provider.name}">
	<div class={styles.iconWrapper}>
		<ProviderIcon {provider} size={32} />
	</div>
	<div class={styles.name}>{provider.name}</div>
	<div class={styles.models}>
		{#if provider.isCustom}
			{provider.baseUrl ?? 'Custom Provider'}
		{:else}
			{getProviderDefaultModels(provider.id)}
		{/if}
	</div>
	<div class={styles.status}>
		{#if provider.isConfigured}
			<span class={[styles.dot, styles.configured].join(' ')}></span>
			<span class={styles.configuredText}>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="12"
					height="12"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<polyline points="20 6 9 17 4 12" />
				</svg>
				Configured
			</span>
		{:else}
			<span class={styles.dot}></span>
			<span>Click to configure</span>
		{/if}
	</div>
</button>

<style>
	/* Styles are in ProviderCard.module.css */
</style>
