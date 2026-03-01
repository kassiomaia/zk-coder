/**
 * Configuration Types
 * TypeScript types for LLM provider and model configuration.
 */

/** API provider type */
export type ApiType = 'openai' | 'anthropic' | 'google';

/** Operation type for model assignment */
export type OperationType = 'analysis' | 'code' | 'vision' | 'audio' | 'tips' | 'review';

/** LLM provider configuration */
export interface Provider {
	/** Unique identifier */
	id: string;
	/** Display name */
	name: string;
	/** Icon identifier (e.g., "AC" for Anthropic) */
	icon: string;
	/** Whether this is a custom provider */
	isCustom: boolean;
	/** API type */
	apiType: ApiType;
	/** Custom base URL */
	baseUrl?: string;
	/** Whether the provider is configured with an API key */
	isConfigured: boolean;
}

/** Model capabilities */
export interface ModelCapabilities {
	analysis: boolean;
	code: boolean;
	vision: boolean;
	audio: boolean;
	tips: boolean;
	review: boolean;
	streaming: boolean;
}

/** LLM model */
export interface Model {
	/** Unique identifier */
	id: string;
	/** Provider ID */
	providerId: string;
	/** Display name */
	name: string;
	/** Model capabilities */
	capabilities: ModelCapabilities;
}

/** Operation default model assignment */
export interface OperationDefault {
	/** Operation type */
	operation: OperationType;
	/** Model ID */
	modelId: string;
}

/** Request to save a provider */
export interface SaveProviderRequest {
	id?: string;
	name: string;
	icon: string;
	isCustom?: boolean;
	apiType: ApiType;
	baseUrl?: string;
	isConfigured?: boolean;
}

/** API key validation result */
export interface ValidationResult {
	valid: boolean;
	message: string;
}

/** Model fetch result */
export interface FetchModelsResult {
	success: boolean;
	models: string[];
	message?: string;
}

/** Operation configuration for UI */
export interface OperationConfig {
	id: OperationType;
	name: string;
	description: string;
	icon: string;
	color: string;
}

/** All operation configurations */
export const OPERATION_CONFIGS: OperationConfig[] = [
	{
		id: 'analysis',
		name: 'Analysis',
		description: 'Simple models for quick questions',
		icon: 'search',
		color: 'var(--color-info)'
	},
	{
		id: 'code',
		name: 'Code Generation',
		description: 'Powerful models for generating code',
		icon: 'code',
		color: 'var(--color-warning)'
	},
	{
		id: 'vision',
		name: 'Images & Vision',
		description: 'Models with vision capability',
		icon: 'eye',
		color: 'var(--color-primary-500)'
	},
	{
		id: 'audio',
		name: 'Audio & Transcription',
		description: 'Audio recording and transcription',
		icon: 'mic',
		color: 'var(--color-success)'
	},
	{
		id: 'tips',
		name: 'Tips & Refinements',
		description: 'Quick suggestions and improvements',
		icon: 'lightbulb',
		color: 'var(--color-warning)'
	},
	{
		id: 'review',
		name: 'Code Review',
		description: 'Quality & best practices',
		icon: 'check-circle',
		color: 'var(--color-primary-500)'
	}
];

/** Provider icon class for styling */
export function getProviderIconClass(providerId: string): string {
	if (providerId === 'anthropic') return 'anthropic';
	if (providerId === 'openai') return 'openai';
	if (providerId === 'google') return 'google';
	return 'custom';
}

/** Default model names for each provider */
const PROVIDER_DEFAULT_MODELS: Record<string, string> = {
	anthropic: 'Claude 3.5, Claude 3 Opus, Claude 3 Sonnet',
	openai: 'GPT-4o, GPT-4 Turbo, GPT-3.5',
	google: 'Gemini Pro, Gemini Flash, Gemma'
};

/** Get default model names for a provider */
export function getProviderDefaultModels(providerId: string): string {
	return PROVIDER_DEFAULT_MODELS[providerId] ?? 'LLM Provider';
}
