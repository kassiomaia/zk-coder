/**
 * Chat Derived Stores
 *
 * Factory functions to create derived Svelte stores from the chat store.
 * Uses factory pattern to avoid circular dependency issues.
 */

import {derived, type Readable} from 'svelte/store';
import type {Conversation, ConversationGroup, Thread} from '$lib/types/thread';
import type {Message, MessageGroup} from '$lib/types/message';
import type {ChatState} from './types';
import {sortThreadsByRecent} from './utils';

// ============================================================================
// Factory Function Types
// ============================================================================

export interface DerivedStores {
	selectedConversation: Readable<Conversation | null>;
	selectedThread: Readable<Thread | null>;
	currentThreads: Readable<Thread[]>;
	activeStreamingThreadIds: Readable<string[]>;
	unreadThreadIds: Readable<string[]>;
	messageGroups: Readable<MessageGroup[]>;
	conversationGroups: Readable<ConversationGroup[]>;
	nlpState: Readable<ChatState['nlp']>;
	llmState: Readable<ChatState['llm']>;
	isLlmStreaming: Readable<boolean>;
	streamingContent: Readable<string>;
	conversationsLoading: Readable<ChatState['conversationsLoading']>;
	messagesLoading: Readable<ChatState['messagesLoading']>;
	threadsLoading: Readable<ChatState['threadsLoading']>;
	chatError: Readable<string | null>;
	isLoading: Readable<boolean>;
}

// ============================================================================
// Derived Stores Factory
// ============================================================================

/**
 * Create all derived stores from the chat store.
 * @param store - The main chat store to derive from
 */
export function createDerivedStores(store: Readable<ChatState>): DerivedStores {
	// Selection derived stores
	const selectedConversation = derived(
		store,
		($chat) => $chat.conversations.find((c) => c.id === $chat.selectedConversationId) ?? null
	);

	const selectedThread = derived(store, ($chat) => {
		if (!$chat.selectedThreadId) return null;
		return $chat.threads.find((t) => t.id === $chat.selectedThreadId) ?? null;
	});

	const currentThreads = derived(store, ($chat) => $chat.threads);
	const activeStreamingThreadIds = derived(store, ($chat) =>
		Object.entries($chat.threadStreaming)
			.filter(([, stream]) => stream.isStreaming)
			.map(([threadId]) => threadId)
	);
	const unreadThreadIds = derived(store, ($chat) => $chat.unreadThreadIds);

	// Message display derived stores
	const messageGroups = derived(store, ($chat): MessageGroup[] => {
		if ($chat.messages.length === 0) return [];

		const groups: MessageGroup[] = [];
		let currentGroup: Message[] = [];
		let lastTime = 0;
		const GROUP_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

		for (const message of $chat.messages) {
			const messageTime = message.timestamp.getTime();

			if (lastTime > 0 && messageTime - lastTime > GROUP_INTERVAL_MS) {
				if (currentGroup.length > 0) {
					groups.push({ messages: currentGroup });
				}
				currentGroup = [];
			}

			currentGroup.push(message);
			lastTime = messageTime;
		}

		if (currentGroup.length > 0) {
			groups.push({ messages: currentGroup });
		}

		return groups;
	});

	// Sidebar derived stores
	const conversationGroups = derived(store, ($chat): ConversationGroup[] => {
		const now = new Date();
		const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		const yesterday = new Date(today.getTime() - 24 * 60 * 60 * 1000);
		const weekAgo = new Date(today.getTime() - 7 * 24 * 60 * 60 * 1000);

		const groups: Record<string, Conversation[]> = {
			Today: [],
			Yesterday: [],
			'This Week': [],
			Older: []
		};

		for (const conversation of $chat.conversations) {
			const date = new Date(conversation.updatedAt);
			if (date >= today) {
				groups['Today'].push(conversation);
			} else if (date >= yesterday) {
				groups['Yesterday'].push(conversation);
			} else if (date >= weekAgo) {
				groups['This Week'].push(conversation);
			} else {
				groups['Older'].push(conversation);
			}
		}

		return Object.entries(groups)
			.filter(([, conversations]) => conversations.length > 0)
			.map(([label, conversations]) => ({
				label,
				conversations: sortThreadsByRecent(conversations)
			}));
	});

	// State derived stores
	const nlpState = derived(store, ($chat) => $chat.nlp);
	const llmState = derived(store, ($chat) => {
		if (!$chat.selectedThreadId) {
			return { isStreaming: false, requestId: null, streamingContent: '', streamingReasoning: '', model: null };
		}

		const stream = $chat.threadStreaming[$chat.selectedThreadId];
		if (!stream) {
			return { isStreaming: false, requestId: null, streamingContent: '', streamingReasoning: '', model: null };
		}

		return {
			isStreaming: stream.isStreaming,
			requestId: stream.requestId,
			streamingContent: stream.streamingContent,
			streamingReasoning: stream.streamingReasoning,
			model: stream.model
		};
	});
	const isLlmStreaming = derived(llmState, ($llmState) => $llmState.isStreaming);
	const streamingContent = derived(llmState, ($llmState) => $llmState.streamingContent);

	// Loading state derived stores
	const conversationsLoading = derived(store, ($chat) => $chat.conversationsLoading);
	const messagesLoading = derived(store, ($chat) => $chat.messagesLoading);
	const threadsLoading = derived(store, ($chat) => $chat.threadsLoading);
	const chatError = derived(store, ($chat) => $chat.error);
	const isLoading = derived(
		store,
		($chat) =>
			$chat.conversationsLoading === 'loading' ||
			$chat.messagesLoading === 'loading' ||
			$chat.threadsLoading === 'loading' ||
			$chat.nlp.processing
	);

	return {
		selectedConversation,
		selectedThread,
		currentThreads,
		activeStreamingThreadIds,
		unreadThreadIds,
		messageGroups,
		conversationGroups,
		nlpState,
		llmState,
		isLlmStreaming,
		streamingContent,
		conversationsLoading,
		messagesLoading,
		threadsLoading,
		chatError,
		isLoading
	};
}
