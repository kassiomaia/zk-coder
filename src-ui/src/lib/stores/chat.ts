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
import {
	loadConversations,
	createConversation as apiCreateConversation,
	loadConversation,
	renameConversation as apiRenameConversation,
	deleteConversation as apiDeleteConversation,
	loadThreads,
	createThread as apiCreateThread,
	resolveThread as apiResolveThread,
	deleteThread as apiDeleteThread,
	loadMessages,
	loadThreadMessages,
	sendMessage as apiSendMessage,
	deleteMessage as apiDeleteMessage,
	updateMessageStatus as apiUpdateMessageStatus,
	type SendMessageOptions
} from '$lib/api/chat';

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
	/** Loading state for conversations */
	conversationsLoading: LoadingState;
	/** Currently selected conversation ID */
	selectedConversationId: string | null;
	/** Currently selected thread ID */
	selectedThreadId: string | null;
	/** Messages for the current view */
	messages: Message[];
	/** Loading state for messages */
	messagesLoading: LoadingState;
	/** Threads for current conversation */
	threads: Thread[];
	/** Loading state for threads */
	threadsLoading: LoadingState;
	/** NLP state */
	nlp: NlpState;
	/** Global error message */
	error: string | null;
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
		conversationsLoading: 'idle',
		selectedConversationId: null,
		selectedThreadId: null,
		messages: [],
		messagesLoading: 'idle',
		threads: [],
		threadsLoading: 'idle',
		nlp: {
			backendAvailable: false,
			detectedIntent: null,
			ambiguities: [],
			processing: false,
			error: null
		},
		error: null
	};

	const { subscribe, set, update } = writable<ChatState>(initialState);

	// Debounced intent classifier
	const classifier = createDebouncedClassifier(300);

	return {
		subscribe,

		/**
		 * Initialize the store, check backend availability, and load conversations.
		 */
		async initialize() {
			const available = await isBackendAvailable();
			update((state) => ({
				...state,
				nlp: { ...state.nlp, backendAvailable: available }
			}));

			// Load conversations from backend
			await this.loadConversationsFromBackend();
		},

		/**
		 * Load all conversations from the backend.
		 */
		async loadConversationsFromBackend() {
			update((state) => ({
				...state,
				conversationsLoading: 'loading',
				error: null
			}));

			try {
				const conversations = await loadConversations();
				update((state) => ({
					...state,
					conversations,
					conversationsLoading: 'success'
				}));
			} catch (error) {
				update((state) => ({
					...state,
					conversationsLoading: 'error',
					error: error instanceof Error ? error.message : 'Failed to load conversations'
				}));
			}
		},

		/**
		 * Set all conversations.
		 */
		setConversations(conversations: Conversation[]) {
			update((state) => ({ ...state, conversations }));
		},

		/**
		 * Create a new conversation in the backend.
		 */
		async createConversation(title: string = 'New Conversation'): Promise<Conversation | null> {
			try {
				const conversation = await apiCreateConversation(title);
				update((state) => ({
					...state,
					conversations: [conversation, ...state.conversations]
				}));
				return conversation;
			} catch (error) {
				update((state) => ({
					...state,
					error: error instanceof Error ? error.message : 'Failed to create conversation'
				}));
				return null;
			}
		},

		/**
		 * Add a new conversation (local only, for backward compatibility).
		 */
		addConversation(conversation: Conversation) {
			update((state) => ({
				...state,
				conversations: [conversation, ...state.conversations]
			}));
		},

		/**
		 * Rename a conversation.
		 */
		async renameConversation(id: string, title: string): Promise<boolean> {
			try {
				const updated = await apiRenameConversation(id, title);
				update((state) => ({
					...state,
					conversations: state.conversations.map((c) =>
						c.id === id ? { ...c, title: updated.title, updatedAt: updated.updatedAt } : c
					)
				}));
				return true;
			} catch (error) {
				update((state) => ({
					...state,
					error: error instanceof Error ? error.message : 'Failed to rename conversation'
				}));
				return false;
			}
		},

		/**
		 * Update a conversation (local only).
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
		async deleteConversation(id: string): Promise<boolean> {
			try {
				await apiDeleteConversation(id);
				update((state) => ({
					...state,
					conversations: state.conversations.filter((c) => c.id !== id),
					selectedConversationId:
						state.selectedConversationId === id ? null : state.selectedConversationId,
					messages: state.selectedConversationId === id ? [] : state.messages,
					threads: state.selectedConversationId === id ? [] : state.threads
				}));
				return true;
			} catch (error) {
				update((state) => ({
					...state,
					error: error instanceof Error ? error.message : 'Failed to delete conversation'
				}));
				return false;
			}
		},

		/**
		 * Select a conversation and load its messages and threads.
		 */
		async selectConversation(id: string | null) {
			update((state) => ({
				...state,
				selectedConversationId: id,
				selectedThreadId: null,
				messages: [],
				threads: [],
				messagesLoading: id ? 'loading' : 'idle',
				threadsLoading: id ? 'loading' : 'idle',
				error: null
			}));

			if (id) {
				try {
					// Load messages from backend persistence
					const [messages, threads] = await Promise.all([
						loadMessages(id),
						loadThreads(id)
					]);

					update((state) => ({
						...state,
						messages,
						threads,
						messagesLoading: 'success',
						threadsLoading: 'success'
					}));
				} catch (error) {
					update((state) => ({
						...state,
						messagesLoading: 'error',
						threadsLoading: 'error',
						error: error instanceof Error ? error.message : 'Failed to load conversation'
					}));
				}
			}
		},

		/**
		 * Select a thread and load its messages.
		 */
		async selectThread(id: string | null) {
			update((state) => ({
				...state,
				selectedThreadId: id,
				messages: [],
				messagesLoading: id ? 'loading' : 'idle'
			}));

			if (id) {
				try {
					const messages = await loadThreadMessages(id);
					update((state) => ({
						...state,
						messages,
						messagesLoading: 'success'
					}));
				} catch (error) {
					update((state) => ({
						...state,
						messagesLoading: 'error',
						error: error instanceof Error ? error.message : 'Failed to load thread messages'
					}));
				}
			}
		},

		/**
		 * Create a new thread in the current conversation.
		 */
		async createThread(title: string = 'New Thread'): Promise<Thread | null> {
			const state = get({ subscribe });
			if (!state.selectedConversationId) {
				update((s) => ({ ...s, error: 'No conversation selected' }));
				return null;
			}

			try {
				const thread = await apiCreateThread(state.selectedConversationId, title);
				update((s) => ({
					...s,
					threads: [...s.threads, thread]
				}));
				return thread;
			} catch (error) {
				update((s) => ({
					...s,
					error: error instanceof Error ? error.message : 'Failed to create thread'
				}));
				return null;
			}
		},

		/**
		 * Resolve or unresolve a thread.
		 */
		async resolveThread(id: string, resolved: boolean): Promise<boolean> {
			try {
				await apiResolveThread(id, resolved);
				update((state) => ({
					...state,
					threads: state.threads.map((t) =>
						t.id === id
							? { ...t, metadata: { ...t.metadata, resolved }, updatedAt: new Date() }
							: t
					)
				}));
				return true;
			} catch (error) {
				update((state) => ({
					...state,
					error: error instanceof Error ? error.message : 'Failed to update thread'
				}));
				return false;
			}
		},

		/**
		 * Delete a thread.
		 */
		async deleteThread(id: string): Promise<boolean> {
			try {
				await apiDeleteThread(id);
				update((state) => ({
					...state,
					threads: state.threads.filter((t) => t.id !== id),
					selectedThreadId: state.selectedThreadId === id ? null : state.selectedThreadId
				}));
				return true;
			} catch (error) {
				update((state) => ({
					...state,
					error: error instanceof Error ? error.message : 'Failed to delete thread'
				}));
				return false;
			}
		},

		/**
		 * Set messages directly.
		 */
		setMessages(messages: Message[]) {
			update((state) => ({ ...state, messages }));
		},

		/**
		 * Send a message to the current conversation via backend.
		 */
		async sendMessage(
			content: string,
			role: 'user' | 'assistant' | 'system' = 'user'
		): Promise<Message | null> {
			const state = get({ subscribe });
			if (!state.selectedConversationId) {
				update((s) => ({ ...s, error: 'No conversation selected' }));
				return null;
			}

			// Create optimistic message
			const optimisticMessage: Message = {
				id: `temp-${Date.now()}`,
				conversationId: state.selectedConversationId,
				role,
				content,
				timestamp: new Date(),
				status: 'sending'
			};

			// Add optimistically
			update((s) => ({
				...s,
				messages: [...s.messages, optimisticMessage]
			}));

			try {
				const options: SendMessageOptions = {
					conversationId: state.selectedConversationId,
					content,
					role
				};

				// Add thread ID if viewing a thread
				if (state.selectedThreadId) {
					options.threadId = state.selectedThreadId;
				}

				const savedMessage = await apiSendMessage(options);

				// Replace optimistic message with saved one
				update((s) => ({
					...s,
					messages: s.messages.map((m) =>
						m.id === optimisticMessage.id ? savedMessage : m
					)
				}));

				return savedMessage;
			} catch (error) {
				// Update optimistic message to error state
				update((s) => ({
					...s,
					messages: s.messages.map((m) =>
						m.id === optimisticMessage.id ? { ...m, status: 'error' as const } : m
					),
					error: error instanceof Error ? error.message : 'Failed to send message'
				}));
				return null;
			}
		},

		/**
		 * Add a message to the current conversation (local only, for backward compatibility).
		 */
		async addMessage(message: Message) {
			update((state) => ({
				...state,
				messages: [...state.messages, message]
			}));

			// Sync with backend context if available (for NLP context)
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
		 * Delete a message.
		 */
		async deleteMessage(id: string): Promise<boolean> {
			try {
				await apiDeleteMessage(id);
				update((state) => ({
					...state,
					messages: state.messages.filter((m) => m.id !== id)
				}));
				return true;
			} catch (error) {
				update((state) => ({
					...state,
					error: error instanceof Error ? error.message : 'Failed to delete message'
				}));
				return false;
			}
		},

		/**
		 * Update a message status.
		 */
		async updateMessageStatus(
			id: string,
			status: 'sending' | 'sent' | 'delivered' | 'error'
		): Promise<boolean> {
			try {
				await apiUpdateMessageStatus(id, status);
				update((state) => ({
					...state,
					messages: state.messages.map((m) => (m.id === id ? { ...m, status } : m))
				}));
				return true;
			} catch (error) {
				console.error('Failed to update message status:', error);
				return false;
			}
		},

		/**
		 * Update a message (local only).
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
