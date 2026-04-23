/**
 * Configuration API Client
 * TypeScript wrappers for Tauri IPC commands to the configuration backend.
 */

import { invokeCommand } from './core';
import type {
	ApiType,
	FetchModelsResult,
	ManualModelEntry,
	Model,
	ModelsSource,
	OperationDefault,
	Provider,
	SaveProviderRequest,
	ValidationResult
} from '$lib/types/config';
import { createLogger } from '$lib/utils/logger';

const log = createLogger('ConfigAPI');

// Re-export error for backwards compatibility
export { ConfigApiError } from './errors';

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
	modelsSource: ModelsSource;
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
	modelId: string | null;
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
		isConfigured: p.isConfigured,
		modelsSource: p.modelsSource
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
	log.debug('Getting providers...');
	const response = await invokeCommand<ProviderResponse[]>('config', 'get_providers');
	log.debug('Raw response from backend:', response);
	const providers = response.map(providerToClient);
	log.debug('Converted providers:', providers);
	return providers;
}

/**
 * Saves a provider (create or update).
 */
export async function saveProvider(request: SaveProviderRequest): Promise<Provider> {
	const response = await invokeCommand<ProviderResponse>('config', 'save_provider', {
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
	await invokeCommand<void>('config', 'remove_provider', { providerId });
}

/**
 * Gets models for a provider.
 */
export async function getModels(providerId: string): Promise<Model[]> {
	const response = await invokeCommand<ModelResponse[]>('config', 'get_models', { providerId });
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
	return invokeCommand<ValidationResult>('config', 'validate_api_key', {
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
	return invokeCommand<ValidationResult>('config', 'test_connection', {
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
	return invokeCommand<FetchModelsResult>('config', 'fetch_models', {
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
	await invokeCommand<void>('config', 'store_api_key', { providerId, apiKey });
}

/**
 * Retrieves an API key from the OS keychain.
 */
export async function retrieveApiKey(providerId: string): Promise<string> {
	return invokeCommand<string>('config', 'retrieve_api_key', { providerId });
}

/**
 * Deletes an API key from the OS keychain.
 */
export async function deleteApiKey(providerId: string): Promise<void> {
	await invokeCommand<void>('config', 'delete_api_key', { providerId });
}

/**
 * Checks if an API key exists for a provider.
 */
export async function hasApiKey(providerId: string): Promise<boolean> {
	return invokeCommand<boolean>('config', 'has_api_key', { providerId });
}

// ============================================================================
// Operation Default Commands
// ============================================================================

/**
 * Gets all operation defaults.
 */
export async function getOperationDefaults(): Promise<OperationDefault[]> {
	const response = await invokeCommand<OperationDefaultResponse[]>('config', 'get_operation_defaults');
	return response.map(operationDefaultToClient);
}

/**
 * Sets the default model for an operation.
 */
export async function setOperationDefault(operation: string, modelId: string): Promise<void> {
	await invokeCommand<void>('config', 'set_operation_default', { operation, modelId });
}

// ============================================================================
// Manual Model Commands
// ============================================================================

interface ManualModelResponse {
	name: string;
	capabilities: {
		analysis: boolean;
		code: boolean;
		vision: boolean;
		audio: boolean;
		tips: boolean;
		review: boolean;
	};
}

function manualModelToClient(m: ManualModelResponse): ManualModelEntry {
	return {
		name: m.name,
		capabilities: {
			analysis: m.capabilities.analysis,
			code: m.capabilities.code,
			vision: m.capabilities.vision,
			audio: m.capabilities.audio,
			tips: m.capabilities.tips,
			review: m.capabilities.review,
			streaming: true
		}
	};
}

/**
 * Gets all manual models for a provider.
 */
export async function getManualModels(providerId: string): Promise<ManualModelEntry[]> {
	const response = await invokeCommand<ManualModelResponse[]>('config', 'get_manual_models', { providerId });
	return response.map(manualModelToClient);
}

/**
 * Adds a manual model for a provider.
 */
export async function addManualModel(
	providerId: string,
	name: string,
	capabilities?: ManualModelEntry['capabilities']
): Promise<void> {
	await invokeCommand<void>('config', 'add_manual_model', {
		request: {
			providerId,
			name,
			capabilities
		}
	});
}

/**
 * Removes a manual model for a provider.
 */
export async function removeManualModel(providerId: string, modelName: string): Promise<void> {
	await invokeCommand<void>('config', 'remove_manual_model', { providerId, modelName });
}

/**
 * Sets the models source mode for a provider.
 */
export async function setProviderModelsSource(
	providerId: string,
	modelsSource: ModelsSource
): Promise<void> {
	await invokeCommand<void>('config', 'set_provider_models_source', { providerId, modelsSource });
}

// ============================================================================
// Prompt Template Commands
// ============================================================================

export interface PromptTemplate {
	id: string;
	slug: string;
	title: string;
	description: string;
	icon: string | null;
	category: string;
	content: string;
	isBuiltin: boolean;
	isEnabled: boolean;
	sortOrder: number;
	createdAt: string;
	updatedAt: string;
}

export interface SavePromptTemplateRequest {
	id?: string;
	slug: string;
	title: string;
	description: string;
	icon?: string | null;
	category: string;
	content: string;
	isEnabled: boolean;
}

/**
 * Lists all prompt templates.
 */
export async function listPromptTemplates(): Promise<PromptTemplate[]> {
	return invokeCommand<PromptTemplate[]>('config', 'list_prompt_templates');
}

/**
 * Creates or updates a prompt template.
 */
export async function savePromptTemplate(
	request: SavePromptTemplateRequest
): Promise<PromptTemplate> {
	return invokeCommand<PromptTemplate>('config', 'save_prompt_template', { request });
}

/**
 * Deletes a custom prompt template. Rejected for builtins by the backend.
 */
export async function deletePromptTemplate(id: string): Promise<void> {
	await invokeCommand<void>('config', 'delete_prompt_template', { id });
}

/**
 * Resets a builtin prompt template to its seeded default content.
 */
export async function resetPromptTemplate(slug: string): Promise<PromptTemplate> {
	return invokeCommand<PromptTemplate>('config', 'reset_prompt_template', { slug });
}
