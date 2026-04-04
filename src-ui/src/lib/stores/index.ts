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
	isLlmStreaming,
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

// Document generation store
export {
	documentGenerationStore,
	activeDocGenJob,
	hasActiveDocGenJob,
	activeDocGenJobs
} from './documentGenerationStore.js';

// Cross-store derived: sessions filtered to current conversation
import { derived } from 'svelte/store';
import { chatStore } from './chat.js';
import { agentStore as _agentStore } from './agentStore.js';

export const conversationSessions = derived(
	[_agentStore, chatStore],
	([$agentState, $chatState]) => {
		const conversationId = $chatState.selectedConversationId;
		if (!conversationId) return [];
		return Array.from($agentState.sessions.values()).filter((s) => {
			if (s.conversationId !== conversationId) return false;
			// Hide sessions for resolved or archived threads
			const thread = $chatState.threads.find((t) => t.id === s.threadId);
			if (!thread) return false;
			return !thread.metadata.resolved && !thread.metadata.archived;
		});
	}
);

/** Map of sessionId → thread title for display in tabs */
export const sessionThreadTitles = derived(
	[_agentStore, chatStore],
	([$agentState, $chatState]) => {
		const titles = new Map<string, string>();
		for (const session of $agentState.sessions.values()) {
			const thread = $chatState.threads.find((t) => t.id === session.threadId);
			if (thread) {
				titles.set(session.id, thread.title);
			}
		}
		return titles;
	}
);
