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
	import OperationIcon from './OperationIcon.svelte';
	import { UserIcon, CheckIcon, AlertCircleIcon, BoxIcon } from '$lib/icons';
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
						<UserIcon size={18} />
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
										<CheckIcon size={14} />
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
							<AlertCircleIcon size={24} />
							<span>No providers configured. Add a provider to get started.</span>
						</div>
					{/if}
				</div>
			</section>

			<!-- Models Section -->
			<section class="modal-section {styles.section}">
				<header class={styles.sectionHeader}>
					<div class={styles.sectionIconWrapper}>
						<BoxIcon size={18} />
					</div>
					<h3 class={styles.sectionTitle}>Default Models by Operation</h3>
				</header>

				{#each OPERATION_CONFIGS as op (op.id)}
					{@const availableModels = $availableModelsByOperation[op.id]}
					{@const selectedModelId = getSelectedModelId(op.id)}
					<div class={styles.operationGroup}>
						<header class={styles.operationHeader}>
							<div class={styles.operationIcon} style:color={op.color}>
								<OperationIcon type={op.id} size={18} color={op.color} />
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
