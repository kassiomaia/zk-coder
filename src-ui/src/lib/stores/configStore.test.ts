/**
 * Tests for configStore
 * Comprehensive test coverage for all configuration store functionality.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import type { Model, OperationDefault, OperationType, Provider } from '$lib/types/config';
import {
	addManualModel,
	deleteApiKey,
	fetchModels,
	getManualModels,
	getModels,
	getOperationDefaults,
	getProviders,
	removeManualModel,
	removeProvider,
	retrieveApiKey,
	saveProvider,
	setOperationDefault,
	setProviderModelsSource,
	storeApiKey,
	validateApiKey
} from '$lib/api/config';

// Now import after mocking
import { configStore } from './configStore';

// Create mock functions that will be used in vi.mock
const mockInvoke = vi.fn();

// Mock Tauri APIs before importing the store
vi.mock('@tauri-apps/api/core', () => ({
	invoke: mockInvoke
}));

// Mock the config API functions
vi.mock('$lib/api/config', () => ({
	getProviders: vi.fn(),
	saveProvider: vi.fn(),
	removeProvider: vi.fn(),
	getModels: vi.fn(),
	validateApiKey: vi.fn(),
	fetchModels: vi.fn(),
	storeApiKey: vi.fn(),
	retrieveApiKey: vi.fn(),
	deleteApiKey: vi.fn(),
	getOperationDefaults: vi.fn(),
	setOperationDefault: vi.fn(),
	getManualModels: vi.fn(),
	addManualModel: vi.fn(),
	removeManualModel: vi.fn(),
	setProviderModelsSource: vi.fn()
}));

// Helper function to create mock provider
const createMockProvider = (id: string, name: string, isConfigured = false): Provider => ({
	id,
	name,
	icon: id.substring(0, 2).toUpperCase(),
	isCustom: false,
	apiType: 'openai',
	isConfigured,
	modelsSource: 'auto'
});

// Helper function to create mock model
const createMockModel = (id: string, providerId: string, name: string): Model => ({
	id,
	providerId,
	name,
	capabilities: {
		analysis: true,
		code: true,
		vision: false,
		audio: false,
		tips: true,
		review: true,
		streaming: true
	}
});

// Helper function to create mock operation default
const createMockOperationDefault = (
	operation: OperationType,
	modelId: string | null
): OperationDefault => ({
	operation,
	modelId
});

describe('configStore', () => {
	beforeEach(() => {
		vi.clearAllMocks();

		// Reset store state
		const state = get(configStore);
		state.providers = [];
		state.providersLoading = 'idle';
		state.modelsByProvider = {};
		state.operationDefaults = [];
		state.defaultsLoading = 'idle';
		state.error = null;

		// Setup default mock implementations
		vi.mocked(getProviders).mockResolvedValue([]);
		vi.mocked(getOperationDefaults).mockResolvedValue([]);
		vi.mocked(getModels).mockResolvedValue([]);
		vi.mocked(saveProvider).mockResolvedValue(createMockProvider('prov-1', 'Test'));
		vi.mocked(validateApiKey).mockResolvedValue({ valid: true, message: 'Valid' });
		vi.mocked(fetchModels).mockResolvedValue({ success: true, models: ['model-1', 'model-2'] });
		vi.mocked(storeApiKey).mockResolvedValue(undefined);
		vi.mocked(retrieveApiKey).mockResolvedValue('test-api-key');
		vi.mocked(deleteApiKey).mockResolvedValue(undefined);
		vi.mocked(removeProvider).mockResolvedValue(undefined);
		vi.mocked(setOperationDefault).mockResolvedValue(undefined);
		vi.mocked(getManualModels).mockResolvedValue([]);
		vi.mocked(addManualModel).mockResolvedValue(undefined);
		vi.mocked(removeManualModel).mockResolvedValue(undefined);
		vi.mocked(setProviderModelsSource).mockResolvedValue(undefined);

		// Mock console methods to avoid noise in tests
		vi.spyOn(console, 'log').mockImplementation(() => {});
		vi.spyOn(console, 'warn').mockImplementation(() => {});
		vi.spyOn(console, 'error').mockImplementation(() => {});
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('Store Initialization & State', () => {
		it('has correct initial state', () => {
			const state = get(configStore);
			expect(state.providers).toEqual([]);
			expect(state.providersLoading).toBe('idle');
			expect(state.modelsByProvider).toEqual({});
			expect(state.operationDefaults).toEqual([]);
			expect(state.defaultsLoading).toBe('idle');
			expect(state.error).toBeNull();
		});

		it('initialize loads providers and defaults', async () => {
			const mockProviders = [
				createMockProvider('prov-1', 'Provider 1', true),
				createMockProvider('prov-2', 'Provider 2', false)
			];
			const mockDefaults = [
				createMockOperationDefault('code', 'prov-1-model-1'),
				createMockOperationDefault('vision', null)
			];

			vi.mocked(getProviders).mockResolvedValue(mockProviders);
			vi.mocked(getOperationDefaults).mockResolvedValue(mockDefaults);

			await configStore.initialize();

			expect(getProviders).toHaveBeenCalled();
			expect(getOperationDefaults).toHaveBeenCalled();

			const state = get(configStore);
			expect(state.providers).toEqual(mockProviders);
			expect(state.operationDefaults).toEqual(mockDefaults);
			expect(state.providersLoading).toBe('success');
			expect(state.defaultsLoading).toBe('success');
		});

		it('initialize loads models for configured providers', async () => {
			const mockProviders = [
				createMockProvider('prov-1', 'Provider 1', true),
				createMockProvider('prov-2', 'Provider 2', false)
			];
			const mockModels = [
				createMockModel('prov-1-model-1', 'prov-1', 'Model 1'),
				createMockModel('prov-1-model-2', 'prov-1', 'Model 2')
			];

			vi.mocked(getProviders).mockResolvedValue(mockProviders);
			vi.mocked(getOperationDefaults).mockResolvedValue([]);
			vi.mocked(getModels).mockResolvedValue(mockModels);

			await configStore.initialize();

			expect(getModels).toHaveBeenCalledWith('prov-1');
			expect(getModels).not.toHaveBeenCalledWith('prov-2');

			const state = get(configStore);
			expect(state.modelsByProvider['prov-1']).toEqual(mockModels);
			expect(state.modelsByProvider['prov-2']).toBeUndefined();
		});

		it('initialize handles getModels errors gracefully', async () => {
			const mockProviders = [createMockProvider('prov-1', 'Provider 1', true)];

			vi.mocked(getProviders).mockResolvedValue(mockProviders);
			vi.mocked(getOperationDefaults).mockResolvedValue([]);
			vi.mocked(getModels).mockRejectedValue(new Error('Failed to load models'));

			await configStore.initialize();

			// Should not throw, should continue with other providers
			const state = get(configStore);
			expect(state.providersLoading).toBe('success');
		});

		it('initialize aggregates per-provider model load failures into state.error', async () => {
			const mockProviders = [
				createMockProvider('prov-1', 'Provider 1', true),
				createMockProvider('prov-2', 'Provider 2', true)
			];

			vi.mocked(getProviders).mockResolvedValue(mockProviders);
			vi.mocked(getOperationDefaults).mockResolvedValue([]);
			vi.mocked(getModels).mockRejectedValue(new Error('Network down'));

			await configStore.initialize();

			const state = get(configStore);
			expect(state.providersLoading).toBe('success');
			expect(state.error).toMatch(/2 provider/);
			expect(state.error).toContain('Provider 1');
			expect(state.error).toContain('Provider 2');
			expect(state.error).toContain('Network down');
		});

		it('initialize leaves state.error null when all provider model loads succeed', async () => {
			const mockProviders = [createMockProvider('prov-1', 'Provider 1', true)];

			vi.mocked(getProviders).mockResolvedValue(mockProviders);
			vi.mocked(getOperationDefaults).mockResolvedValue([]);
			vi.mocked(getModels).mockResolvedValue([createMockModel('m-1', 'prov-1', 'Model 1')]);

			await configStore.initialize();

			const state = get(configStore);
			expect(state.providersLoading).toBe('success');
			expect(state.error).toBeNull();
		});

		it('initialize handles initialization error', async () => {
			vi.mocked(getProviders).mockRejectedValue(new Error('Network error'));

			await configStore.initialize();

			const state = get(configStore);
			expect(state.providersLoading).toBe('error');
			expect(state.error).toBe('Network error');
		});
	});

	describe('Provider Getters', () => {
		it('providers getter returns current providers', () => {
			const mockProviders = [
				createMockProvider('prov-1', 'Provider 1'),
				createMockProvider('prov-2', 'Provider 2')
			];

			const state = get(configStore);
			state.providers = mockProviders;

			expect(configStore.providers).toEqual(mockProviders);
		});

		it('configuredCount returns count of configured providers', () => {
			const state = get(configStore);
			state.providers = [
				createMockProvider('prov-1', 'Provider 1', true),
				createMockProvider('prov-2', 'Provider 2', false),
				createMockProvider('prov-3', 'Provider 3', true)
			];

			expect(configStore.configuredCount).toBe(2);
		});

		it('getProvider returns provider by ID', () => {
			const mockProviders = [
				createMockProvider('prov-1', 'Provider 1'),
				createMockProvider('prov-2', 'Provider 2')
			];

			const state = get(configStore);
			state.providers = mockProviders;

			expect(configStore.getProvider('prov-1')?.name).toBe('Provider 1');
			expect(configStore.getProvider('nonexistent')).toBeUndefined();
		});

		it('getModels returns models for provider', () => {
			const mockModels = [
				createMockModel('model-1', 'prov-1', 'Model 1'),
				createMockModel('model-2', 'prov-1', 'Model 2')
			];

			const state = get(configStore);
			state.modelsByProvider = { 'prov-1': mockModels };

			expect(configStore.getModels('prov-1')).toEqual(mockModels);
			expect(configStore.getModels('nonexistent')).toEqual([]);
		});

		it('getDefault returns default model for operation', () => {
			const state = get(configStore);
			state.operationDefaults = [
				createMockOperationDefault('code', 'prov-1-model-1'),
				createMockOperationDefault('vision', null)
			];

			expect(configStore.getDefault('code')).toBe('prov-1-model-1');
			expect(configStore.getDefault('vision')).toBeNull();
			expect(configStore.getDefault('analysis' as OperationType)).toBeNull();
		});

		it('getAvailableModels returns models with matching capability', () => {
			const mockProviders = [createMockProvider('prov-1', 'Provider 1', true)];
			const mockModels = [
				createMockModel('model-1', 'prov-1', 'Code Model'),
				createMockModel('model-2', 'prov-1', 'Vision Model')
			];
			mockModels[1].capabilities.vision = true;
			mockModels[1].capabilities.code = false;

			const state = get(configStore);
			state.providers = mockProviders;
			state.modelsByProvider = { 'prov-1': mockModels };

			const codeModels = configStore.getAvailableModels('code');
			const visionModels = configStore.getAvailableModels('vision');

			expect(codeModels).toHaveLength(1);
			expect(codeModels[0].id).toBe('model-1');
			expect(visionModels).toHaveLength(1);
			expect(visionModels[0].id).toBe('model-2');
		});

		it('getAvailableModels returns empty array for unconfigured providers', () => {
			const state = get(configStore);
			state.providers = [createMockProvider('prov-1', 'Provider 1', false)];

			expect(configStore.getAvailableModels('code')).toEqual([]);
		});
	});

	describe('Provider Management', () => {
		it('saveProvider creates new provider', async () => {
			const newProvider = createMockProvider('prov-new', 'New Provider');

			vi.mocked(saveProvider).mockResolvedValue(newProvider);

			const result = await configStore.saveProvider({
				id: 'prov-new',
				name: 'New Provider',
				icon: 'NP',
				apiType: 'openai'
			});

			expect(result).toEqual(newProvider);
			expect(saveProvider).toHaveBeenCalledWith({
				id: 'prov-new',
				name: 'New Provider',
				icon: 'NP',
				apiType: 'openai'
			});

			const state = get(configStore);
			expect(state.providers).toContainEqual(newProvider);
		});

		it('saveProvider updates existing provider', async () => {
			const existingProvider = createMockProvider('prov-1', 'Old Name');
			const updatedProvider = createMockProvider('prov-1', 'New Name');

			const state = get(configStore);
			state.providers = [existingProvider];

			vi.mocked(saveProvider).mockResolvedValue(updatedProvider);

			await configStore.saveProvider({
				id: 'prov-1',
				name: 'New Name',
				icon: 'P1',
				apiType: 'openai'
			});

			const newState = get(configStore);
			expect(newState.providers[0].name).toBe('New Name');
		});

		it('removeProvider removes provider and associated data', async () => {
			const mockProviders = [
				createMockProvider('prov-1', 'Provider 1'),
				createMockProvider('prov-2', 'Provider 2')
			];
			const mockModels = [createMockModel('model-1', 'prov-1', 'Model')];

			const state = get(configStore);
			state.providers = mockProviders;
			state.modelsByProvider = { 'prov-1': mockModels };
			state.operationDefaults = [
				createMockOperationDefault('code', 'prov-1-model-1'),
				createMockOperationDefault('vision', 'prov-2-model-1')
			];

			await configStore.removeProvider('prov-1');

			expect(removeProvider).toHaveBeenCalledWith('prov-1');

			const newState = get(configStore);
			expect(newState.providers).toHaveLength(1);
			expect(newState.providers[0].id).toBe('prov-2');
			expect(newState.modelsByProvider['prov-1']).toBeUndefined();
			// Operation defaults referencing removed provider should be cleared
			expect(newState.operationDefaults).toHaveLength(1);
			expect(newState.operationDefaults[0].operation).toBe('vision');
		});

		it('reloadProviders reloads from backend', async () => {
			const mockProviders = [createMockProvider('prov-1', 'Provider 1', true)];

			vi.mocked(getProviders).mockResolvedValue(mockProviders);

			await configStore.reloadProviders();

			expect(getProviders).toHaveBeenCalled();

			const state = get(configStore);
			expect(state.providers).toEqual(mockProviders);
		});

		it('fetchModels fetches and caches models', async () => {
			const mockModels = ['gpt-4', 'gpt-3.5-turbo'];

			vi.mocked(fetchModels).mockResolvedValue({
				success: true,
				models: mockModels
			});
			vi.mocked(getProviders).mockResolvedValue([createMockProvider('prov-1', 'Provider 1', true)]);

			const result = await configStore.fetchModels('prov-1', 'test-key', 'openai');

			expect(result).toEqual(mockModels);
			expect(fetchModels).toHaveBeenCalledWith('prov-1', 'test-key', 'openai', undefined);

			const state = get(configStore);
			expect(state.modelsByProvider['prov-1']).toBeDefined();
			expect(state.modelsByProvider['prov-1'][0].name).toBe('gpt-4');
		});

		it('fetchModels sets provider as configured after success', async () => {
			vi.mocked(fetchModels).mockResolvedValue({
				success: true,
				models: ['model-1']
			});
			vi.mocked(getProviders).mockResolvedValue([
				createMockProvider('prov-1', 'Provider 1', false)
			]);

			await configStore.fetchModels('prov-1', 'test-key', 'openai');

			const state = get(configStore);
			expect(state.providers[0].isConfigured).toBe(true);
		});

		it('fetchModels throws error on failure', async () => {
			vi.mocked(fetchModels).mockResolvedValue({
				success: false,
				models: [],
				message: 'Invalid API key'
			});

			await expect(configStore.fetchModels('prov-1', 'invalid-key', 'openai')).rejects.toThrow(
				'Invalid API key'
			);
		});

		it('fetchModels calls reloadProviders after success', async () => {
			vi.mocked(fetchModels).mockResolvedValue({
				success: true,
				models: ['model-1']
			});
			vi.mocked(getProviders).mockResolvedValue([createMockProvider('prov-1', 'Provider 1', true)]);

			await configStore.fetchModels('prov-1', 'test-key', 'openai');

			expect(getProviders).toHaveBeenCalled();
		});
	});

	describe('API Key Management', () => {
		it('validateApiKey validates key with backend', async () => {
			const result = await configStore.validateApiKey('prov-1', 'test-key', 'openai');

			expect(result).toEqual({ valid: true, message: 'Valid' });
			expect(validateApiKey).toHaveBeenCalledWith('prov-1', 'test-key', 'openai', undefined);
		});

		it('validateApiKey passes baseUrl to API', async () => {
			await configStore.validateApiKey('prov-1', 'test-key', 'openai', 'https://api.example.com');

			expect(validateApiKey).toHaveBeenCalledWith(
				'prov-1',
				'test-key',
				'openai',
				'https://api.example.com'
			);
		});

		it('storeApiKey stores key in keychain', async () => {
			await configStore.storeApiKey('prov-1', 'test-api-key');

			expect(storeApiKey).toHaveBeenCalledWith('prov-1', 'test-api-key');
		});

		it('retrieveApiKey retrieves key from keychain', async () => {
			const result = await configStore.retrieveApiKey('prov-1');

			expect(result).toBe('test-api-key');
			expect(retrieveApiKey).toHaveBeenCalledWith('prov-1');
		});

		it('deleteApiKey deletes key from keychain', async () => {
			await configStore.deleteApiKey('prov-1');

			expect(deleteApiKey).toHaveBeenCalledWith('prov-1');
		});
	});

	describe('Operation Defaults', () => {
		it('setOperationDefault creates new default', async () => {
			const state = get(configStore);
			state.operationDefaults = [];

			await configStore.setOperationDefault('code', 'prov-1-model-1');

			expect(setOperationDefault).toHaveBeenCalledWith('code', 'prov-1-model-1');

			const newState = get(configStore);
			expect(newState.operationDefaults).toHaveLength(1);
			expect(newState.operationDefaults[0]).toEqual({
				operation: 'code',
				modelId: 'prov-1-model-1'
			});
		});

		it('setOperationDefault updates existing default', async () => {
			const state = get(configStore);
			state.operationDefaults = [createMockOperationDefault('code', 'old-model-id')];

			await configStore.setOperationDefault('code', 'new-model-id');

			expect(setOperationDefault).toHaveBeenCalledWith('code', 'new-model-id');

			const newState = get(configStore);
			expect(newState.operationDefaults).toHaveLength(1);
			expect(newState.operationDefaults[0].modelId).toBe('new-model-id');
		});
	});

	describe('Manual Models', () => {
		it('getManualModels returns manual models for provider', async () => {
			const mockManualModels = [
				{
					name: 'custom-model-1',
					capabilities: {
						analysis: true,
						code: true,
						vision: false,
						audio: false,
						tips: true,
						review: true,
						streaming: true
					}
				}
			];

			vi.mocked(getManualModels).mockResolvedValue(mockManualModels);

			const result = await configStore.getManualModels('prov-1');

			expect(result).toEqual(mockManualModels);
			expect(getManualModels).toHaveBeenCalledWith('prov-1');
		});

		it('addManualModel adds manual model', async () => {
			await configStore.addManualModel('prov-1', 'custom-model', {
				analysis: true,
				code: true,
				vision: false,
				audio: false,
				tips: true,
				review: true,
				streaming: true
			});

			expect(addManualModel).toHaveBeenCalledWith('prov-1', 'custom-model', {
				analysis: true,
				code: true,
				vision: false,
				audio: false,
				tips: true,
				review: true,
				streaming: true
			});
		});

		it('addManualModel works without capabilities', async () => {
			await configStore.addManualModel('prov-1', 'custom-model');

			expect(addManualModel).toHaveBeenCalledWith('prov-1', 'custom-model', undefined);
		});

		it('removeManualModel removes manual model', async () => {
			await configStore.removeManualModel('prov-1', 'custom-model');

			expect(removeManualModel).toHaveBeenCalledWith('prov-1', 'custom-model');
		});
	});

	describe('Provider Models Source', () => {
		it('setProviderModelsSource updates provider models source', async () => {
			const state = get(configStore);
			state.providers = [createMockProvider('prov-1', 'Provider 1')];

			await configStore.setProviderModelsSource('prov-1', 'manual');

			expect(setProviderModelsSource).toHaveBeenCalledWith('prov-1', 'manual');

			const newState = get(configStore);
			expect(newState.providers[0].modelsSource).toBe('manual');
		});
	});

	describe('Error Handling', () => {
		it('clearError clears error state', () => {
			const state = get(configStore);
			state.error = 'Test error';

			configStore.clearError();

			const newState = get(configStore);
			expect(newState.error).toBeNull();
		});

		it('handles saveProvider errors', async () => {
			vi.mocked(saveProvider).mockRejectedValue(new Error('Save failed'));

			await expect(
				configStore.saveProvider({
					name: 'Test',
					icon: 'T',
					apiType: 'openai'
				})
			).rejects.toThrow('Save failed');
		});

		it('saveProvider surfaces error into store state on failure', async () => {
			const state = get(configStore);
			state.error = null;

			vi.mocked(saveProvider).mockRejectedValue(new Error('Database locked'));

			await expect(
				configStore.saveProvider({ name: 'Test', icon: 'T', apiType: 'openai' })
			).rejects.toThrow('Database locked');

			const newState = get(configStore);
			expect(newState.error).toBe('Database locked');
		});

		it('handles removeProvider errors', async () => {
			vi.mocked(removeProvider).mockRejectedValue(new Error('Remove failed'));

			await expect(configStore.removeProvider('prov-1')).rejects.toThrow('Remove failed');
		});

		it('removeProvider surfaces error into store state on failure', async () => {
			const state = get(configStore);
			state.providers = [
				createMockProvider('prov-1', 'Provider 1'),
				createMockProvider('prov-2', 'Provider 2')
			];
			state.error = null;

			vi.mocked(removeProvider).mockRejectedValue(new Error('DB locked'));

			await expect(configStore.removeProvider('prov-1')).rejects.toThrow('DB locked');

			const newState = get(configStore);
			expect(newState.error).toBe('DB locked');
		});

		it('removeProvider does not mutate providers list on failure', async () => {
			const state = get(configStore);
			const initialProviders = [
				createMockProvider('prov-1', 'Provider 1'),
				createMockProvider('prov-2', 'Provider 2')
			];
			state.providers = initialProviders;

			vi.mocked(removeProvider).mockRejectedValue(new Error('Remove failed'));

			await expect(configStore.removeProvider('prov-1')).rejects.toThrow();

			const newState = get(configStore);
			expect(newState.providers).toHaveLength(2);
			expect(newState.providers.map((p) => p.id)).toEqual(['prov-1', 'prov-2']);
		});

		it('handles fetchModels API errors', async () => {
			vi.mocked(fetchModels).mockRejectedValue(new Error('Network error'));

			await expect(configStore.fetchModels('prov-1', 'test-key', 'openai')).rejects.toThrow(
				'Network error'
			);
		});

		it('fetchModels surfaces network errors into store state', async () => {
			const state = get(configStore);
			state.error = null;

			vi.mocked(fetchModels).mockRejectedValue(new Error('Network error'));

			await expect(configStore.fetchModels('prov-1', 'test-key', 'openai')).rejects.toThrow(
				'Network error'
			);

			const newState = get(configStore);
			expect(newState.error).toBe('Network error');
		});

		it('fetchModels surfaces backend-reported failures into store state', async () => {
			const state = get(configStore);
			state.error = null;

			vi.mocked(fetchModels).mockResolvedValue({
				success: false,
				models: [],
				message: 'Invalid API key'
			});

			await expect(configStore.fetchModels('prov-1', 'bad-key', 'openai')).rejects.toThrow(
				'Invalid API key'
			);

			const newState = get(configStore);
			expect(newState.error).toBe('Invalid API key');
		});

		it('handles storeApiKey errors', async () => {
			vi.mocked(storeApiKey).mockRejectedValue(new Error('Keychain error'));

			await expect(configStore.storeApiKey('prov-1', 'test-key')).rejects.toThrow('Keychain error');
		});

		it('handles retrieveApiKey errors', async () => {
			vi.mocked(retrieveApiKey).mockRejectedValue(new Error('Not found'));

			await expect(configStore.retrieveApiKey('prov-1')).rejects.toThrow('Not found');
		});

		it('handles deleteApiKey errors', async () => {
			vi.mocked(deleteApiKey).mockRejectedValue(new Error('Delete failed'));

			await expect(configStore.deleteApiKey('prov-1')).rejects.toThrow('Delete failed');
		});

		it('setOperationDefault surfaces error into store state on failure', async () => {
			const state = get(configStore);
			state.error = null;

			vi.mocked(setOperationDefault).mockRejectedValue(new Error('Update failed'));

			await expect(configStore.setOperationDefault('code', 'model-1')).rejects.toThrow(
				'Update failed'
			);

			const newState = get(configStore);
			expect(newState.error).toBe('Update failed');
		});

		it('handles setOperationDefault errors', async () => {
			vi.mocked(setOperationDefault).mockRejectedValue(new Error('Update failed'));

			await expect(configStore.setOperationDefault('code', 'model-1')).rejects.toThrow(
				'Update failed'
			);
		});
	});
});

describe('configStore Derived Stores', () => {
	beforeEach(() => {
		vi.clearAllMocks();

		// Reset store state
		const state = get(configStore);
		state.providers = [];
		state.modelsByProvider = {};
		state.operationDefaults = [];
		state.error = null;
	});

	it('configuredProviders returns only configured providers', async () => {
		const { configuredProviders } = await import('./configStore');

		const state = get(configStore);
		state.providers = [
			createMockProvider('prov-1', 'Provider 1', true),
			createMockProvider('prov-2', 'Provider 2', false),
			createMockProvider('prov-3', 'Provider 3', true)
		];

		expect(get(configuredProviders)).toHaveLength(2);
		expect(get(configuredProviders)[0].id).toBe('prov-1');
		expect(get(configuredProviders)[1].id).toBe('prov-3');
	});

	it('hasConfiguredProvider returns true when any provider is configured', async () => {
		const { hasConfiguredProvider } = await import('./configStore');

		expect(get(hasConfiguredProvider)).toBe(false);

		const state = get(configStore);
		state.providers = [createMockProvider('prov-1', 'Provider 1', true)];

		expect(get(hasConfiguredProvider)).toBe(true);
	});

	it('isSetupComplete returns true when any provider is configured', async () => {
		const { isSetupComplete } = await import('./configStore');

		expect(get(isSetupComplete)).toBe(false);

		const state = get(configStore);
		state.providers = [createMockProvider('prov-1', 'Provider 1', true)];

		expect(get(isSetupComplete)).toBe(true);
	});

	it('availableModelsByOperation groups models by operation', async () => {
		const { availableModelsByOperation } = await import('./configStore');

		const codeModel = createMockModel('model-1', 'prov-1', 'Code Model');
		const visionModel = createMockModel('model-2', 'prov-1', 'Vision Model');
		visionModel.capabilities.vision = true;
		visionModel.capabilities.code = false;

		const state = get(configStore);
		state.providers = [createMockProvider('prov-1', 'Provider 1', true)];
		state.modelsByProvider = { 'prov-1': [codeModel, visionModel] };

		const available = get(availableModelsByOperation);

		expect(available.code).toHaveLength(1);
		expect(available.code[0].id).toBe('model-1');
		expect(available.vision).toHaveLength(1);
		expect(available.vision[0].id).toBe('model-2');
	});

	it('availableModelsByOperation includes models in multiple operation groups', async () => {
		const { availableModelsByOperation } = await import('./configStore');

		const allCapabilitiesModel = createMockModel('model-1', 'prov-1', 'All Capabilities');
		allCapabilitiesModel.capabilities = {
			analysis: true,
			code: true,
			vision: true,
			audio: true,
			tips: true,
			review: true,
			streaming: true
		};

		const state = get(configStore);
		state.providers = [createMockProvider('prov-1', 'Provider 1', true)];
		state.modelsByProvider = { 'prov-1': [allCapabilitiesModel] };

		const available = get(availableModelsByOperation);

		// Model should appear in all operation groups
		expect(available.analysis).toHaveLength(1);
		expect(available.code).toHaveLength(1);
		expect(available.vision).toHaveLength(1);
		expect(available.audio).toHaveLength(1);
		expect(available.tips).toHaveLength(1);
		expect(available.review).toHaveLength(1);
	});

	it('availableModelsByOperation handles no providers configured', async () => {
		const { availableModelsByOperation } = await import('./configStore');

		const state = get(configStore);
		state.providers = [createMockProvider('prov-1', 'Provider 1', false)];

		const available = get(availableModelsByOperation);

		expect(available.code).toEqual([]);
		expect(available.vision).toEqual([]);
		expect(available.analysis).toEqual([]);
	});

	it('availableModelsByOperation handles no models loaded', async () => {
		const { availableModelsByOperation } = await import('./configStore');

		const state = get(configStore);
		state.providers = [createMockProvider('prov-1', 'Provider 1', true)];
		state.modelsByProvider = {};

		const available = get(availableModelsByOperation);

		expect(available.code).toEqual([]);
		expect(available.vision).toEqual([]);
	});
});
