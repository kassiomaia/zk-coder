<script lang="ts">
	/**
	 * ProvidersModal Component
	 * Main modal for configuring AI providers.
	 */
	import styles from './ProvidersModal.module.css';
	import Modal from '../shared/Modal.svelte';
	import Button from '../shared/Button.svelte';
	import ProviderCard from './ProviderCard.svelte';
	import { configStore } from '$lib/stores/configStore';
	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('ProvidersModal');

	interface Props {
		open: boolean;
		onclose: () => void;
		oncontinue: () => void;
		onaddcustom: () => void;
		onconfigure: (providerId: string) => void;
	}

	let { open, onclose, oncontinue, onaddcustom, onconfigure }: Props = $props();

	let configuredCount = $derived($configStore.providers.filter((p) => p.isConfigured).length);
	let canContinue = $derived(configuredCount > 0);

	// Debug logging (only in development)
	$effect(() => {
		if (open) {
			log.debug('Modal opened, providers in store:', $configStore.providers);
			log.debug('Providers count:', $configStore.providers.length);
		}
	});
</script>

<Modal title="Configure Providers" {open} {onclose} size="lg" contentClass={styles.modalBody}>
	{#snippet children()}
		<div class={styles.header}>
			<p class={styles.description}>
				Configure one or more AI providers. At least one provider is required to continue.
			</p>

			{#if configuredCount > 0}
				<div class={styles.summaryBadge}>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.5"
					>
						<path d="M20 6L9 17l-5-5" />
					</svg>
					<span>{configuredCount} provider{configuredCount > 1 ? 's' : ''} configured</span>
				</div>
			{/if}
		</div>

		<div class={styles.list}>
			{#each $configStore.providers as provider (provider.id)}
				<ProviderCard {provider} onclick={() => onconfigure(provider.id)} />
			{/each}

			<!-- Add Custom Provider Card -->
			<button class={[styles.card, styles.addCard].join(' ')} onclick={onaddcustom} type="button">
				<div class={styles.addIcon}>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="20"
						height="20"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<circle cx="12" cy="12" r="10" />
						<line x1="12" y1="8" x2="12" y2="16" />
						<line x1="8" y1="12" x2="16" y2="12" />
					</svg>
				</div>
				<div class={styles.details}>
					<div class={styles.providerName}>Add Custom Provider</div>
					<div class={styles.providerModels}>OpenAI, Anthropic, or custom API</div>
				</div>
				<div class={styles.status}>
					<span class={styles.actionLabel}>Click to add</span>
				</div>
			</button>
		</div>
	{/snippet}

	{#snippet footer()}
		<div class={styles.footer}>
			<Button variant="secondary" onclick={onclose}>Cancel</Button>
			<div class={styles.footerRight}>
				<Button variant="ghost" onclick={onaddcustom}>Add Provider</Button>
				<Button variant="primary" disabled={!canContinue} onclick={oncontinue}>
					Continue to Settings
				</Button>
			</div>
		</div>
	{/snippet}
</Modal>
