/**
 * API Exports
 * Central export point for all API clients.
 */

// NLP API
export {
	NlpApiError,
	classifyIntent,
	getConversationContext,
	addContextMessage,
	clearConversationContext,
	detectMultiIntent,
	detectAmbiguities,
	analyzeCode,
	updateProjectFiles,
	createDebouncedClassifier,
	isBackendAvailable
} from './nlp.js';

export type {
	ClassifyIntentOptions,
	AddContextMessageOptions,
	DetectMultiIntentOptions,
	DetectAmbiguitiesOptions,
	AnalyzeCodeOptions
} from './nlp.js';
