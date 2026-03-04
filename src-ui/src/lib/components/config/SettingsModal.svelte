<script lang="ts">
	/**
	 * SettingsModal Component
	 * Full settings modal with providers and model configuration.
	 */
	import styles from './SettingsModal.module.css';
	import Modal from '../shared/Modal.svelte';
	import Button from '../shared/Button.svelte';
	import { configStore, availableModelsByOperation } from '$lib/stores/configStore';
	import { OPERATION_CONFIGS, type OperationType } from '$lib/types/config';
	import ModelCard from './ModelCard.svelte';
	import ProviderIcon from './ProviderIcon.svelte';
	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('SettingsModal');

	interface Props {
		open: boolean;
		onclose: () => void;
		onconfigure: (providerId: string) => void;
		onaddcustom: () => void;
		onremoveprovider?: (providerId: string) => void;
	}

	let { open, onclose, onconfigure, onaddcustom, onremoveprovider }: Props = $props();

	// Helper to get selected model ID from store (reactive via $configStore)
	function getSelectedModelId(operation: OperationType): string | null {
		const default_ = $configStore.operationDefaults.find((d) => d.operation === operation);
		return default_?.modelId ?? null;
	}

	// Helper to get short name for selected model badge
	function getSelectedModelShortName(operation: OperationType): string | null {
		const modelId = getSelectedModelId(operation);
		if (!modelId) return null;

		// Find the model in available models
		const models = $availableModelsByOperation[operation];
		const model = models.find((m) => m.id === modelId);
		if (!model) return null;

		// Return shortened name (e.g., "Claude Opus" -> "Opus")
		const shortNames: Record<string, string> = {
			'claude opus': 'Opus',
			'claude sonnet': 'Sonnet',
			'claude haiku': 'Haiku',
			'gpt-4 turbo': 'GPT-4T',
			'gpt-4': 'GPT-4',
			'gemini ultra': 'GemU',
			'gemini pro': 'GemP'
		};
		return shortNames[model.name.toLowerCase()] ?? model.name.split(' ').pop();
	}

	async function handleSelectModel(operation: OperationType, modelId: string) {
		try {
			await configStore.setOperationDefault(operation, modelId);
		} catch (error) {
			log.error('Failed to set operation default:', error);
		}
	}

	async function handleRemoveProvider(providerId: string) {
		if (onremoveprovider) {
			onremoveprovider(providerId);
		} else {
			try {
				await configStore.removeProvider(providerId);
			} catch (error) {
				log.error('Failed to remove provider:', error);
			}
		}
	}

	// Debug logging when modal opens (only in development)
	$effect(() => {
		if (open) {
			log.debug('Modal opened');
			log.debug('Providers in store:', $configStore.providers);
			log.debug('Providers count:', $configStore.providers.length);
			log.debug(
				'Provider IDs:',
				$configStore.providers.map((p) => p.id)
			);
		}
	});
</script>

<Modal title="Settings" {open} size="xl" {onclose}>
	{#snippet children()}
		<div class={styles.sections}>
			<!-- Providers Section -->
			<section class="modal-section {styles.section}">
				<header class={styles.sectionHeader}>
					<div class={styles.sectionIconWrapper}>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							width="18"
							height="18"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
							<circle cx="9" cy="7" r="4" />
							<path d="M23 21v-2a4 4 0 0 0-3-3.87" />
							<path d="M16 3.13a4 4 0 0 1 0 7.75" />
						</svg>
					</div>
					<h3 class={styles.sectionTitle}>Providers</h3>
					<Button variant="primary" size="sm" onclick={onaddcustom}>Add Provider</Button>
				</header>

				<div class={styles.providersList}>
					{#each $configStore.providers as provider (provider.id)}
						<div
							class={[styles.providerItem, provider.isConfigured && styles.configured].join(' ')}
						>
							<div class={styles.providerInfo}>
								<div class={styles.providerIconWrapper}>
									<ProviderIcon {provider} size={24} />
								</div>
								<div class={styles.providerDetails}>
									<div class={styles.providerName}>{provider.name}</div>
									<div class={styles.providerMeta}>
										<span class={styles.apiType}>{provider.apiType}</span>
									</div>
								</div>
							</div>
							<div class={styles.providerActions}>
								{#if provider.isConfigured}
									<span class={styles.configuredBadge}>
										<svg
											xmlns="http://www.w3.org/2000/svg"
											width="14"
											height="14"
											viewBox="0 0 24 24"
											fill="none"
											stroke="currentColor"
											stroke-width="2"
										>
											<polyline points="20 6 9 17 4 12" />
										</svg>
										Connected
									</span>
								{/if}
								<Button variant="ghost" size="sm" onclick={() => onconfigure(provider.id)}>
									{provider.isConfigured ? 'Edit' : 'Configure'}
								</Button>
								{#if provider.isCustom}
									<Button
										variant="ghost"
										size="sm"
										onclick={() => handleRemoveProvider(provider.id)}
									>
										Remove
									</Button>
								{/if}
							</div>
						</div>
					{/each}

					{#if $configStore.providers.length === 0}
						<div class={styles.emptyState}>
							<svg
								xmlns="http://www.w3.org/2000/svg"
								width="24"
								height="24"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								<circle cx="12" cy="12" r="10" />
								<line x1="12" y1="8" x2="12" y2="12" />
								<line x1="12" y1="16" x2="12.01" y2="16" />
							</svg>
							<span>No providers configured. Add a provider to get started.</span>
						</div>
					{/if}
				</div>
			</section>

			<!-- Models Section -->
			<section class="modal-section {styles.section}">
				<header class={styles.sectionHeader}>
					<div class={styles.sectionIconWrapper}>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							width="18"
							height="18"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path
								d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
							/>
							<polyline points="3.27 6.96 12 12.01 20.73 6.96" />
							<line x1="12" y1="22.08" x2="12" y2="12" />
						</svg>
					</div>
					<h3 class={styles.sectionTitle}>Default Models by Operation</h3>
				</header>

				{#each OPERATION_CONFIGS as op (op.id)}
					{@const availableModels = $availableModelsByOperation[op.id]}
					{@const selectedModelId = getSelectedModelId(op.id)}
					<div class={styles.operationGroup}>
						<header class={styles.operationHeader}>
							<div class={styles.operationIcon} style:color={op.color}>
								{#if op.icon === 'search'}
									<svg
										xmlns="http://www.w3.org/2000/svg"
										width="18"
										height="18"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
									>
										<circle cx="11" cy="11" r="8" />
										<line x1="21" y1="21" x2="16.65" y2="16.65" />
									</svg>
								{:else if op.icon === 'code'}
									<svg
										xmlns="http://www.w3.org/2000/svg"
										width="18"
										height="18"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
									>
										<polyline points="16 18 22 12 16 6" />
										<polyline points="8 6 2 12 8 18" />
									</svg>
								{:else if op.icon === 'eye'}
									<svg
										xmlns="http://www.w3.org/2000/svg"
										width="18"
										height="18"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
									>
										<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
										<circle cx="12" cy="12" r="3" />
									</svg>
								{:else if op.icon === 'mic'}
									<svg
										xmlns="http://www.w3.org/2000/svg"
										width="18"
										height="18"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
									>
										<path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
										<path d="M19 10v2a7 7 0 0 1-14 0v-2" />
										<line x1="12" y1="19" x2="12" y2="23" />
										<line x1="8" y1="23" x2="16" y2="23" />
									</svg>
								{:else if op.icon === 'lightbulb'}
									<svg
										xmlns="http://www.w3.org/2000/svg"
										width="18"
										height="18"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
									>
										<path d="M9 18h6" />
										<path d="M10 22h4" />
										<path
											d="M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0 0 18 8 6 6 0 0 0 6 8c0 1 .23 2.23 1.5 3.5A4.61 4.61 0 0 1 8.91 14"
										/>
									</svg>
								{:else if op.icon === 'check-circle'}
									<svg
										xmlns="http://www.w3.org/2000/svg"
										width="18"
										height="18"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
									>
										<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
										<polyline points="22 4 12 14.01 9 11.01" />
									</svg>
								{/if}
							</div>
							<div class={styles.operationInfo}>
								<span class={styles.operationName}>{op.name}</span>
								<span class={styles.operationDesc}>{op.description}</span>
							</div>
							<!-- Selected model badge -->
							{#if selectedModelId}
								<div class={styles.operationSelected}>
									<span class={styles.selectedLabel}>{getSelectedModelShortName(op.id)}</span>
								</div>
							{/if}
						</header>

						<div class={styles.modelGrid}>
							{#each availableModels as model (model.id)}
								<ModelCard
									{model}
									selected={selectedModelId === model.id}
									onclick={() => handleSelectModel(op.id, model.id)}
								/>
							{/each}

							{#if availableModels.length === 0}
								<div class={styles.emptyModels}>
									<span>No models available - configure a provider first</span>
								</div>
							{/if}
						</div>
					</div>
				{/each}
			</section>
		</div>
	{/snippet}
</Modal>

<style>
	/* Styles are in SettingsModal.module.css */
</style>
