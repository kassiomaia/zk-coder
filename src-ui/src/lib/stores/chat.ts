/**
 * Chat State Management
 * Svelte stores for managing conversations, messages, threads, and NLP state.
 */

import { writable, derived, get } from 'svelte/store';
import type { Conversation, Thread, ConversationGroup } from '$lib/types/thread';
import type { Message, MessageGroup } from '$lib/types/message';
import type { IntentClassification, Ambiguity } from '$lib/types/nlp';
import {
	classifyIntent,
	addContextMessage,
	getConversationContext,
	detectAmbiguities,
	createDebouncedClassifier,
	isBackendAvailable
} from '$lib/api/nlp';

// ============================================================================
// Types
// ============================================================================

/**
 * Loading states for async operations.
 */
export type LoadingState = 'idle' | 'loading' | 'success' | 'error';

/**
 * NLP state for the current input.
 */
export interface NlpState {
	/** Whether backend is available */
	backendAvailable: boolean;
	/** Currently detected intent */
	detectedIntent: IntentClassification | null;
	/** Detected ambiguities */
	ambiguities: Ambiguity[];
	/** Whether NLP is processing */
	processing: boolean;
	/** Last error */
	error: string | null;
}

/**
 * Chat store state.
 */
export interface ChatState {
	/** All conversations */
	conversations: Conversation[];
	/** Currently selected conversation ID */
	selectedConversationId: string | null;
	/** Currently selected thread ID */
	selectedThreadId: string | null;
	/** Messages for the current view */
	messages: Message[];
	/** Loading state for messages */
	messagesLoading: LoadingState;
	/** NLP state */
	nlp: NlpState;
}

// ============================================================================
// Stores
// ============================================================================

/**
 * Main chat store.
 */
function createChatStore() {
	const initialState: ChatState = {
		conversations: [],
		selectedConversationId: null,
		selectedThreadId: null,
		messages: [],
		messagesLoading: 'idle',
		nlp: {
			backendAvailable: false,
			detectedIntent: null,
			ambiguities: [],
			processing: false,
			error: null
		}
	};

	const { subscribe, set, update } = writable<ChatState>(initialState);

	// Debounced intent classifier
	const classifier = createDebouncedClassifier(300);

	return {
		subscribe,

		/**
		 * Initialize the store and check backend availability.
		 */
		async initialize() {
			const available = await isBackendAvailable();
			update((state) => ({
				...state,
				nlp: { ...state.nlp, backendAvailable: available }
			}));
		},

		/**
		 * Set all conversations.
		 */
		setConversations(conversations: Conversation[]) {
			update((state) => ({ ...state, conversations }));
		},

		/**
		 * Add a new conversation.
		 */
		addConversation(conversation: Conversation) {
			update((state) => ({
				...state,
				conversations: [conversation, ...state.conversations]
			}));
		},

		/**
		 * Update a conversation.
		 */
		updateConversation(id: string, updates: Partial<Conversation>) {
			update((state) => ({
				...state,
				conversations: state.conversations.map((c) => (c.id === id ? { ...c, ...updates } : c))
			}));
		},

		/**
		 * Delete a conversation.
		 */
		deleteConversation(id: string) {
			update((state) => ({
				...state,
				conversations: state.conversations.filter((c) => c.id !== id),
				selectedConversationId:
					state.selectedConversationId === id ? null : state.selectedConversationId
			}));
		},

		/**
		 * Select a conversation and load its messages.
		 */
		async selectConversation(id: string | null) {
			update((state) => ({
				...state,
				selectedConversationId: id,
				selectedThreadId: null,
				messages: [],
				messagesLoading: id ? 'loading' : 'idle'
			}));

			if (id) {
				try {
					// Load messages from backend context
					const context = await getConversationContext(id);
					const messages: Message[] = context.recent_messages.map((msg) => ({
						id: msg.id,
						conversationId: id,
						role: msg.role,
						content: msg.content,
						timestamp: new Date(msg.timestamp),
						status: 'sent' as const
					}));

					update((state) => ({
						...state,
						messages,
						messagesLoading: 'success'
					}));
				} catch (error) {
					update((state) => ({
						...state,
						messagesLoading: 'error',
						nlp: {
							...state.nlp,
							error: error instanceof Error ? error.message : 'Failed to load messages'
						}
					}));
				}
			}
		},

		/**
		 * Select a thread.
		 */
		selectThread(id: string | null) {
			update((state) => ({
				...state,
				selectedThreadId: id
			}));
		},

		/**
		 * Set messages directly.
		 */
		setMessages(messages: Message[]) {
			update((state) => ({ ...state, messages }));
		},

		/**
		 * Add a message to the current conversation.
		 */
		async addMessage(message: Message) {
			update((state) => ({
				...state,
				messages: [...state.messages, message]
			}));

			// Sync with backend if available
			const state = get({ subscribe });
			if (state.nlp.backendAvailable && message.conversationId) {
				try {
					await addContextMessage({
						conversationId: message.conversationId,
						role: message.role,
						content: message.content
					});
				} catch (error) {
					console.error('Failed to sync message with backend:', error);
				}
			}
		},

		/**
		 * Update a message.
		 */
		updateMessage(id: string, updates: Partial<Message>) {
			update((state) => ({
				...state,
				messages: state.messages.map((m) => (m.id === id ? { ...m, ...updates } : m))
			}));
		},

		/**
		 * Classify intent for input text.
		 */
		async classifyInput(input: string) {
			const state = get({ subscribe });
			if (!state.nlp.backendAvailable || !state.selectedConversationId) {
				return;
			}

			update((s) => ({
				...s,
				nlp: { ...s.nlp, processing: true, error: null }
			}));

			try {
				const result = await classifier.classify({
					input,
					conversationId: state.selectedConversationId
				});

				update((s) => ({
					...s,
					nlp: {
						...s.nlp,
						detectedIntent: result,
						processing: false
					}
				}));
			} catch (error) {
				update((s) => ({
					...s,
					nlp: {
						...s.nlp,
						processing: false,
						error: error instanceof Error ? error.message : 'Classification failed'
					}
				}));
			}
		},

		/**
		 * Detect ambiguities for input text.
		 */
		async detectAmbiguitiesForInput(input: string) {
			const state = get({ subscribe });
			if (!state.nlp.backendAvailable || !state.selectedConversationId) {
				return;
			}

			try {
				const ambiguities = await detectAmbiguities({
					input,
					conversationId: state.selectedConversationId
				});

				update((s) => ({
					...s,
					nlp: { ...s.nlp, ambiguities }
				}));
			} catch (error) {
				console.error('Failed to detect ambiguities:', error);
			}
		},

		/**
		 * Clear NLP state.
		 */
		clearNlpState() {
			classifier.cancel();
			update((state) => ({
				...state,
				nlp: {
					...state.nlp,
					detectedIntent: null,
					ambiguities: [],
					processing: false,
					error: null
				}
			}));
		},

		/**
		 * Reset the entire store.
		 */
		reset() {
			classifier.cancel();
			set(initialState);
		}
	};
}

// Export the store instance
export const chatStore = createChatStore();

// ============================================================================
// Derived Stores
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
export const selectedThread = derived(
	[chatStore, selectedConversation],
	([$chat, $conversation]) => {
		if (!$chat.selectedThreadId || !$conversation) return null;
		return $conversation.threads.find((t) => t.id === $chat.selectedThreadId) ?? null;
	}
);

/**
 * Threads for the current conversation.
 */
export const currentThreads = derived(
	selectedConversation,
	($conversation) => $conversation?.threads ?? []
);

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

		// Start new group if time gap exceeds threshold
		if (lastTime > 0 && messageTime - lastTime > GROUP_INTERVAL_MS) {
			if (currentGroup.length > 0) {
				groups.push({ messages: currentGroup });
			}
			currentGroup = [];
		}

		currentGroup.push(message);
		lastTime = messageTime;
	}

	// Add final group
	if (currentGroup.length > 0) {
		groups.push({ messages: currentGroup });
	}

	return groups;
});

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
			conversations: conversations.sort(
				(a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
			)
		}));
});

/**
 * NLP state derived store.
 */
export const nlpState = derived(chatStore, ($chat) => $chat.nlp);

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Generates a unique ID.
 */
export function generateId(): string {
	return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Creates a new conversation object.
 */
export function createNewConversation(title: string = 'New Conversation'): Conversation {
	const now = new Date();
	return {
		id: generateId(),
		title,
		createdAt: now,
		updatedAt: now,
		threads: [],
		settings: {
			model: 'claude-3-sonnet',
			temperature: 0.7,
			maxTokens: 4096
		}
	};
}

/**
 * Creates a new thread object.
 */
export function createNewThread(conversationId: string, title: string = 'New Thread'): Thread {
	const now = new Date();
	return {
		id: generateId(),
		conversationId,
		parentId: null,
		title,
		createdAt: now,
		updatedAt: now,
		messageCount: 0,
		metadata: {
			resolved: false
		}
	};
}

/**
 * Creates a new message object.
 */
export function createNewMessage(
	conversationId: string,
	role: 'user' | 'assistant' | 'system',
	content: string
): Message {
	return {
		id: generateId(),
		conversationId,
		role,
		content,
		timestamp: new Date(),
		status: 'sending'
	};
}
