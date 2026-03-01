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

// LLM API
export {
	LlmApiError,
	sendChatMessage,
	sendChatMessageStreaming,
	cancelChatRequest,
	messageResponseToClient,
	isStartedEvent,
	isChunkEvent,
	isCompleteEvent,
	isErrorEvent
} from './llm.js';

export type {
	UsageStats,
	SendChatMessageOptions,
	SendChatMessageResponse,
	StreamChatOptions,
	StreamChatResult,
	LlmEventType,
	LlmEventBase,
	LlmStartedEvent,
	LlmChunkEvent,
	LlmCompleteEvent,
	LlmErrorEvent,
	LlmEvent,
	LlmEventCallback
} from './llm.js';
