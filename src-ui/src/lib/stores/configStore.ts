/**
 * Configuration State Management
 * Svelte store for managing LLM providers, models, and operation defaults.
 */

import { derived, get, writable } from 'svelte/store';
import type {
	ApiType,
	ManualModelEntry,
	Model,
	ModelsSource,
	OperationDefault,
	OperationType,
	Provider,
	SaveProviderRequest
} from '$lib/types/config';
import type { LoadingState } from '$lib/types/state';
import { findById } from '$lib/utils/collection';
import { addManualModel as apiAddManualModel,
	deleteApiKey as apiDeleteApiKey,
	fetchModels as apiFetchModels,
	getManualModels as apiGetManualModels,
	getModels as apiGetModels,
	getOperationDefaults,
	getProviders,
	removeManualModel as apiRemoveManualModel,
	removeProvider as apiRemoveProvider,
	retrieveApiKey as apiRetrieveApiKey,
	saveProvider as apiSaveProvider,
	setOperationDefault as apiSetOperationDefault,
	setProviderModelsSource as apiSetProviderModelsSource,
	storeApiKey as apiStoreApiKey,
	validateApiKey as apiValidateApiKey
} from '$lib/api/config';
import { createLogger } from '$lib/utils/logger';

const log = createLogger('ConfigStore');

// ============================================================================
// Types
// ============================================================================

export interface ValidationState {
	status: 'idle' | 'loading' | 'success' | 'error';
	message: string;
	icon: string;
}

export interface ConfigState {
	/** All providers */
	providers: Provider[];
	/** Providers loading state */
	providersLoading: LoadingState;
	/** Models by provider ID */
	modelsByProvider: Record<string, Model[]>;
	/** Operation defaults */
	operationDefaults: OperationDefault[];
	/** Defaults loading state */
	defaultsLoading: LoadingState;
	/** Global error */
	error: string | null;
}

// ============================================================================
// Store
// ============================================================================

function createConfigStore() {
	const initialState: ConfigState = {
		providers: [],
		providersLoading: 'idle',
		modelsByProvider: {},
		operationDefaults: [],
		defaultsLoading: 'idle',
		error: null
	};

	const { subscribe, set, update } = writable<ConfigState>(initialState);

	return {
		subscribe,

		/**
		 * Initializes the store by loading providers and defaults.
		 */
		async initialize(): Promise<void> {
			update((state) => ({ ...state, providersLoading: 'loading', error: null }));

			try {
				log.info('initializing...');
				const [providers, defaults] = await Promise.all([getProviders(), getOperationDefaults()]);

				log.info('loaded providers:', providers);
				log.info('loaded defaults:', defaults);
				log.info(
					'provider IDs:',
					providers.map((p) => ({ id: p.id, name: p.name, isConfigured: p.isConfigured }))
				);

				// Load models for all configured providers from the database
				const modelsByProvider: Record<string, Model[]> = {};
				const providerLoadFailures: string[] = [];
				for (const provider of providers) {
					if (provider.isConfigured) {
						try {
							const models = await apiGetModels(provider.id);
							if (models.length > 0) {
								modelsByProvider[provider.id] = models;
								log.info(`loaded ${models.length} models for ${provider.id}`);
							}
						} catch (error) {
							log.warn(`failed to load models for ${provider.id}:`, error);
							const detail = error instanceof Error ? error.message : String(error);
							providerLoadFailures.push(`${provider.name}: ${detail}`);
						}
					}
				}

				log.info('loaded models:', modelsByProvider);

				const aggregatedError =
					providerLoadFailures.length > 0
						? `Failed to load models for ${providerLoadFailures.length} provider(s): ${providerLoadFailures.join('; ')}`
						: null;

				update((state) => ({
					...state,
					providers,
					modelsByProvider,
					operationDefaults: defaults,
					providersLoading: 'success',
					defaultsLoading: 'success',
					error: aggregatedError
				}));
			} catch (error) {
				log.error('initialization failed:', error);
				update((state) => ({
					...state,
					providersLoading: 'error',
					error: error instanceof Error ? error.message : String(error)
				}));
			}
		},

		/**
		 * Gets all configured providers.
		 */
		get providers(): Provider[] {
			return get({ subscribe }).providers;
		},

		/**
		 * Gets the count of configured providers.
		 */
		get configuredCount(): number {
			return get({ subscribe }).providers.filter((p) => p.isConfigured).length;
		},

		/**
		 * Gets a provider by ID.
		 */
		getProvider(id: string): Provider | undefined {
			return findById(get({ subscribe }).providers, id);
		},

		/**
		 * Gets models for a provider.
		 */
		getModels(providerId: string): Model[] {
			return get({ subscribe }).modelsByProvider[providerId] || [];
		},

		/**
		 * Gets the default model for an operation.
		 */
		getDefault(operation: OperationType): string | null {
			const defaults = get({ subscribe }).operationDefaults;
			return defaults.find((d) => d.operation === operation)?.modelId ?? null;
		},

		/**
		 * Gets available models for an operation (from configured providers).
		 */
		getAvailableModels(operation: OperationType): Model[] {
			const state = get({ subscribe });
			const models: Model[] = [];

			for (const provider of state.providers) {
				if (!provider.isConfigured) continue;

				const providerModels = state.modelsByProvider[provider.id] || [];
				for (const model of providerModels) {
					const caps = model.capabilities;
					const supports =
						operation === 'analysis'
							? caps.analysis
							: operation === 'code'
								? caps.code
								: operation === 'vision'
									? caps.vision
									: operation === 'audio'
										? caps.audio
										: operation === 'tips'
											? caps.tips
											: caps.review;

					if (supports) {
						models.push(model);
					}
				}
			}

			return models;
		},

		/**
		 * Saves a provider (create or update).
		 */
		async saveProvider(request: SaveProviderRequest): Promise<Provider> {
			let provider: Provider;
			try {
				provider = await apiSaveProvider(request);
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				update((state) => ({ ...state, error: message }));
				throw error;
			}
			update((state) => {
				const existing = state.providers.findIndex((p) => p.id === provider.id);
				if (existing >= 0) {
					const providers = [...state.providers];
					providers[existing] = provider;
					return { ...state, providers, error: null };
				}
				return { ...state, providers: [...state.providers, provider], error: null };
			});
			return provider;
		},

		/**
		 * Removes a custom provider.
		 */
		async removeProvider(providerId: string): Promise<void> {
			try {
				await apiRemoveProvider(providerId);
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				update((state) => ({ ...state, error: message }));
				throw error;
			}
			update((state) => {
				const providers = state.providers.filter((p) => p.id !== providerId);
				const modelsByProvider = { ...state.modelsByProvider };
				delete modelsByProvider[providerId];

				// Clear any operation defaults that reference this provider
				const operationDefaults = state.operationDefaults.filter(
					(d) => !d.modelId?.startsWith(providerId)
				);

				return { ...state, providers, modelsByProvider, operationDefaults, error: null };
			});
		},

		/**
		 * Validates an API key.
		 */
		async validateApiKey(
			providerId: string,
			apiKey: string,
			apiType: ApiType,
			baseUrl?: string
		): Promise<{ valid: boolean; message: string }> {
			return apiValidateApiKey(providerId, apiKey, apiType, baseUrl ?? undefined);
		},

		/**
		 * Reloads providers from the backend to ensure synchronization.
		 */
		async reloadProviders(): Promise<void> {
			const providers = await getProviders();
			update((state) => ({ ...state, providers }));
		},

		/**
		 * Fetches and caches models for a provider.
		 * After successful fetch, reloads providers from backend to ensure consistency.
		 */
		async fetchModels(
			providerId: string,
			apiKey: string,
			apiType: ApiType,
			baseUrl?: string
		): Promise<string[]> {
			log.info('fetchModels called for provider:', providerId, 'apiType:', apiType);
			let result;
			try {
				result = await apiFetchModels(providerId, apiKey, apiType, baseUrl ?? undefined);
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				update((state) => ({ ...state, error: message }));
				throw error;
			}
			log.info('fetchModels result:', result);

			if (result.success) {
				log.info('Fetch successful, models:', result.models);

				// Convert model names to Model objects
				const models: Model[] = result.models.map((name) => ({
					id: `${providerId}-${name.toLowerCase().replace(/\s+/g, '-')}`,
					providerId,
					name,
					capabilities: {
						analysis: true,
						code: true,
						vision: name.toLowerCase().includes('vision') || name.toLowerCase().includes('gpt-4'),
						audio: name.toLowerCase().includes('whisper') || name.toLowerCase().includes('audio'),
						tips: true,
						review: true,
						streaming: true
					}
				}));

				// Update both models and provider status in a single update for atomicity
				update((state) => {
					const providers = state.providers.map((p) =>
						p.id === providerId ? { ...p, isConfigured: true } : p
					);
					return {
						...state,
						providers,
						modelsByProvider: {
							...state.modelsByProvider,
							[providerId]: models
						}
					};
				});

				log.info('Provider configured successfully, reloading providers from backend...');
				// Reload providers from backend to ensure synchronization
				await this.reloadProviders();
			} else {
				log.error('Fetch failed:', result.message);
				const message = result.message || 'Failed to fetch models';
				update((state) => ({ ...state, error: message }));
				throw new Error(message);
			}

			return result.models;
		},

		/**
		 * Stores an API key in the keychain.
		 */
		async storeApiKey(providerId: string, apiKey: string): Promise<void> {
			await apiStoreApiKey(providerId, apiKey);
		},

		/**
		 * Retrieves an API key from the keychain.
		 */
		async retrieveApiKey(providerId: string): Promise<string> {
			return apiRetrieveApiKey(providerId);
		},

		/**
		 * Deletes an API key from the keychain.
		 */
		async deleteApiKey(providerId: string): Promise<void> {
			await apiDeleteApiKey(providerId);
		},

		/**
		 * Sets the default model for an operation.
		 */
		async setOperationDefault(operation: OperationType, modelId: string): Promise<void> {
			try {
				await apiSetOperationDefault(operation, modelId);
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				update((state) => ({ ...state, error: message }));
				throw error;
			}
			update((state) => {
				const existing = state.operationDefaults.findIndex((d) => d.operation === operation);
				if (existing >= 0) {
					const operationDefaults = [...state.operationDefaults];
					operationDefaults[existing] = { operation, modelId };
					return { ...state, operationDefaults, error: null };
				}
				return {
					...state,
					operationDefaults: [...state.operationDefaults, { operation, modelId }],
					error: null
				};
			});
		},

		/**
		 * Gets manual models for a provider.
		 */
		async getManualModels(providerId: string): Promise<ManualModelEntry[]> {
			return apiGetManualModels(providerId);
		},

		/**
		 * Adds a manual model for a provider.
		 */
		async addManualModel(
			providerId: string,
			name: string,
			capabilities?: ManualModelEntry['capabilities']
		): Promise<void> {
			await apiAddManualModel(providerId, name, capabilities);
		},

		/**
		 * Removes a manual model for a provider.
		 */
		async removeManualModel(providerId: string, modelName: string): Promise<void> {
			await apiRemoveManualModel(providerId, modelName);
		},

		/**
		 * Sets the models source mode for a provider.
		 */
		async setProviderModelsSource(providerId: string, modelsSource: ModelsSource): Promise<void> {
			await apiSetProviderModelsSource(providerId, modelsSource);
			update((state) => {
				const providers = state.providers.map((p) =>
					p.id === providerId ? { ...p, modelsSource } : p
				);
				return { ...state, providers };
			});
		},

		/**
		 * Clears the error state.
		 */
		clearError(): void {
			update((state) => ({ ...state, error: null }));
		}
	};
}

export const configStore = createConfigStore();

// ============================================================================
// Derived Stores
// ============================================================================

/** Configured providers only */
export const configuredProviders = derived(configStore, ($config) =>
	$config.providers.filter((p) => p.isConfigured)
);

/** Whether any provider is configured */
export const hasConfiguredProvider = derived(configStore, ($config) =>
	$config.providers.some((p) => p.isConfigured)
);

/** Whether all required setup is complete */
export const isSetupComplete = derived([configStore], ([$config]) =>
	$config.providers.some((p) => p.isConfigured)
);

/** Available models grouped by operation - reactive */
export const availableModelsByOperation = derived(configStore, ($config) => {
	const result: Record<OperationType, Model[]> = {
		analysis: [],
		code: [],
		vision: [],
		audio: [],
		tips: [],
		review: []
	};

	for (const provider of $config.providers) {
		if (!provider.isConfigured) continue;

		const providerModels = $config.modelsByProvider[provider.id] || [];
		for (const model of providerModels) {
			const caps = model.capabilities;
			if (caps.analysis) result.analysis.push(model);
			if (caps.code) result.code.push(model);
			if (caps.vision) result.vision.push(model);
			if (caps.audio) result.audio.push(model);
			if (caps.tips) result.tips.push(model);
			if (caps.review) result.review.push(model);
		}
	}

	return result;
});
