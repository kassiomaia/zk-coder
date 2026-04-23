<script lang="ts">
	/**
	 * SettingsModal Component
	 * Full settings modal with providers and model configuration.
	 */
	import styles from './SettingsModal.module.css';
	import Modal from '../shared/Modal.svelte';
	import Button from '../shared/Button.svelte';
	import { availableModelsByOperation, configStore } from '$lib/stores/configStore';
	import { OPERATION_CONFIGS, type OperationType } from '$lib/types/config';
	import ModelCard from './ModelCard.svelte';
	import ProviderIcon from './ProviderIcon.svelte';
	import OperationIcon from './OperationIcon.svelte';
	import PromptTemplatesSection from './PromptTemplatesSection.svelte';
	import Select from '../shared/Select.svelte';
	import WebSearchModal from '../settings/web-search/WebSearchModal.svelte';
	import { AlertCircleIcon, BoxIcon, CheckIcon, SearchIcon, UserIcon } from '$lib/icons';

	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('SettingsModal');

	interface Props {
		open: boolean;
		onclose: () => void;
		onconfigure: (providerId: string) => void;
		onaddcustom: () => void;
		onremoveprovider: (providerId: string) => void;
		/** Canonicalized workspace path for the web-search policy editor. */
		workspacePath?: string;
		/** Tilde-compressed display path. */
		workspaceDisplayPath?: string;
	}

	let {
		open,
		onclose,
		onconfigure,
		onaddcustom,
		onremoveprovider,
		workspacePath = '',
		workspaceDisplayPath
	}: Props = $props();

	// Derived reactive map of selected model IDs by operation
	// Using $derived ensures Svelte 5 properly tracks $configStore.operationDefaults changes
	let webSearchModalOpen = $state(false);

	function openWebSearchModal() {
		webSearchModalOpen = true;
	}

	function closeWebSearchModal() {
		webSearchModalOpen = false;
	}

	let selectedModelIds = $derived.by(() => {
		const map = new Map<OperationType, string | null>();
		for (const d of $configStore.operationDefaults) {
			map.set(d.operation, d.modelId);
		}
		return map;
	});

	// Helper to get short name for selected model badge
	function getSelectedModelShortName(
		operation: OperationType,
		modelId: string | null
	): string | null {
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

	function handleRemoveProvider(providerId: string) {
		onremoveprovider(providerId);
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
					<div class={styles.sectionTitleGroup}>
						<div class={styles.sectionIcon}>
							<UserIcon size={16} />
						</div>
						<h3 class={styles.sectionTitle}>Providers</h3>
					</div>
					<Button variant="ghost" size="sm" onclick={onaddcustom}>Add Provider</Button>
				</header>

				<div class={styles.providersList}>
					{#each $configStore.providers.filter((p) => p.isConfigured) as provider (provider.id)}
						<div class={styles.providerItem}>
							<div class={styles.providerInfo}>
								<div class={styles.providerIconWrapper}>
									<ProviderIcon {provider} size={20} />
								</div>
								<div class={styles.providerDetails}>
									<div class={styles.providerName}>{provider.name}</div>
									<div class={styles.apiType}>{provider.apiType}</div>
								</div>
							</div>
							<div class={styles.providerActions}>
								<span class={styles.configuredBadge}>
									<CheckIcon size={14} />
									Connected
								</span>
								<Button variant="ghost" size="sm" onclick={() => onconfigure(provider.id)}>
									Edit
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

					{#if $configStore.providers.filter((p) => p.isConfigured).length === 0}
						<div class={styles.emptyState}>
							<span>No providers configured. Add a provider to get started.</span>
						</div>
					{/if}
				</div>
			</section>

			<!-- Tools Section -->
			<section class="modal-section {styles.section}">
				<header class={styles.sectionHeader}>
					<div class={styles.sectionTitleGroup}>
						<div class={styles.sectionIcon}>
							<SearchIcon size={16} />
						</div>
						<h3 class={styles.sectionTitle}>Tools</h3>
					</div>
				</header>

				<div class={styles.toolsList}>
					<div class={styles.toolItem}>
						<div class={styles.toolInfo}>
							<div class={styles.toolIconWrapper}>
								<SearchIcon size={18} />
							</div>
							<div class={styles.toolDetails}>
								<div class={styles.toolName}>Web Search</div>
								<div class={styles.toolMeta}>
									Let the assistant look up information on the web.
								</div>
							</div>
						</div>
						<div class={styles.toolActions}>
							<Button variant="ghost" size="sm" onclick={openWebSearchModal}>
								Configure
							</Button>
						</div>
					</div>
				</div>
			</section>

			<!-- Prompt Templates Section -->
			<PromptTemplatesSection />

			<!-- Models Section -->
			<section class="modal-section {styles.section}">
				<header class={styles.sectionHeader}>
					<div class={styles.sectionTitleGroup}>
						<div class={styles.sectionIcon}>
							<BoxIcon size={16} />
						</div>
						<h3 class={styles.sectionTitle}>Default Models</h3>
					</div>
				</header>

				{#each OPERATION_CONFIGS as op (op.id)}
					{@const availableModels = $availableModelsByOperation[op.id]}
					{@const selectedModelId = selectedModelIds.get(op.id)}
					{@const options = availableModels.map((m) => ({ value: m.id, label: m.name }))}

					<div class={styles.operationGroup}>
						<div class={styles.operationHeader}>
							<div class={styles.operationIcon}>
								<OperationIcon type={op.id} size={16} color={op.color} />
							</div>
							<div class={styles.operationInfo}>
								<span class={styles.operationName}>{op.name}</span>
								<span class={styles.operationDesc}>{op.description}</span>
							</div>
						</div>

						<div class={styles.modelSelectWrapper}>
							<Select
								value={selectedModelId ?? ''}
								{options}
								placeholder={availableModels.length === 0
									? 'No models available - configure provider'
									: 'Select a model'}
								disabled={availableModels.length === 0}
								onchange={(val) => handleSelectModel(op.id, val)}
							/>
						</div>
					</div>
				{/each}
			</section>
		</div>
	{/snippet}
</Modal>

<WebSearchModal
	open={webSearchModalOpen}
	onclose={closeWebSearchModal}
	{workspacePath}
	{workspaceDisplayPath}
/>
