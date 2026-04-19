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

// Web search tool config APIs
export {
	WEB_SEARCH_TOOL_ID,
	setToolCredentials,
	getToolCredentials,
	deleteToolCredentials,
	hasToolCredentials,
	testWebSearchCredentials
} from './toolCredentials.js';
export type { WebSearchCredentials } from './toolCredentials.js';

export {
	getWorkspaceSearchPolicy,
	setWorkspaceSearchPolicy,
	deleteWorkspaceSearchPolicy
} from './workspacePolicy.js';
export type { WorkspaceSearchPolicy } from './workspacePolicy.js';

export {
	getWebSearchCacheStats,
	clearWebSearchCache
} from './webSearchCache.js';
export type { WebSearchCacheStats } from './webSearchCache.js';

export { listWorkspaceDir } from './workspace.js';
export type { WorkspaceDirResponse } from './workspace.js';
