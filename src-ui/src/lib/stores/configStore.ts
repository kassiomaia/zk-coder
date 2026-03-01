/**
 * Configuration State Management
 * Svelte store for managing LLM providers, models, and operation defaults.
 */

import { writable, derived, get } from 'svelte/store';
import type { Provider, Model, OperationDefault, ApiType, OperationType, SaveProviderRequest } from '$lib/types/config';
import {
	getProviders,
	saveProvider as apiSaveProvider,
	removeProvider as apiRemoveProvider,
	getModels as apiGetModels,
	validateApiKey as apiValidateApiKey,
	fetchModels as apiFetchModels,
	storeApiKey as apiStoreApiKey,
	retrieveApiKey as apiRetrieveApiKey,
	deleteApiKey as apiDeleteApiKey,
	getOperationDefaults,
	setOperationDefault as apiSetOperationDefault
} from '$lib/api/config';

// ============================================================================
// Types
// ============================================================================

export type LoadingState = 'idle' | 'loading' | 'success' | 'error';

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
				console.log('Config store: initializing...');
				const [providers, defaults] = await Promise.all([
					getProviders(),
					getOperationDefaults()
				]);

				console.log('Config store: loaded providers:', providers);
				console.log('Config store: loaded defaults:', defaults);
				console.log('Config store: provider IDs:', providers.map(p => ({ id: p.id, name: p.name, isConfigured: p.isConfigured })));

				// Load models for all configured providers from the database
				const modelsByProvider: Record<string, Model[]> = {};
				for (const provider of providers) {
					if (provider.isConfigured) {
						try {
							const models = await apiGetModels(provider.id);
							if (models.length > 0) {
								modelsByProvider[provider.id] = models;
								console.log(`Config store: loaded ${models.length} models for ${provider.id}`);
							}
						} catch (error) {
							console.warn(`Config store: failed to load models for ${provider.id}:`, error);
						}
					}
				}

				console.log('Config store: loaded models:', modelsByProvider);

				update((state) => ({
					...state,
					providers,
					modelsByProvider,
					operationDefaults: defaults,
					providersLoading: 'success',
					defaultsLoading: 'success'
				}));
			} catch (error) {
				console.error('Config store: initialization failed:', error);
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
			return get({ subscribe }).providers.find((p) => p.id === id);
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
					const supports = operation === 'analysis' ? caps.analysis :
						operation === 'code' ? caps.code :
						operation === 'vision' ? caps.vision :
						operation === 'audio' ? caps.audio :
						operation === 'tips' ? caps.tips :
						caps.review;

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
			const provider = await apiSaveProvider(request);
			update((state) => {
				const existing = state.providers.findIndex((p) => p.id === provider.id);
				if (existing >= 0) {
					const providers = [...state.providers];
					providers[existing] = provider;
					return { ...state, providers };
				}
				return { ...state, providers: [...state.providers, provider] };
			});
			return provider;
		},

		/**
		 * Removes a custom provider.
		 */
		async removeProvider(providerId: string): Promise<void> {
			await apiRemoveProvider(providerId);
			update((state) => {
				const providers = state.providers.filter((p) => p.id !== providerId);
				const modelsByProvider = { ...state.modelsByProvider };
				delete modelsByProvider[providerId];

				// Clear any operation defaults that reference this provider
				const operationDefaults = state.operationDefaults.filter(
					(d) => !d.modelId?.startsWith(providerId)
				);

				return { ...state, providers, modelsByProvider, operationDefaults };
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
			const result = await apiFetchModels(providerId, apiKey, apiType, baseUrl ?? undefined);

			if (result.success) {
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

				console.log('[ConfigStore] Provider configured successfully, reloading providers from backend...');
				// Reload providers from backend to ensure synchronization
				await this.reloadProviders();
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
			await apiSetOperationDefault(operation, modelId);
			update((state) => {
				const existing = state.operationDefaults.findIndex((d) => d.operation === operation);
				if (existing >= 0) {
					const operationDefaults = [...state.operationDefaults];
					operationDefaults[existing] = { operation, modelId };
					return { ...state, operationDefaults };
				}
				return {
					...state,
					operationDefaults: [...state.operationDefaults, { operation, modelId }]
				};
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
export const isSetupComplete = derived(
	[configStore],
	([$config]) => $config.providers.some((p) => p.isConfigured)
);

/** Available models grouped by operation - reactive */
export const availableModelsByOperation = derived(
	configStore,
	($config) => {
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
	}
);
