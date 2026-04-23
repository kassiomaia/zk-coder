/**
 * Chat State Management
 *
 * Main Svelte store for managing conversations, messages, threads, and NLP state.
 * LLM streaming logic extracted to ./chat/llm-handler.ts
 */

import { get, writable } from 'svelte/store';
import type { Thread } from '$lib/types/thread';
import type { Message } from '$lib/types/message';
import { createDebouncedClassifier, isBackendAvailable } from '$lib/api/nlp';
import {
	createThread as apiCreateThread,
	createThreadWithMessage as apiCreateThreadWithMessage,
	loadConversations,
	loadThreadMessages,
	loadThreads,
	sendMessage as apiSendMessage,
	setThreadAgentSession
} from '$lib/api/chat';
import { LlmHandler } from './chat/llm-handler';
import { createConversationMethods } from './chat/conversation-methods';
import { createMessageMethods } from './chat/message-methods';
import { createNlpMethods } from './chat/nlp-methods';
import { createThreadMethods } from './chat/thread-methods';
import { createUtilityMethods } from './chat/utility-methods';
import type { ChatState, LlmRequestOptions } from './chat/types';
import { deriveThreadTitle, sortThreadsByRecent, touchThreadAndConversation } from './chat/utils';
import { clearUnreadForThread } from '$lib/utils/state';
import { agentStore } from './agentStore';
import { configStore } from './configStore';
import { createDerivedStores } from './chat/derived-stores';
import { documentGenerationStore } from './documentGenerationStore';
import { createLogger } from '$lib/utils/logger';
import { NLP_DEBOUNCE_MS } from '$lib/config/runtime';

const log = createLogger('chatStore');

// ============================================================================
// Initial State
// ============================================================================

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
		pendingAmbiguities: [],
		resolvedAmbiguities: new Map(),
		processing: false,
		error: null
	},
	llm: {
		isStreaming: false,
		requestId: null,
		streamingContent: '',
		streamingReasoning: '',
		model: null
	},
	threadStreaming: {},
	unreadThreadIds: [],
	error: null
};

// ============================================================================
// Store Factory
// ============================================================================

/**
 * Main chat store.
 */
function createChatStore() {
	const { subscribe, set, update } = writable<ChatState>(initialState);

	// Debounced intent classifier
	const classifier = createDebouncedClassifier(NLP_DEBOUNCE_MS);

	// LLM handler for streaming operations
	const llmHandler = new LlmHandler(update, () => get({ subscribe }));

	// NLP methods (classification / ambiguity detection / resolution)
	const nlp = createNlpMethods({
		update,
		getState: () => get({ subscribe }),
		classifier
	});

	// Conversation CRUD methods (no cross-domain orchestration)
	const conversationMethods = createConversationMethods({
		update,
		getState: () => get({ subscribe })
	});

	// Thread CRUD methods (no cross-domain orchestration)
	const threadMethods = createThreadMethods({
		update,
		getState: () => get({ subscribe })
	});

	// Message CRUD methods (non-send path)
	const messageMethods = createMessageMethods({
		update,
		getState: () => get({ subscribe })
	});

	// Small utilities that touch a single state field
	const utilityMethods = createUtilityMethods({ update });

	// Pending thread creation tracking to prevent race conditions
	let pendingThreadReady: Promise<Thread | null> | null = null;
	let pendingThreadConversationId: string | null = null;

	return {
		subscribe,

		// ====================================================================
		// Initialization
		// ====================================================================

		/**
		 * Initialize the store, check backend availability, and load conversations.
		 */
		async initialize() {
			const available = await isBackendAvailable();
			update((state) => ({
				...state,
				nlp: { ...state.nlp, backendAvailable: available }
			}));
			await this.loadConversationsFromBackend();
		},

		/**
		 * Load all conversations from the backend.
		 */
		async loadConversationsFromBackend() {
			update((state) => ({ ...state, conversationsLoading: 'loading', error: null }));

			try {
				const conversations = await loadConversations();
				update((state) => ({ ...state, conversations, conversationsLoading: 'success' }));
			} catch (error) {
				update((state) => ({
					...state,
					conversationsLoading: 'error',
					error: error instanceof Error ? error.message : 'Failed to load conversations'
				}));
			}
		},

		// ====================================================================
		// Conversation Methods (implemented in ./chat/conversation-methods.ts)
		// ====================================================================

		...conversationMethods,

		/**
		 * Select a conversation and load its threads.
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
					const threads = await loadThreads(id);
					update((state) => ({
						...state,
						threads,
						threadsLoading: 'success',
						conversations: state.conversations.map((c) => (c.id === id ? { ...c, threads } : c))
					}));

					const selectableThreads = threads.filter((thread) => !thread.metadata.archived);
					if (selectableThreads.length > 0) {
						const mostRecentThread = sortThreadsByRecent(selectableThreads)[0];
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

		// ====================================================================
		// Thread Methods
		// ====================================================================

		/**
		 * Select a thread and load its messages.
		 */
		async selectThread(id: string | null) {
			if (id !== null) {
				const currentState = get({ subscribe });
				if (
					currentState.threadsLoading === 'success' &&
					!currentState.threads.some((t) => t.id === id)
				) {
					agentStore.setActiveSession(null);
					update((state) => ({
						...state,
						selectedThreadId: null,
						unreadThreadIds: clearUnreadForThread(state.unreadThreadIds, null),
						messages: [],
						messagesLoading: 'idle',
						error: 'Thread no longer exists'
					}));
					return;
				}
			}

			update((state) => ({
				...state,
				selectedThreadId: id,
				unreadThreadIds: clearUnreadForThread(state.unreadThreadIds, id),
				messages: [],
				messagesLoading: id ? 'loading' : 'idle'
			}));

			if (id) {
				// Check if thread has an agent session and activate it
				const thread = get({ subscribe }).threads.find((t) => t.id === id);
				let pausedSessionRecoveryMessage: Message | null = null;

				log.info('selectThread:', {
					threadId: id,
					hasAgentSession: !!thread?.metadata.agentSessionId,
					agentSessionId: thread?.metadata.agentSessionId
				});

				if (thread?.metadata.agentSessionId) {
					// Load the session from backend before setting as active
					const session = await agentStore.loadSession(thread.metadata.agentSessionId);
					log.info('Loaded session:', session ? { id: session.id, state: session.state } : null);
					if (session) {
						agentStore.setActiveSession(thread.metadata.agentSessionId);

						// Check if session is paused and create recovery message
						if (session.state.type === 'paused') {
							log.info('Creating recovery message for paused session');
							pausedSessionRecoveryMessage = {
								id: `recovery-${session.id}`,
								conversationId: thread.conversationId,
								threadId: id,
								role: 'assistant',
								kind: 'recovery',
								content: session.state.message,
								timestamp: new Date(session.state.pausedAt),
								agentData: {
									sessionId: session.id,
									recoveryResolved: false
								}
							};
						}
					}
				} else {
					// Thread has no agent session - clear active session
					agentStore.setActiveSession(null);
				}

				try {
					let loadedMessages = await loadThreadMessages(id);

					// Add recovery message if session is paused (avoid duplicates)
					if (pausedSessionRecoveryMessage) {
						const hasRecoveryMessage = loadedMessages.some(
							(m) => m.id === pausedSessionRecoveryMessage?.id
						);
						if (!hasRecoveryMessage) {
							loadedMessages = [...loadedMessages, pausedSessionRecoveryMessage];
						}
					}

					// Replay tool-call history into doc-gen tracker once, in a batched pass.
					// Replaces a per-render O(N) reactive scan that used to run inside
					// ChatMessagesList on every streaming tick.
					documentGenerationStore.rebuildFromMessages(loadedMessages);

					update((state) => {
						const threadStream = state.threadStreaming[id];
						const sessionWasRecovered = !!pausedSessionRecoveryMessage;

						if (!threadStream?.isStreaming || sessionWasRecovered) {
							const nextThreadStreaming = { ...state.threadStreaming };
							if (sessionWasRecovered) {
								delete nextThreadStreaming[id];
							}
							return {
								...state,
								threadStreaming: nextThreadStreaming,
								unreadThreadIds: clearUnreadForThread(state.unreadThreadIds, id),
								messages: loadedMessages,
								messagesLoading: 'success'
							};
						}

						const hasStreamingMessage = loadedMessages.some(
							(message) => message.id === threadStream.assistantMessageId
						);

						const streamingMessage: Message = {
							id: threadStream.assistantMessageId,
							conversationId: threadStream.conversationId,
							threadId: id,
							role: 'assistant',
							content: threadStream.streamingContent,
							timestamp: new Date(),
							status: 'sending'
						};

						return {
							...state,
							unreadThreadIds: clearUnreadForThread(state.unreadThreadIds, id),
							messages: hasStreamingMessage
								? loadedMessages.map((message) =>
										message.id === threadStream.assistantMessageId
											? {
													...message,
													content: threadStream.streamingContent,
													status: 'sending' as const
												}
											: message
									)
								: [...loadedMessages, streamingMessage],
							messagesLoading: 'success'
						};
					});
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
		 * Automatically creates an agent session for the thread.
		 */
		async createThread(title: string = 'New Thread'): Promise<Thread | null> {
			const state = get({ subscribe });
			if (!state.selectedConversationId) {
				update((s) => ({ ...s, error: 'No conversation selected' }));
				return null;
			}

			const conversationId = state.selectedConversationId;
			const conversation = state.conversations.find((c) => c.id === conversationId);
			const workspaceRoot = conversation?.settings?.workspaceRoot;
			const readyPromise = (async () => {
				try {
					const thread = await apiCreateThread(conversationId, title);

					// Auto-create an agent session for this thread
					try {
						const codingModelId = configStore.getDefault('code');
						if (codingModelId) {
							await this.updateConversationSettings(conversationId, {
								model: codingModelId
							});
						}

						const agentSession = await agentStore.createSession(
							'code-agent', // Default to code agent
							conversationId,
							thread.id,
							workspaceRoot // Inherit workspace from conversation
						);

						if (agentSession) {
							// Update thread metadata with agent session ID
							await setThreadAgentSession(thread.id, agentSession.id);
							thread.metadata.agentSessionId = agentSession.id;

							// Open the agent panel
							agentStore.setActiveSession(agentSession.id);
						}
					} catch (agentError) {
						log.warn('Failed to create agent session:', agentError);
						// Continue without agent - thread can still work as a regular chat
					}

					update((s) => ({
						...s,
						threads: [...s.threads, thread],
						conversations: s.conversations.map((c) =>
							c.id === conversationId ? { ...c, threads: [...(c.threads || []), thread] } : c
						)
					}));
					await this.selectThread(thread.id);
					return thread;
				} catch (error) {
					update((s) => ({
						...s,
						error: error instanceof Error ? error.message : 'Failed to create thread'
					}));
					return null;
				}
			})();

			pendingThreadReady = readyPromise;
			pendingThreadConversationId = conversationId;

			try {
				return await readyPromise;
			} finally {
				pendingThreadReady = null;
				pendingThreadConversationId = null;
			}
		},

		// Simple thread CRUD (implemented in ./chat/thread-methods.ts)
		...threadMethods,

		// ====================================================================
		// Message Methods
		// ====================================================================

		/**
		 * Send a message to the current conversation via backend.
		 */
		async sendMessage(
			content: string,
			role: 'user' | 'assistant' | 'system' = 'user'
		): Promise<Message | null> {
			let state = get({ subscribe });
			if (!state.selectedConversationId) {
				update((s) => ({ ...s, error: 'No conversation selected' }));
				return null;
			}

			const conversationId = state.selectedConversationId;

			// Wait for any pending thread creation
			if (pendingThreadReady && pendingThreadConversationId === conversationId) {
				try {
					await pendingThreadReady;
				} catch (error) {
					log.warn('Pending thread creation failed:', error);
				}
				state = get({ subscribe });
			}

			let targetThreadId = state.selectedThreadId;

			// Handle thread creation/selection if needed
			if (!targetThreadId) {
				const existingThreads = state.threads.filter((t) => t.conversationId === conversationId);

				if (existingThreads.length === 0) {
					const threadTitle = deriveThreadTitle(content);
					try {
						const { thread, message } = await apiCreateThreadWithMessage({
							conversationId,
							threadTitle,
							content,
							role
						});

						// Auto-create an agent session for this thread
						try {
							// Get the conversation's workspace if set
							const conversation = state.conversations.find((c) => c.id === conversationId);
							const workspaceRoot = conversation?.settings.workspaceRoot;

							const agentSession = await agentStore.createSession(
								'code-agent',
								conversationId,
								thread.id,
								workspaceRoot
							);

							if (agentSession) {
								await setThreadAgentSession(thread.id, agentSession.id);
								thread.metadata.agentSessionId = agentSession.id;
								agentStore.setActiveSession(agentSession.id);
							}
						} catch (agentError) {
							log.warn('Failed to create agent session:', agentError);
						}

						update((s) => ({
							...s,
							threads: [...s.threads, thread],
							selectedThreadId: thread.id,
							messages: [...s.messages, message],
							conversations: s.conversations.map((c) =>
								c.id === conversationId ? { ...c, threads: [...(c.threads || []), thread] } : c
							)
						}));

						return message;
					} catch (error) {
						update((s) => ({
							...s,
							error: error instanceof Error ? error.message : 'Failed to create thread'
						}));
						return null;
					}
				} else {
					const sortedThreads = sortThreadsByRecent(existingThreads);
					targetThreadId = sortedThreads[0].id;
					await this.selectThread(targetThreadId);
				}
			}

			if (!targetThreadId) {
				update((s) => ({ ...s, error: 'No thread selected' }));
				return null;
			}

			// Create optimistic message
			const optimisticMessage: Message = {
				id: `temp-${Date.now()}`,
				conversationId,
				threadId: targetThreadId,
				role,
				content,
				timestamp: new Date(),
				status: 'sending'
			};

			update((s) => {
				const touched = touchThreadAndConversation(s, targetThreadId, conversationId, {
					messageDelta: 1
				});
				return {
					...touched,
					unreadThreadIds: clearUnreadForThread(touched.unreadThreadIds, targetThreadId),
					messages: [...touched.messages, optimisticMessage]
				};
			});

			try {
				const savedMessage = await apiSendMessage({
					conversationId,
					content,
					role,
					threadId: targetThreadId
				});

				update((s) => ({
					...s,
					messages: s.messages.map((m) => (m.id === optimisticMessage.id ? savedMessage : m))
				}));

				return savedMessage;
			} catch (error) {
				update((s) => {
					const rolledBack = touchThreadAndConversation(s, targetThreadId, conversationId, {
						messageDelta: -1
					});
					return {
						...rolledBack,
						messages: rolledBack.messages.map((m) =>
							m.id === optimisticMessage.id ? { ...m, status: 'error' as const } : m
						),
						error: error instanceof Error ? error.message : 'Failed to send message'
					};
				});
				return null;
			}
		},

		// Message CRUD (implemented in ./chat/message-methods.ts)
		...messageMethods,

		// ====================================================================
		// NLP Methods (implemented in ./chat/nlp-methods.ts)
		// ====================================================================

		...nlp,

		// ====================================================================
		// LLM Integration Methods
		// ====================================================================

		/**
		 * Send a message and get an LLM response with streaming.
		 * Routes through agent if the thread has an active agent session.
		 */
		async sendMessageWithLlm(
			content: string,
			options?: LlmRequestOptions
		): Promise<Message | null> {
			// Send user message first
			const userMessage = await this.sendMessage(content, 'user');
			if (!userMessage) return null;

			// Check if thread has an agent session - route through agent instead
			const state = get({ subscribe });
			const thread = state.threads.find((t) => t.id === userMessage.threadId);
			log.debug('sendMessageWithLlm:', {
				threadId: userMessage.threadId,
				hasAgentSession: !!thread?.metadata.agentSessionId,
				agentSessionId: thread?.metadata.agentSessionId
			});

			if (thread?.metadata.agentSessionId) {
				let targetSessionId = thread.metadata.agentSessionId;
				const linkedSession = await agentStore.loadSession(targetSessionId);

				if (!linkedSession || linkedSession.state.type === 'cancelled') {
					const conversation = state.conversations.find((c) => c.id === thread.conversationId);
					const replacementSession = await agentStore.createSession(
						'code-agent',
						thread.conversationId,
						thread.id,
						conversation?.settings.workspaceRoot
					);

					if (replacementSession) {
						targetSessionId = replacementSession.id;
						await setThreadAgentSession(thread.id, replacementSession.id);
						update((s) => ({
							...s,
							threads: s.threads.map((item) =>
								item.id === thread.id
									? {
											...item,
											metadata: {
												...item.metadata,
												agentSessionId: replacementSession.id
											}
										}
									: item
							),
							conversations: s.conversations.map((conversation) =>
								conversation.id === thread.conversationId
									? {
											...conversation,
											threads: (conversation.threads || []).map((item) =>
												item.id === thread.id
													? {
															...item,
															metadata: {
																...item.metadata,
																agentSessionId: replacementSession.id
															}
														}
													: item
											)
										}
									: conversation
							)
						}));
					}
				}

				// Route through agent - the agent will emit events with results
				// We'll add the assistant message when execution_complete event arrives
				if (!targetSessionId) {
					update((s) => ({ ...s, error: 'Agent session is unavailable for this thread' }));
					return userMessage;
				}

				log.debug('Routing to agent:', targetSessionId);
				try {
					await agentStore.sendRequest(targetSessionId, content);
				} catch (err) {
					log.error('Agent request failed:', err);
					update((s) => ({ ...s, error: `Agent request failed: ${err}` }));
				}
				return userMessage;
			}

			log.debug('Using normal LLM flow');
			// Normal LLM flow (no agent)
			const validationError = llmHandler.validateCanStream(options);
			if (validationError) {
				update((s) => ({ ...s, error: validationError }));
				return userMessage;
			}

			// Initialize streaming
			const init = llmHandler.initStreaming(userMessage, options);
			if (!init) {
				update((s) => ({ ...s, error: 'Failed to initialize streaming' }));
				return userMessage;
			}

			const { assistantMessageId, streamingContext } = init;

			try {
				await llmHandler.startStream(content, assistantMessageId, streamingContext, options);
				return userMessage;
			} catch (error) {
				llmHandler.handleStreamError(assistantMessageId, error);
				return userMessage;
			}
		},

		/**
		 * Cancel the current LLM streaming request for the selected thread.
		 */
		async cancelLlmStream(): Promise<void> {
			const state = get({ subscribe });
			if (state.selectedThreadId) {
				// Cancel only the selected thread's stream
				await llmHandler.cancelThreadStream(state.selectedThreadId);
			} else {
				// Fallback: cancel all streams if no thread is selected (shouldn't happen in normal flow)
				await llmHandler.cancelStream();
			}
		},

		/**
		 * Check if LLM is currently streaming.
		 */
		isLlmStreaming(): boolean {
			return llmHandler.isStreaming();
		},

		/**
		 * Get the current streaming content.
		 */
		getStreamingContent(): string {
			return llmHandler.getStreamingContent();
		},

		// ====================================================================
		// Utility Methods (implemented in ./chat/utility-methods.ts)
		// ====================================================================

		...utilityMethods,

		/**
		 * Reset the entire store.
		 */
		reset() {
			classifier.cancel();
			llmHandler.reset();
			set(initialState);
		}
	};
}

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
