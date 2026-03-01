/**
 * Chat State Management
 * Svelte stores for managing conversations, messages, threads, and NLP state.
 */

import { writable, derived, get } from 'svelte/store';
import type { Conversation, Thread, ConversationGroup } from '$lib/types/thread';
import type { Message, MessageGroup } from '$lib/types/message';
import type { IntentClassification, Ambiguity } from '$lib/types/nlp';
import type { OperationType } from '$lib/types/config';
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
	createThreadWithMessage as apiCreateThreadWithMessage,
	type SendMessageOptions
} from '$lib/api/chat';
import {
	sendChatMessageStreaming,
	cancelChatRequest,
	isStartedEvent,
	isChunkEvent,
	isCompleteEvent,
	isErrorEvent,
	type LlmEvent,
	type StreamChatResult
} from '$lib/api/llm';
import { configStore } from './configStore';

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
 * LLM streaming state.
 */
export interface LlmStreamingState {
	/** Whether LLM is currently generating a response */
	isStreaming: boolean;
	/** Current request ID (if streaming) */
	requestId: string | null;
	/** Accumulated content during streaming */
	streamingContent: string;
	/** Model being used for current stream */
	model: string | null;
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
	/** LLM streaming state */
	llm: LlmStreamingState;
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
		llm: {
			isStreaming: false,
			requestId: null,
			streamingContent: '',
			model: null
		},
		error: null
	};

	const { subscribe, set, update } = writable<ChatState>(initialState);

	// Debounced intent classifier
	const classifier = createDebouncedClassifier(300);

	// Active stream tracking for cancellation
	let activeStream: StreamChatResult | null = null;

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
		 * Select a conversation and load its threads.
		 * If threads exist, auto-select the most recent one and load its messages.
		 * If no threads exist, show empty state prompting user to create a thread.
		 */
		async selectConversation(id: string | null) {
			update((state) => ({
				...state,
				selectedConversationId: id,
				selectedThreadId: null,
				messages: [],
				threads: [],
				messagesLoading: 'idle',
				threadsLoading: id ? 'loading' : 'idle',
				error: null
			}));

			if (id) {
				try {
					// Load threads first
					const threads = await loadThreads(id);

					// Update conversation with thread count for sidebar display
					update((state) => ({
						...state,
						threads,
						threadsLoading: 'success',
						conversations: state.conversations.map((c) =>
							c.id === id ? { ...c, threads } : c
						)
					}));

					// If there are threads, auto-select the most recent one
					if (threads.length > 0) {
						// Sort by updatedAt descending and get the most recent
						const sortedThreads = [...threads].sort(
							(a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
						);
						const mostRecentThread = sortedThreads[0];

						// Auto-select the most recent thread and load its messages
						await this.selectThread(mostRecentThread.id);
					}
				} catch (error) {
					update((state) => ({
						...state,
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
		 * Auto-selects the newly created thread.
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
					threads: [...s.threads, thread],
					// Update conversation with new thread count
					conversations: s.conversations.map((c) =>
						c.id === state.selectedConversationId
							? { ...c, threads: [...(c.threads || []), thread] }
							: c
					)
				}));

				// Auto-select the newly created thread
				await this.selectThread(thread.id);

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
		 * Send a message to the current conversation via backend.
		 * If no thread is selected and none exist, auto-creates a thread first.
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

			let targetThreadId = state.selectedThreadId;

			// If no thread selected, check if threads exist or create one
			if (!targetThreadId) {
				const existingThreads = state.threads.filter(
					(t) => t.conversationId === state.selectedConversationId
				);

				if (existingThreads.length === 0) {
					// No threads exist - create thread with message atomically
					const threadTitle = this.deriveThreadTitle(content);

					try {
						const { thread, message } = await apiCreateThreadWithMessage({
							conversationId: state.selectedConversationId,
							threadTitle,
							content,
							role
						});

						update((s) => ({
							...s,
							threads: [...s.threads, thread],
							selectedThreadId: thread.id,
							messages: [...s.messages, message]
						}));

						return message;
					} catch (error) {
						update((s) => ({
							...s,
							error:
								error instanceof Error ? error.message : 'Failed to create thread'
						}));
						return null;
					}
				} else {
					// Threads exist but none selected - select most recent
					const sortedThreads = [...existingThreads].sort(
						(a, b) =>
							new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
					);
					targetThreadId = sortedThreads[0].id;
					await this.selectThread(targetThreadId);
				}
			}

			// Create optimistic message with guaranteed threadId
			const optimisticMessage: Message = {
				id: `temp-${Date.now()}`,
				conversationId: state.selectedConversationId,
				threadId: targetThreadId,
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
					role,
					threadId: targetThreadId
				};

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
		 * Derives a thread title from message content.
		 * Uses first ~50 characters, cleaned up for display.
		 */
		deriveThreadTitle(content: string): string {
			const cleaned = content
				.replace(/[#*_`]/g, '')
				.replace(/\n/g, ' ')
				.trim();

			if (!cleaned) {
				return 'New Thread';
			}

			if (cleaned.length <= 50) {
				return cleaned;
			}

			const truncated = cleaned.substring(0, 50);
			const lastSpace = truncated.lastIndexOf(' ');

			return (lastSpace > 20 ? truncated.substring(0, lastSpace) : truncated) + '...';
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

		// ========================================================================
		// LLM Integration Methods
		// ========================================================================

		/**
		 * Send a message and get an LLM response with streaming.
		 * This combines message sending with LLM response generation.
		 *
		 * @param content - The user's message content
		 * @param options - Optional configuration for the LLM request
		 * @returns The user message that was sent, or null on error
		 */
		async sendMessageWithLlm(
			content: string,
			options?: {
				/** Model ID to use (e.g., "openai-gpt-4"). If not provided, uses default for 'code' operation */
				modelId?: string;
				/** System prompt override */
				systemPrompt?: string;
				/** Temperature override (0-1) */
				temperature?: number;
				/** Max tokens override */
				maxTokens?: number;
			}
		): Promise<Message | null> {
			const state = get({ subscribe });

			// Must have a conversation selected
			if (!state.selectedConversationId) {
				update((s) => ({ ...s, error: 'No conversation selected' }));
				return null;
			}

			// Must not already be streaming
			if (state.llm.isStreaming) {
				console.warn('Already streaming, ignoring new request');
				return null;
			}

			// Get the model to use
			let modelId: string | undefined = options?.modelId;
			if (!modelId) {
				// Try to get default model for 'code' operation
				const defaultModel = configStore.getDefault('code');
				if (defaultModel) {
					modelId = defaultModel;
				} else {
					// Fallback: get first available model for code
					const availableModels = configStore.getAvailableModels('code');
					if (availableModels.length === 0) {
						update((s) => ({
							...s,
							error: 'No model configured. Please configure a provider first.'
						}));
						return null;
					}
					modelId = availableModels[0].id;
				}
			}

			// Send the user message first (using existing logic)
			const userMessage = await this.sendMessage(content, 'user');
			if (!userMessage) {
				return null;
			}

			// Create optimistic assistant message for streaming
			const assistantMessageId = `streaming-${Date.now()}`;
			const optimisticAssistant: Message = {
				id: assistantMessageId,
				conversationId: state.selectedConversationId,
				threadId: state.selectedThreadId ?? '',
				role: 'assistant',
				content: '',
				timestamp: new Date(),
				status: 'sending'
			};

			// Add optimistic assistant message and start streaming
			update((s) => ({
				...s,
				messages: [...s.messages, optimisticAssistant],
				llm: {
					isStreaming: true,
					requestId: null,
					streamingContent: '',
					model: modelId ?? null
				}
			}));

			try {
				// Start streaming request
				const result = await sendChatMessageStreaming({
					conversationId: state.selectedConversationId,
					threadId: state.selectedThreadId ?? undefined,
					content,
					modelId: modelId!,
					systemPrompt: options?.systemPrompt,
					temperature: options?.temperature,
					maxTokens: options?.maxTokens,
					onEvent: (event: LlmEvent) => {
						if (isChunkEvent(event)) {
							// Update streaming content
							update((s) => {
								const newContent = s.llm.streamingContent + event.delta;
								return {
									...s,
									llm: {
										...s.llm,
										streamingContent: newContent
									},
									messages: s.messages.map((m) =>
										m.id === assistantMessageId
											? { ...m, content: newContent }
											: m
									)
								};
							});
						} else if (isCompleteEvent(event)) {
							// Streaming complete - update with final content
							update((s) => ({
								...s,
								messages: s.messages.map((m) =>
									m.id === assistantMessageId
										? {
												...m,
												content: event.content,
												status: 'sent' as const,
												metadata: event.usage
													? {
															tokens: event.usage.totalTokens,
															model: s.llm.model ?? undefined
														}
													: undefined
											}
										: m
								),
								llm: {
									isStreaming: false,
									requestId: null,
									streamingContent: '',
									model: null
								}
							}));
							// Clear active stream
							activeStream = null;
						} else if (isErrorEvent(event)) {
							// Error during streaming
							console.error('LLM streaming error:', event.error);
							update((s) => ({
								...s,
								messages: s.messages.map((m) =>
									m.id === assistantMessageId
										? { ...m, status: 'error' as const }
										: m
								),
								llm: {
									isStreaming: false,
									requestId: null,
									streamingContent: '',
									model: null
								},
								error: event.error
							}));
							activeStream = null;
						} else if (isStartedEvent(event)) {
							// Store request ID for cancellation
							update((s) => ({
								...s,
								llm: {
									...s.llm,
									requestId: event.requestId,
									model: event.model
								}
							}));
						}
					}
				});

				// Store active stream for cancellation
				activeStream = result;

				// Update with request ID
				update((s) => ({
					...s,
					llm: {
						...s.llm,
						requestId: result.requestId
					}
				}));

				return userMessage;
			} catch (error) {
				// Clean up on error
				update((s) => ({
					...s,
					messages: s.messages.map((m) =>
						m.id === assistantMessageId ? { ...m, status: 'error' as const } : m
					),
					llm: {
						isStreaming: false,
						requestId: null,
						streamingContent: '',
						model: null
					},
					error: error instanceof Error ? error.message : 'Failed to get LLM response'
				}));
				activeStream = null;
				return userMessage;
			}
		},

		/**
		 * Cancel the current LLM streaming request.
		 */
		async cancelLlmStream(): Promise<void> {
			if (!activeStream) return;

			try {
				await activeStream.cancel();
				activeStream.unsubscribe();
			} catch (error) {
				console.error('Failed to cancel LLM stream:', error);
			} finally {
				activeStream = null;
				update((s) => ({
					...s,
					llm: {
						isStreaming: false,
						requestId: null,
						streamingContent: '',
						model: null
					}
				}));
			}
		},

		/**
		 * Check if LLM is currently streaming.
		 */
		isLlmStreaming(): boolean {
			return get({ subscribe }).llm.isStreaming;
		},

		/**
		 * Get the current streaming content.
		 */
		getStreamingContent(): string {
			return get({ subscribe }).llm.streamingContent;
		},

		/**
		 * Clear error state.
		 */
		clearError() {
			update((state) => ({ ...state, error: null }));
		},

		/**
		 * Reset the entire store.
		 */
		reset() {
			classifier.cancel();
			// Cancel any active stream
			if (activeStream) {
				activeStream.cancel().catch(console.error);
				activeStream.unsubscribe();
				activeStream = null;
			}
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
export const selectedThread = derived(chatStore, ($chat) => {
	if (!$chat.selectedThreadId) return null;
	return $chat.threads.find((t) => t.id === $chat.selectedThreadId) ?? null;
});

/**
 * Threads for the current conversation.
 */
export const currentThreads = derived(chatStore, ($chat) => $chat.threads);

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
	threadId: string,
	role: 'user' | 'assistant' | 'system',
	content: string
): Message {
	return {
		id: generateId(),
		conversationId,
		threadId,
		role,
		content,
		timestamp: new Date(),
		status: 'sending'
	};
}
