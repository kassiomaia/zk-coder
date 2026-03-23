/**
 * Stores Exports
 * Central export point for all Svelte stores.
 */

// Chat store
export {
	chatStore,
	selectedConversation,
	selectedThread,
	currentThreads,
	activeStreamingThreadIds,
	unreadThreadIds,
	messageGroups,
	conversationGroups,
	nlpState,
	conversationsLoading,
	messagesLoading,
	threadsLoading,
	chatError,
	isLoading,
	generateId,
	createNewConversation,
	createNewThread,
	createNewMessage
} from './chat.js';

export type { LoadingState, NlpState, ChatState } from './chat.js';

// Config store
export {
	configStore,
	configuredProviders,
	hasConfiguredProvider,
	isSetupComplete
} from './configStore.js';
export type { ConfigState, ValidationState } from './configStore.js';

// Agent store
export {
	agentStore,
	activeSession,
	allSessions,
	pendingActions,
	agentPanelOpen,
	agentLoading,
	agentError
} from './agentStore.js';
