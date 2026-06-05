/**
 * Chat State Management
 *
 * Main Svelte store for managing conversations, messages, threads, and NLP state.
 * LLM streaming logic extracted to ./chat/llm-handler.ts
 */

import { createDerivedStores } from './chat/derived-stores';
import { createChatStore, initialState, log } from './createChatStore';

// ============================================================================
// Store Instance
// ============================================================================

export const chatStore = createChatStore();

// ============================================================================
// Derived Stores (created via factory to avoid circular dependency)
// ============================================================================

const derivedStores = createDerivedStores(chatStore);

export const selectedConversation = derivedStores.selectedConversation;
export const selectedThread = derivedStores.selectedThread;
export const currentThreads = derivedStores.currentThreads;
export const activeStreamingThreadIds = derivedStores.activeStreamingThreadIds;
export const unreadThreadIds = derivedStores.unreadThreadIds;
export const messageGroups = derivedStores.messageGroups;
export const conversationGroups = derivedStores.conversationGroups;
export const nlpState = derivedStores.nlpState;
export const isLlmStreaming = derivedStores.isLlmStreaming;
export const streamingContent = derivedStores.streamingContent;
export const conversationsLoading = derivedStores.conversationsLoading;
export const messagesLoading = derivedStores.messagesLoading;
export const threadsLoading = derivedStores.threadsLoading;
export const chatError = derivedStores.chatError;
export const isLoading = derivedStores.isLoading;

// ============================================================================
// Re-exports
// ============================================================================

// Types are in ./chat/types.ts - re-exported for convenience
export type {
	LoadingState,
	NlpState,
	LlmStreamingState,
	ThreadStreamingState,
	ChatState,
	LlmRequestOptions
} from './chat/types';

// Utility functions are in ./chat/utils.ts - re-exported for convenience
export { createNewConversation, createNewThread, createNewMessage } from './chat/utils';
