/**
 * Tests for Config API Client
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
	getProviders,
	saveProvider,
	removeProvider,
	getModels,
	validateApiKey,
	testConnection,
	fetchModels,
	storeApiKey,
	retrieveApiKey,
	deleteApiKey,
	hasApiKey,
	getOperationDefaults,
	setOperationDefault,
	getManualModels,
	addManualModel,
	removeManualModel,
	setProviderModelsSource,
	ConfigApiError
} from '../config';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = vi.mocked(invoke);

describe('config API', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('getProviders', () => {
		it('should return providers on success', async () => {
			const mockProviders = [
				{
					id: 'anthropic',
					name: 'Anthropic',
					icon: 'AC',
					isCustom: false,
					apiType: 'anthropic' as const,
					baseUrl: undefined,
					isConfigured: true,
					modelsSource: 'auto' as const,
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T01:00:00Z'
				},
				{
					id: 'custom-1',
					name: 'Custom Provider',
					icon: 'CP',
					isCustom: true,
					apiType: 'openai' as const,
					baseUrl: 'https://api.example.com',
					isConfigured: false,
					modelsSource: 'manual' as const,
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T01:00:00Z'
				}
			];
			mockInvoke.mockResolvedValueOnce(mockProviders);

			const result = await getProviders();

			expect(mockInvoke).toHaveBeenCalledWith('get_providers', undefined);
			expect(result).toHaveLength(2);
			expect(result[0].id).toBe('anthropic');
			expect(result[0].isCustom).toBe(false);
			expect(result[1].id).toBe('custom-1');
			expect(result[1].isCustom).toBe(true);
			expect(result[1].baseUrl).toBe('https://api.example.com');
		});

		it('should return empty array when no providers exist', async () => {
			mockInvoke.mockResolvedValueOnce([]);

			const result = await getProviders();

			expect(mockInvoke).toHaveBeenCalledWith('get_providers', undefined);
			expect(result).toEqual([]);
		});

		it('should throw ConfigApiError on failure', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('Database error'));

			await expect(getProviders()).rejects.toThrow(ConfigApiError);
		});

		it('should handle missing streaming capability in model response', async () => {
			const mockModels = [
				{
					id: 'model-1',
					providerId: 'anthropic',
					name: 'Claude 3.5',
					capabilities: {
						analysis: true,
						code: true,
						vision: false,
						audio: false,
						tips: true,
						review: true,
						streaming: undefined // Missing streaming field
					},
					lastFetched: '2024-01-01T00:00:00Z'
				}
			];
			mockInvoke.mockResolvedValueOnce(mockModels);

			const result = await getModels('anthropic');

			expect(result[0].capabilities.streaming).toBe(true); // Should default to true
		});
	});

	describe('saveProvider', () => {
		it('should create a new provider', async () => {
			const newProvider = {
				id: 'custom-1',
				name: 'Custom Provider',
				icon: 'CP',
				isCustom: true,
				apiType: 'openai' as const,
				baseUrl: 'https://api.example.com',
				isConfigured: false,
				modelsSource: 'manual' as const
			};

			mockInvoke.mockResolvedValueOnce({
				...newProvider,
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T01:00:00Z'
			});

			const result = await saveProvider({
				name: 'Custom Provider',
				icon: 'CP',
				apiType: 'openai',
				baseUrl: 'https://api.example.com'
			});

			expect(mockInvoke).toHaveBeenCalledWith('save_provider', {
				request: {
					id: undefined,
					name: 'Custom Provider',
					icon: 'CP',
					apiType: 'openai',
					baseUrl: 'https://api.example.com'
				}
			});
			expect(result.name).toBe('Custom Provider');
		});

		it('should update an existing provider', async () => {
			const updatedProvider = {
				id: 'custom-1',
				name: 'Updated Provider',
				icon: 'UP',
				isCustom: true,
				apiType: 'openai' as const,
				baseUrl: 'https://new-url.com',
				isConfigured: true,
				modelsSource: 'auto' as const,
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-02T00:00:00Z'
			};

			mockInvoke.mockResolvedValueOnce(updatedProvider);

			const result = await saveProvider({
				id: 'custom-1',
				name: 'Updated Provider',
				icon: 'UP',
				apiType: 'openai',
				baseUrl: 'https://new-url.com'
			});

			expect(mockInvoke).toHaveBeenCalledWith('save_provider', {
				request: {
					id: 'custom-1',
					name: 'Updated Provider',
					icon: 'UP',
					apiType: 'openai',
					baseUrl: 'https://new-url.com'
				}
			});
			expect(result.name).toBe('Updated Provider');
		});

		it('should handle provider without baseUrl', async () => {
			const provider = {
				id: 'anthropic',
				name: 'Anthropic',
				icon: 'AC',
				isCustom: false,
				apiType: 'anthropic' as const,
				baseUrl: undefined,
				isConfigured: true,
				modelsSource: 'auto' as const,
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T01:00:00Z'
			};

			mockInvoke.mockResolvedValueOnce(provider);

			const result = await saveProvider({
				name: 'Anthropic',
				icon: 'AC',
				apiType: 'anthropic'
			});

			expect(result.baseUrl).toBeUndefined();
		});
	});

	describe('removeProvider', () => {
		it('should remove a provider', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await removeProvider('custom-1');

			expect(mockInvoke).toHaveBeenCalledWith('remove_provider', {
				providerId: 'custom-1'
			});
		});

		it('should throw ConfigApiError on failure', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('Provider not found'));

			await expect(removeProvider('invalid-id')).rejects.toThrow(ConfigApiError);
		});
	});

	describe('getModels', () => {
		it('should return models for a provider', async () => {
			const mockModels = [
				{
					id: 'anthropic-claude-3-5',
					providerId: 'anthropic',
					name: 'Claude 3.5 Sonnet',
					capabilities: {
						analysis: true,
						code: true,
						vision: true,
						audio: false,
						tips: true,
						review: true,
						streaming: true
					},
					lastFetched: '2024-01-01T00:00:00Z'
				}
			];
			mockInvoke.mockResolvedValueOnce(mockModels);

			const result = await getModels('anthropic');

			expect(mockInvoke).toHaveBeenCalledWith('get_models', {
				providerId: 'anthropic'
			});
			expect(result).toHaveLength(1);
			expect(result[0].name).toBe('Claude 3.5 Sonnet');
			expect(result[0].capabilities.vision).toBe(true);
		});

		it('should return empty array when no models exist', async () => {
			mockInvoke.mockResolvedValueOnce([]);

			const result = await getModels('new-provider');

			expect(result).toEqual([]);
		});
	});

	describe('validateApiKey', () => {
		it('should return valid result for good key', async () => {
			const validationResult = {
				valid: true,
				message: 'API key is valid'
			};
			mockInvoke.mockResolvedValueOnce(validationResult);

			const result = await validateApiKey('anthropic', 'sk-test-key', 'anthropic');

			expect(mockInvoke).toHaveBeenCalledWith('validate_api_key', {
				providerId: 'anthropic',
				apiKey: 'sk-test-key',
				apiType: 'anthropic',
				baseUrl: undefined
			});
			expect(result.valid).toBe(true);
		});

		it('should return invalid result for bad key', async () => {
			const validationResult = {
				valid: false,
				message: 'Invalid API key'
			};
			mockInvoke.mockResolvedValueOnce(validationResult);

			const result = await validateApiKey('anthropic', 'bad-key', 'anthropic');

			expect(result.valid).toBe(false);
			expect(result.message).toBe('Invalid API key');
		});

		it('should include baseUrl for custom provider', async () => {
			const validationResult = {
				valid: true,
				message: 'Connected successfully'
			};
			mockInvoke.mockResolvedValueOnce(validationResult);

			await validateApiKey('custom-1', 'sk-test-key', 'openai', 'https://api.example.com');

			expect(mockInvoke).toHaveBeenCalledWith('validate_api_key', {
				providerId: 'custom-1',
				apiKey: 'sk-test-key',
				apiType: 'openai',
				baseUrl: 'https://api.example.com'
			});
		});
	});

	describe('testConnection', () => {
		it('should test connection to custom provider', async () => {
			const result = {
				valid: true,
				message: 'Connection successful'
			};
			mockInvoke.mockResolvedValueOnce(result);

			const response = await testConnection('https://api.example.com', 'sk-test-key', 'openai');

			expect(mockInvoke).toHaveBeenCalledWith('test_connection', {
				baseUrl: 'https://api.example.com',
				apiKey: 'sk-test-key',
				apiType: 'openai'
			});
			expect(response.valid).toBe(true);
		});

		it('should return failure for invalid connection', async () => {
			const result = {
				valid: false,
				message: 'Connection failed: Invalid URL'
			};
			mockInvoke.mockResolvedValueOnce(result);

			const response = await testConnection('https://invalid-url.com', 'bad-key', 'openai');

			expect(response.valid).toBe(false);
		});
	});

	describe('fetchModels', () => {
		it('should fetch models from provider', async () => {
			const result = {
				success: true,
				models: ['gpt-4', 'gpt-3.5-turbo'],
				message: 'Fetched 2 models'
			};
			mockInvoke.mockResolvedValueOnce(result);

			const response = await fetchModels('openai', 'sk-test-key', 'openai');

			expect(mockInvoke).toHaveBeenCalledWith('fetch_models', {
				providerId: 'openai',
				apiKey: 'sk-test-key',
				apiType: 'openai',
				baseUrl: undefined
			});
			expect(response.models).toEqual(['gpt-4', 'gpt-3.5-turbo']);
		});

		it('should handle fetch failure', async () => {
			const result = {
				success: false,
				models: [],
				message: 'Authentication failed'
			};
			mockInvoke.mockResolvedValueOnce(result);

			const response = await fetchModels('openai', 'bad-key', 'openai');

			expect(response.success).toBe(false);
			expect(response.models).toEqual([]);
		});
	});

	describe('Keychain commands', () => {
		describe('storeApiKey', () => {
			it('should store API key', async () => {
				mockInvoke.mockResolvedValueOnce(undefined);

				await storeApiKey('anthropic', 'sk-test-key');

				expect(mockInvoke).toHaveBeenCalledWith('store_api_key', {
					providerId: 'anthropic',
					apiKey: 'sk-test-key'
				});
			});
		});

		describe('retrieveApiKey', () => {
			it('should retrieve stored API key', async () => {
				mockInvoke.mockResolvedValueOnce('sk-test-key');

				const result = await retrieveApiKey('anthropic');

				expect(mockInvoke).toHaveBeenCalledWith('retrieve_api_key', {
					providerId: 'anthropic'
				});
				expect(result).toBe('sk-test-key');
			});

			it('should throw ConfigApiError when key not found', async () => {
				mockInvoke.mockRejectedValueOnce(new Error('Key not found'));

				await expect(retrieveApiKey('unknown-provider')).rejects.toThrow(ConfigApiError);
			});
		});

		describe('deleteApiKey', () => {
			it('should delete API key', async () => {
				mockInvoke.mockResolvedValueOnce(undefined);

				await deleteApiKey('anthropic');

				expect(mockInvoke).toHaveBeenCalledWith('delete_api_key', {
					providerId: 'anthropic'
				});
			});
		});

		describe('hasApiKey', () => {
			it('should return true when key exists', async () => {
				mockInvoke.mockResolvedValueOnce(true);

				const result = await hasApiKey('anthropic');

				expect(result).toBe(true);
			});

			it('should return false when key does not exist', async () => {
				mockInvoke.mockResolvedValueOnce(false);

				const result = await hasApiKey('new-provider');

				expect(result).toBe(false);
			});
		});
	});

	describe('Operation defaults', () => {
		describe('getOperationDefaults', () => {
			it('should return all operation defaults', async () => {
				const mockDefaults = [
					{ operation: 'code', modelId: 'anthropic-claude-3-5' },
					{ operation: 'analysis', modelId: 'openai-gpt-4' },
					{ operation: 'vision', modelId: null }
				];
				mockInvoke.mockResolvedValueOnce(mockDefaults);

				const result = await getOperationDefaults();

				expect(mockInvoke).toHaveBeenCalledWith('get_operation_defaults', undefined);
				expect(result).toHaveLength(3);
				expect(result[0].operation).toBe('code');
				expect(result[2].modelId).toBeNull();
			});
		});

		describe('setOperationDefault', () => {
			it('should set default model for operation', async () => {
				mockInvoke.mockResolvedValueOnce(undefined);

				await setOperationDefault('code', 'anthropic-claude-3-5');

				expect(mockInvoke).toHaveBeenCalledWith('set_operation_default', {
					operation: 'code',
					modelId: 'anthropic-claude-3-5'
				});
			});
		});
	});

	describe('Manual models', () => {
		describe('getManualModels', () => {
			it('should return manual models for provider', async () => {
				const mockModels = [
					{
						name: 'custom-model-1',
						capabilities: {
							analysis: true,
							code: true,
							vision: false,
							audio: false,
							tips: true,
							review: true
						}
					}
				];
				mockInvoke.mockResolvedValueOnce(mockModels);

				const result = await getManualModels('custom-provider');

				expect(mockInvoke).toHaveBeenCalledWith('get_manual_models', {
					providerId: 'custom-provider'
				});
				expect(result).toHaveLength(1);
				expect(result[0].name).toBe('custom-model-1');
				expect(result[0].capabilities.streaming).toBe(true); // Should default to true
			});

			it('should return empty array when no manual models', async () => {
				mockInvoke.mockResolvedValueOnce([]);

				const result = await getManualModels('new-provider');

				expect(result).toEqual([]);
			});
		});

		describe('addManualModel', () => {
			it('should add manual model', async () => {
				mockInvoke.mockResolvedValueOnce(undefined);

				await addManualModel('custom-provider', 'gpt-custom', {
					analysis: true,
					code: true,
					vision: false,
					audio: false,
					tips: true,
					review: true,
					streaming: true
				});

				expect(mockInvoke).toHaveBeenCalledWith('add_manual_model', {
					request: {
						providerId: 'custom-provider',
						name: 'gpt-custom',
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
				});
			});

			it('should add manual model without capabilities', async () => {
				mockInvoke.mockResolvedValueOnce(undefined);

				await addManualModel('custom-provider', 'gpt-custom');

				expect(mockInvoke).toHaveBeenCalledWith('add_manual_model', {
					request: {
						providerId: 'custom-provider',
						name: 'gpt-custom',
						capabilities: undefined
					}
				});
			});
		});

		describe('removeManualModel', () => {
			it('should remove manual model', async () => {
				mockInvoke.mockResolvedValueOnce(undefined);

				await removeManualModel('custom-provider', 'gpt-custom');

				expect(mockInvoke).toHaveBeenCalledWith('remove_manual_model', {
					providerId: 'custom-provider',
					modelName: 'gpt-custom'
				});
			});
		});

		describe('setProviderModelsSource', () => {
			it('should set models source for provider', async () => {
				mockInvoke.mockResolvedValueOnce(undefined);

				await setProviderModelsSource('anthropic', 'manual');

				expect(mockInvoke).toHaveBeenCalledWith('set_provider_models_source', {
					providerId: 'anthropic',
					modelsSource: 'manual'
				});
			});
		});
	});

	describe('Error handling', () => {
		it('should include command name in error', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('Backend error'));

			await expect(getProviders()).rejects.toThrow('get_providers');
		});

		it('should include original cause in error', async () => {
			const originalError = new Error('Network error');
			mockInvoke.mockRejectedValueOnce(originalError);

			try {
				await getProviders();
				expect.fail('Should have thrown');
			} catch (error) {
				expect(error).toBeInstanceOf(ConfigApiError);
				expect((error as ConfigApiError).cause).toBe(originalError);
			}
		});

		it('should handle non-Error errors', async () => {
			mockInvoke.mockRejectedValueOnce('String error');

			await expect(getProviders()).rejects.toThrow(ConfigApiError);
		});
	});
});
