/**
 * Configuration API Client
 * TypeScript wrappers for Tauri IPC commands to the configuration backend.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
	ApiType,
	Provider,
	Model,
	OperationDefault,
	SaveProviderRequest,
	ValidationResult,
	FetchModelsResult
} from '$lib/types/config';

// ============================================================================
// Error Handling
// ============================================================================

/**
 * Error class for config API errors.
 */
export class ConfigApiError extends Error {
	constructor(
		message: string,
		public readonly command: string,
		public readonly cause?: unknown
	) {
		super(message);
		this.name = 'ConfigApiError';
	}
}

/**
 * Wraps invoke with error handling.
 */
async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await invoke<T>(command, args);
	} catch (error) {
		throw new ConfigApiError(
			`Config command '${command}' failed: ${error instanceof Error ? error.message : String(error)}`,
			command,
			error
		);
	}
}

// ============================================================================
// Backend Response Types
// ============================================================================

interface ProviderResponse {
	id: string;
	name: string;
	icon: string;
	isCustom: boolean;
	apiType: ApiType;
	baseUrl?: string;
	isConfigured: boolean;
	createdAt: string;
	updatedAt: string;
}

interface ModelResponse {
	id: string;
	providerId: string;
	name: string;
	capabilities: {
		analysis: boolean;
		code: boolean;
		vision: boolean;
		audio: boolean;
		tips: boolean;
		review: boolean;
		streaming: boolean;
	};
	lastFetched: string;
}

interface OperationDefaultResponse {
	operation: string;
	modelId: string;
}

// ============================================================================
// Type Conversion
// ============================================================================

function providerToClient(p: ProviderResponse): Provider {
	return {
		id: p.id,
		name: p.name,
		icon: p.icon,
		isCustom: p.isCustom,
		apiType: p.apiType,
		baseUrl: p.baseUrl,
		isConfigured: p.isConfigured
	};
}

function modelToClient(m: ModelResponse): Model {
	return {
		id: m.id,
		providerId: m.providerId,
		name: m.name,
		capabilities: {
			analysis: m.capabilities.analysis,
			code: m.capabilities.code,
			vision: m.capabilities.vision,
			audio: m.capabilities.audio,
			tips: m.capabilities.tips,
			review: m.capabilities.review,
			streaming: m.capabilities.streaming ?? true
		}
	};
}

function operationDefaultToClient(d: OperationDefaultResponse): OperationDefault {
	return {
		operation: d.operation as OperationDefault['operation'],
		modelId: d.modelId
	};
}

// ============================================================================
// Provider Commands
// ============================================================================

/**
 * Gets all providers.
 */
export async function getProviders(): Promise<Provider[]> {
	console.log('API: Getting providers...');
	const response = await safeInvoke<ProviderResponse[]>('get_providers');
	console.log('API: Raw response from backend:', response);
	const providers = response.map(providerToClient);
	console.log('API: Converted providers:', providers);
	return providers;
}

/**
 * Saves a provider (create or update).
 */
export async function saveProvider(request: SaveProviderRequest): Promise<Provider> {
	const response = await safeInvoke<ProviderResponse>('save_provider', {
		request: {
			id: request.id,
			name: request.name,
			icon: request.icon,
			apiType: request.apiType,
			baseUrl: request.baseUrl
		}
	});
	return providerToClient(response);
}

/**
 * Removes a custom provider.
 */
export async function removeProvider(providerId: string): Promise<void> {
	await safeInvoke<void>('remove_provider', { providerId });
}

/**
 * Gets models for a provider.
 */
export async function getModels(providerId: string): Promise<Model[]> {
	const response = await safeInvoke<ModelResponse[]>('get_models', { providerId });
	return response.map(modelToClient);
}

// ============================================================================
// Validation Commands
// ============================================================================

/**
 * Validates an API key with the provider.
 */
export async function validateApiKey(
	providerId: string,
	apiKey: string,
	apiType: ApiType,
	baseUrl?: string
): Promise<ValidationResult> {
	return safeInvoke<ValidationResult>('validate_api_key', {
		providerId,
		apiKey,
		apiType,
		baseUrl
	});
}

/**
 * Tests connection to a custom provider.
 */
export async function testConnection(
	baseUrl: string,
	apiKey: string,
	apiType: ApiType
): Promise<ValidationResult> {
	return safeInvoke<ValidationResult>('test_connection', {
		baseUrl,
		apiKey,
		apiType
	});
}

/**
 * Fetches available models from a provider.
 */
export async function fetchModels(
	providerId: string,
	apiKey: string,
	apiType: ApiType,
	baseUrl?: string
): Promise<FetchModelsResult> {
	return safeInvoke<FetchModelsResult>('fetch_models', {
		providerId,
		apiKey,
		apiType,
		baseUrl
	});
}

// ============================================================================
// Keychain Commands
// ============================================================================

/**
 * Stores an API key in the OS keychain.
 */
export async function storeApiKey(providerId: string, apiKey: string): Promise<void> {
	await safeInvoke<void>('store_api_key', { providerId, apiKey });
}

/**
 * Retrieves an API key from the OS keychain.
 */
export async function retrieveApiKey(providerId: string): Promise<string> {
	return safeInvoke<string>('retrieve_api_key', { providerId });
}

/**
 * Deletes an API key from the OS keychain.
 */
export async function deleteApiKey(providerId: string): Promise<void> {
	await safeInvoke<void>('delete_api_key', { providerId });
}

/**
 * Checks if an API key exists for a provider.
 */
export async function hasApiKey(providerId: string): Promise<boolean> {
	return safeInvoke<boolean>('has_api_key', { providerId });
}

// ============================================================================
// Operation Default Commands
// ============================================================================

/**
 * Gets all operation defaults.
 */
export async function getOperationDefaults(): Promise<OperationDefault[]> {
	const response = await safeInvoke<OperationDefaultResponse[]>('get_operation_defaults');
	return response.map(operationDefaultToClient);
}

/**
 * Sets the default model for an operation.
 */
export async function setOperationDefault(
	operation: string,
	modelId: string
): Promise<void> {
	await safeInvoke<void>('set_operation_default', { operation, modelId });
}
