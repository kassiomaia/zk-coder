<script lang="ts">
	/**
	 * ConfigModal Component
	 * Modal for configuring an AI provider with API key.
	 */
	import styles from './ConfigModal.module.css';
	import Modal from '../shared/Modal.svelte';
	import Button from '../shared/Button.svelte';
	import Input from '../shared/Input.svelte';
	import { configStore, type ValidationState } from '$lib/stores/configStore';
	import type { Provider, ApiType, ModelsSource } from '$lib/types/config';
	import ProviderIcon from './ProviderIcon.svelte';
	import ManualModelList from '../ManualModelList.svelte';
	import { createLogger } from '$lib/utils/logger';

	// Create a secure logger for this component that redacts sensitive data
	const log = createLogger('ConfigModal');

	interface Props {
		open: boolean;
		providerId: string | null;
		onclose: () => void;
		onsaved?: () => void;
	}

	let { open, providerId, onclose, onsaved }: Props = $props();

	let apiKey = $state('');
	let validation: ValidationState = $state({
		status: 'idle',
		message: '',
		icon: ''
	});
	let isSaving = $state(false);
	let modelsSource = $state<ModelsSource>('auto');
	let isUpdatingModelsSource = $state(false);

	// Use reactive store subscription for proper reactivity
	let provider = $derived.by(() => {
		if (!providerId) return null;
		const state = $configStore;
		return state.providers.find((p) => p.id === providerId) ?? null;
	});

	async function handleValidate() {
		if (!provider) return;

		// Empty check
		if (!apiKey.trim()) {
			validation = {
				status: 'error',
				message: 'Please enter an API key',
				icon: 'alert-circle'
			};
			return;
		}

		validation = {
			status: 'loading',
			message: 'Validating...',
			icon: 'loader-2'
		};

		try {
			const result = await configStore.validateApiKey(
				provider.id,
				apiKey,
				provider.apiType,
				provider.baseUrl
			);

			if (result.valid) {
				validation = {
					status: 'success',
					message: 'API key validated successfully!',
					icon: 'check-circle'
				};
			} else {
				validation = {
					status: 'error',
					message: result.message || 'Invalid API key. Please check and try again.',
					icon: 'alert-circle'
				};
			}
		} catch (error) {
			validation = {
				status: 'error',
				message: error instanceof Error ? error.message : 'Validation failed',
				icon: 'alert-circle'
			};
		}
	}

	async function handleSave() {
		if (!provider || !apiKey.trim()) return;

		isSaving = true;
		validation = { status: 'idle', message: '', icon: '' };

		try {
			// Store API key in keychain
			log.info('Storing API key for provider:', provider.id);
			await configStore.storeApiKey(provider.id, apiKey);
			log.info('API key stored successfully');

			// Fetch models from the provider (this also marks provider as configured in backend)
			validation = {
				status: 'loading',
				message: 'Fetching available models...',
				icon: 'loader-2'
			};

			log.info('Fetching models for provider:', {
				id: provider.id,
				apiType: provider.apiType,
				baseUrl: provider.baseUrl
			});
			const models = await configStore.fetchModels(
				provider.id,
				apiKey,
				provider.apiType,
				provider.baseUrl
			);
			log.info('Models fetched:', { count: models.length });

			// Check if any models were fetched
			if (models.length === 0) {
				log.warn('No models were fetched for provider:', provider.id);
				validation = {
					status: 'error',
					message: `No models were found for ${provider.name}. Check the browser console for details and verify your API key is valid.`,
					icon: 'alert-circle'
				};
				return;
			}

			// The backend's fetch_models command already marks the provider as configured,
			// so we just need to update the local store state
			log.info('Provider configured successfully');

			// Call onsaved callback
			log.info('Calling onsaved callback');
			onsaved?.();

			log.info('Closing modal');
			onclose();
		} catch (error) {
			log.error('Save failed:', error);
			validation = {
				status: 'error',
				message: error instanceof Error ? error.message : 'Failed to save configuration',
				icon: 'alert-circle'
			};
		} finally {
			isSaving = false;
		}
	}

	function handleClose() {
		apiKey = '';
		validation = { status: 'idle', message: '', icon: '' };
		onclose();
	}

	async function handleSetModelsSource(source: ModelsSource) {
		if (!provider) return;

		isUpdatingModelsSource = true;
		try {
			await configStore.setProviderModelsSource(provider.id, source);
			modelsSource = source;
		} catch (error) {
			log.error('Failed to set models source:', error);
		} finally {
			isUpdatingModelsSource = false;
		}
	}

	$effect(() => {
		if (open && provider) {
			// Reset state when modal opens
			apiKey = '';
			validation = { status: 'idle', message: '', icon: '' };
			modelsSource = provider.modelsSource;

			// Load existing API key if available
			configStore
				.retrieveApiKey(provider.id)
				.then((key) => {
					if (key) apiKey = key;
				})
				.catch(() => {
					// Key doesn't exist, that's fine
				});
		}
	});
</script>

<Modal
	title={provider ? `Configure ${provider.name}` : 'Configure Provider'}
	{open}
	onclose={handleClose}
>
	{#snippet children()}
		{#if provider}
			<div class={styles.header}>
				<div class={styles.iconWrapper}>
					<ProviderIcon {provider} size={48} />
				</div>
				<div class={styles.details}>
					<div class={styles.name}>{provider.name}</div>
					<div class={styles.type}>
						{#if provider.apiType === 'openai'}
							OpenAI Compatible
						{:else if provider.apiType === 'google'}
							Google (Gemini) Compatible
						{:else}
							Anthropic Compatible
						{/if}
					</div>
				</div>
			</div>

			<div class={styles.form}>
				{#if provider.apiType === 'anthropic'}
					<Input
						name="apiKey"
						type="password"
						value={apiKey}
						onchange={(v) => (apiKey = v)}
						label="API Key"
						placeholder="sk-ant-..."
						helper="Get your API key from the provider's dashboard"
					/>
				{:else if provider.apiType === 'google'}
					<Input
						name="apiKey"
						type="password"
						value={apiKey}
						onchange={(v) => (apiKey = v)}
						label="API Key"
						placeholder="AIza..."
						helper="Get your API key from the provider's dashboard"
					/>
				{:else}
					<Input
						name="apiKey"
						type="password"
						value={apiKey}
						onchange={(v) => (apiKey = v)}
						label="API Key"
						placeholder="sk-..."
						helper="Get your API key from the provider's dashboard"
					/>
				{/if}
			</div>

			<!-- Model Source Configuration -->
			{#if provider.isConfigured}
				<div class={styles.modelSourceSection}>
					<h4 class={styles.modelSourceTitle}>Model Discovery Mode</h4>
					<p class={styles.modelSourceDesc}>Choose how models are discovered for this provider.</p>
					<div class={styles.modelSourceOptions}>
						<label
							class={[styles.modelSourceOption, modelsSource === 'auto' && styles.selected].join(
								' '
							)}
						>
							<input
								type="radio"
								name="modelsSource"
								value="auto"
								bind:group={modelsSource}
								onchange={() => handleSetModelsSource('auto')}
								disabled={isUpdatingModelsSource}
							/>
							<div class={styles.optionContent}>
								<span class={styles.optionName}>Auto</span>
								<span class={styles.optionDesc}>Automatically fetch models from provider API</span>
							</div>
						</label>
						<label
							class={[styles.modelSourceOption, modelsSource === 'manual' && styles.selected].join(
								' '
							)}
						>
							<input
								type="radio"
								name="modelsSource"
								value="manual"
								bind:group={modelsSource}
								onchange={() => handleSetModelsSource('manual')}
								disabled={isUpdatingModelsSource}
							/>
							<div class={styles.optionContent}>
								<span class={styles.optionName}>Manual Only</span>
								<span class={styles.optionDesc}>Use only manually configured models</span>
							</div>
						</label>
						<label
							class={[styles.modelSourceOption, modelsSource === 'hybrid' && styles.selected].join(
								' '
							)}
						>
							<input
								type="radio"
								name="modelsSource"
								value="hybrid"
								bind:group={modelsSource}
								onchange={() => handleSetModelsSource('hybrid')}
								disabled={isUpdatingModelsSource}
							/>
							<div class={styles.optionContent}>
								<span class={styles.optionName}>Hybrid</span>
								<span class={styles.optionDesc}>Combine API models with manual models</span>
							</div>
						</label>
					</div>

					{#if modelsSource !== 'auto' && providerId}
						<ManualModelList {providerId} />
					{/if}
				</div>
			{/if}

			{#if validation.status !== 'idle'}
				<div class={[styles.validation, styles[validation.status]].join(' ')}>
					{#if validation.icon === 'loader-2'}
						<svg
							class={styles.spinner}
							xmlns="http://www.w3.org/2000/svg"
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<line x1="12" y1="2" x2="12" y2="6" />
							<line x1="12" y1="18" x2="12" y2="22" />
							<line x1="4.93" y1="4.93" x2="7.76" y2="7.76" />
							<line x1="16.24" y1="16.24" x2="19.07" y2="19.07" />
							<line x1="2" y1="12" x2="6" y2="12" />
							<line x1="18" y1="12" x2="22" y2="12" />
							<line x1="4.93" y1="19.07" x2="7.76" y2="16.24" />
							<line x1="16.24" y1="7.76" x2="19.07" y2="4.93" />
						</svg>
					{:else if validation.icon === 'check-circle'}
						<svg
							xmlns="http://www.w3.org/2000/svg"
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
							<polyline points="22 4 12 14.01 9 11.01" />
						</svg>
					{:else if validation.icon === 'alert-circle'}
						<svg
							xmlns="http://www.w3.org/2000/svg"
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<circle cx="12" cy="12" r="10" />
							<line x1="12" y1="8" x2="12" y2="12" />
							<line x1="12" y1="16" x2="12.01" y2="16" />
						</svg>
					{/if}
					<span>{validation.message}</span>
				</div>
			{/if}

			<div class={styles.info}>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
					<path d="M7 11V7a5 5 0 0 1 10 0v4" />
				</svg>
				<span>Your API key is stored securely in your system keychain.</span>
			</div>
		{:else}
			<div class={styles.loading}>Loading provider...</div>
		{/if}
	{/snippet}

	{#snippet footer()}
		{#if provider}
			<div class={styles.footer}>
				<Button variant="secondary" onclick={handleClose} disabled={isSaving}>Cancel</Button>
				<Button variant="secondary" onclick={handleValidate} disabled={isSaving}>Validate</Button>
				<Button variant="primary" onclick={handleSave} disabled={!apiKey.trim() || isSaving}>
					{isSaving ? 'Saving...' : 'Save'}
				</Button>
			</div>
		{/if}
	{/snippet}
</Modal>

<style>
	/* Styles are in ConfigModal.module.css */
</style>
