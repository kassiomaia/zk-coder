/**
 * Chat Derived Stores
 *
 * Derived Svelte stores computed from the main chat store.
 */

import { derived } from 'svelte/store';
import type { ConversationGroup, Conversation } from '$lib/types/thread';
import type { MessageGroup, Message } from '$lib/types/message';
import { chatStore } from '../chat';
import { sortThreadsByRecent } from './utils';

// ============================================================================
// Selection Derived Stores
// ============================================================================

/**
 * Currently selected conversation.
 */
export const selectedConversation = derived(
	chatStore,
	($chat) => $chat.conversations.find((c) => c.id === $chat.selectedConversationId) ?? null
);

/**
 * Currently selected thread.
 */
export const selectedThread = derived(chatStore, ($chat) => {
	if (!$chat.selectedThreadId) return null;
	return $chat.threads.find((t) => t.id === $chat.selectedThreadId) ?? null;
});

/**
 * Threads for the current conversation.
 */
export const currentThreads = derived(chatStore, ($chat) => $chat.threads);

// ============================================================================
// Message Display Derived Stores
// ============================================================================

/**
 * Messages grouped by time (for ChatArea display).
 */
export const messageGroups = derived(chatStore, ($chat): MessageGroup[] => {
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

// ============================================================================
// Sidebar Derived Stores
// ============================================================================

/**
 * Conversations grouped by time for sidebar.
 */
export const conversationGroups = derived(chatStore, ($chat): ConversationGroup[] => {
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

// ============================================================================
// State Derived Stores
// ============================================================================

/**
 * NLP state derived store.
 */
export const nlpState = derived(chatStore, ($chat) => $chat.nlp);

/**
 * LLM streaming state derived store.
 */
export const llmState = derived(chatStore, ($chat) => $chat.llm);

/**
 * Whether LLM is currently streaming.
 */
export const isLlmStreaming = derived(chatStore, ($chat) => $chat.llm.isStreaming);

/**
 * Current streaming content (for real-time display).
 */
export const streamingContent = derived(chatStore, ($chat) => $chat.llm.streamingContent);

// ============================================================================
// Loading State Derived Stores
// ============================================================================

/**
 * Conversations loading state.
 */
export const conversationsLoading = derived(chatStore, ($chat) => $chat.conversationsLoading);

/**
 * Messages loading state.
 */
export const messagesLoading = derived(chatStore, ($chat) => $chat.messagesLoading);

/**
 * Threads loading state.
 */
export const threadsLoading = derived(chatStore, ($chat) => $chat.threadsLoading);

/**
 * Global error state.
 */
export const chatError = derived(chatStore, ($chat) => $chat.error);

/**
 * Is any loading in progress.
 */
export const isLoading = derived(
	chatStore,
	($chat) =>
		$chat.conversationsLoading === 'loading' ||
		$chat.messagesLoading === 'loading' ||
		$chat.threadsLoading === 'loading' ||
		$chat.nlp.processing
);
