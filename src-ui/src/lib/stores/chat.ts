/**
 * Chat State Management
 *
 * Main Svelte store for managing conversations, messages, threads, and NLP state.
 * LLM streaming logic extracted to ./chat/llm-handler.ts
 */

import { writable, get } from 'svelte/store';
import type { Conversation, Thread, ConversationSettings } from '$lib/types/thread';
import type { Message } from '$lib/types/message';
import {
	addContextMessage,
	detectAmbiguities,
	resolveAmbiguity,
	applyResolutions,
	createDebouncedClassifier,
	isBackendAvailable
} from '$lib/api/nlp';
import type { ResolutionType } from '$lib/types/nlp';
import {
	loadConversations,
	createConversation as apiCreateConversation,
	renameConversation as apiRenameConversation,
	deleteConversation as apiDeleteConversation,
	updateConversationSettings as apiUpdateConversationSettings,
	setConversationWorkspace as apiSetConversationWorkspace,
	loadThreads,
	createThread as apiCreateThread,
	resolveThread as apiResolveThread,
	deleteThread as apiDeleteThread,
	renameThread as apiRenameThread,
	loadThreadMessages,
	sendMessage as apiSendMessage,
	deleteMessage as apiDeleteMessage,
	updateMessageStatus as apiUpdateMessageStatus,
	updateMessageFeedback as apiUpdateMessageFeedback,
	createThreadWithMessage as apiCreateThreadWithMessage,
	setThreadAgentSession
} from '$lib/api/chat';
import { LlmHandler } from './chat/llm-handler';
import type { ChatState, LlmRequestOptions } from './chat/types';
import { deriveThreadTitle, sortThreadsByRecent } from './chat/utils';
import { agentStore } from './agentStore';

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
	const classifier = createDebouncedClassifier(300);

	// LLM handler for streaming operations
	const llmHandler = new LlmHandler(update, () => get({ subscribe }));

	// Pending thread creation tracking to prevent race conditions
	let pendingThreadReady: Promise<Thread | null> | null = null;
	let pendingThreadConversationId: string | null = null;

	const clearUnreadForThread = (unreadThreadIds: string[], threadId: string | null): string[] => {
		if (!threadId) return unreadThreadIds;
		return unreadThreadIds.filter((id) => id !== threadId);
	};

	const touchThreadAndConversation = (
		state: ChatState,
		threadId: string,
		conversationId: string,
		options?: { messageDelta?: number; timestamp?: Date }
	): ChatState => {
		const now = options?.timestamp ?? new Date();
		const messageDelta = options?.messageDelta ?? 0;

		return {
			...state,
			threads: state.threads.map((thread) =>
				thread.id === threadId
					? {
							...thread,
							updatedAt: now,
							messageCount: Math.max(0, thread.messageCount + messageDelta)
						}
					: thread
			),
			conversations: state.conversations.map((conversation) =>
				conversation.id === conversationId
					? {
							...conversation,
							updatedAt: now,
							threads: (conversation.threads || []).map((thread) =>
								thread.id === threadId
									? {
											...thread,
											updatedAt: now,
											messageCount: Math.max(0, thread.messageCount + messageDelta)
										}
									: thread
							)
						}
					: conversation
			)
		};
	};

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
		// Conversation Methods
		// ====================================================================

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
		 * Update a conversation's settings (model, temperature, etc.).
		 */
		async updateConversationSettings(
			id: string,
			settings: Partial<ConversationSettings>
		): Promise<boolean> {
			try {
				const state = get({ subscribe });
				const conversation = state.conversations.find((c) => c.id === id);
				if (!conversation) {
					update((s) => ({ ...s, error: 'Conversation not found' }));
					return false;
				}

				// Merge with existing settings
				const newSettings: ConversationSettings = {
					...conversation.settings,
					...settings
				};

				const updated = await apiUpdateConversationSettings(id, {
					model: newSettings.model,
					temperature: newSettings.temperature,
					maxTokens: newSettings.maxTokens,
					systemPrompt: newSettings.systemPrompt,
					workspaceRoot: newSettings.workspaceRoot
				});

				update((state) => ({
					...state,
					conversations: state.conversations.map((c) =>
						c.id === id ? { ...c, settings: updated.settings } : c
					)
				}));
				return true;
			} catch (error) {
				update((state) => ({
					...state,
					error: error instanceof Error ? error.message : 'Failed to update settings'
				}));
				return false;
			}
		},

		/**
		 * Set a conversation's workspace root.
		 */
		async setConversationWorkspace(id: string, workspaceRoot: string): Promise<boolean> {
			try {
				const updated = await apiSetConversationWorkspace(id, workspaceRoot);
				update((state) => ({
					...state,
					conversations: state.conversations.map((c) =>
						c.id === id
							? { ...c, settings: { ...c.settings, workspaceRoot: updated.settings.workspaceRoot } }
							: c
					)
				}));
				return true;
			} catch (error) {
				update((state) => ({
					...state,
					error: error instanceof Error ? error.message : 'Failed to set workspace'
				}));
				return false;
			}
		},

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

					if (threads.length > 0) {
						const mostRecentThread = sortThreadsByRecent(threads)[0];
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
				if (thread?.metadata.agentSessionId) {
					// Load the session from backend before setting as active
					const session = await agentStore.loadSession(thread.metadata.agentSessionId);
					if (session) {
						agentStore.setActiveSession(thread.metadata.agentSessionId);
					}
				}

				try {
					const loadedMessages = await loadThreadMessages(id);
					update((state) => {
						const threadStream = state.threadStreaming[id];
						if (!threadStream?.isStreaming) {
							return {
								...state,
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
			const readyPromise = (async () => {
				try {
					const thread = await apiCreateThread(conversationId, title);

					// Auto-create an agent session for this thread
					try {
						const agentSession = await agentStore.createSession(
							'code-agent', // Default to code agent
							conversationId,
							thread.id,
							undefined // No workspace root initially
						);

						if (agentSession) {
							// Update thread metadata with agent session ID
							await setThreadAgentSession(thread.id, agentSession.id);
							thread.metadata.agentSessionId = agentSession.id;

							// Open the agent panel
							agentStore.setActiveSession(agentSession.id);
						}
					} catch (agentError) {
						console.warn('[chatStore] Failed to create agent session:', agentError);
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

		/**
		 * Resolve or unresolve a thread.
		 */
		async resolveThread(id: string, resolved: boolean): Promise<boolean> {
			try {
				await apiResolveThread(id, resolved);
				update((state) => ({
					...state,
					threads: state.threads.map((t) =>
						t.id === id ? { ...t, metadata: { ...t.metadata, resolved }, updatedAt: new Date() } : t
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
					threadStreaming: Object.fromEntries(
						Object.entries(state.threadStreaming).filter(([threadId]) => threadId !== id)
					),
					unreadThreadIds: state.unreadThreadIds.filter((threadId) => threadId !== id),
					selectedThreadId: state.selectedThreadId === id ? null : state.selectedThreadId,
					messages: state.selectedThreadId === id ? [] : state.messages,
					conversations: state.conversations.map((c) =>
						c.id === state.selectedConversationId
							? { ...c, threads: (c.threads || []).filter((t) => t.id !== id) }
							: c
					)
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
		 * Rename a thread.
		 */
		async renameThread(id: string, title: string): Promise<boolean> {
			try {
				const updated = await apiRenameThread(id, title);
				update((state) => ({
					...state,
					threads: state.threads.map((t) => (t.id === id ? updated : t)),
					conversations: state.conversations.map((c) =>
						c.id === state.selectedConversationId
							? { ...c, threads: (c.threads || []).map((t) => (t.id === id ? updated : t)) }
							: c
					)
				}));
				return true;
			} catch (error) {
				update((state) => ({
					...state,
					error: error instanceof Error ? error.message : 'Failed to rename thread'
				}));
				return false;
			}
		},

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
					console.warn('Pending thread creation failed:', error);
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
							console.warn('[chatStore] Failed to create agent session:', agentError);
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

		/**
		 * Add a message to the current conversation (local only).
		 */
		async addMessage(message: Message) {
			update((state) => ({ ...state, messages: [...state.messages, message] }));

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
		 * Update a message's feedback (like/dislike).
		 * Pass null to clear the feedback.
		 * Clicking the same feedback toggles it off.
		 */
		async updateMessageFeedback(id: string, feedback: 'like' | 'dislike' | null): Promise<boolean> {
			try {
				await apiUpdateMessageFeedback(id, feedback);
				update((state) => ({
					...state,
					messages: state.messages.map((m) =>
						m.id === id
							? {
									...m,
									metadata: {
										...m.metadata,
										feedback: feedback ?? undefined
									}
								}
							: m
					)
				}));
				return true;
			} catch (error) {
				console.error('Failed to update message feedback:', error);
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

		// ====================================================================
		// NLP Methods
		// ====================================================================

		/**
		 * Classify intent for input text.
		 */
		async classifyInput(input: string) {
			const state = get({ subscribe });
			if (!state.nlp.backendAvailable || !state.selectedConversationId) return;

			update((s) => ({ ...s, nlp: { ...s.nlp, processing: true, error: null } }));

			try {
				const result = await classifier.classify({
					input,
					conversationId: state.selectedConversationId
				});

				update((s) => ({
					...s,
					nlp: { ...s.nlp, detectedIntent: result, processing: false }
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
			if (!state.nlp.backendAvailable || !state.selectedConversationId) return;

			try {
				const ambiguities = await detectAmbiguities({
					input,
					conversationId: state.selectedConversationId
				});
				update((s) => ({
					...s,
					nlp: {
						...s.nlp,
						ambiguities,
						// Auto-populate pending ambiguities for high/medium severity
						pendingAmbiguities: ambiguities.filter((a) => a.severity !== 'low')
					}
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
					pendingAmbiguities: [],
					resolvedAmbiguities: new Map(),
					processing: false,
					error: null
				}
			}));
		},

		/**
		 * Resolve an ambiguity with the provided resolution.
		 */
		async resolveAmbiguity(
			ambiguityId: string,
			resolution: string,
			resolutionType: ResolutionType
		): Promise<boolean> {
			const state = get({ subscribe });
			const ambiguity = state.nlp.pendingAmbiguities.find((a) => a.id === ambiguityId);

			if (!ambiguity) {
				update((s) => ({
					...s,
					nlp: { ...s.nlp, error: 'Ambiguity not found' }
				}));
				return false;
			}

			try {
				const resolved = await resolveAmbiguity({
					ambiguityId,
					originalText: ambiguity.span.text,
					resolution,
					resolutionType
				});

				update((s) => {
					const newResolvedAmbiguities = new Map(s.nlp.resolvedAmbiguities);
					newResolvedAmbiguities.set(ambiguityId, resolved);

					return {
						...s,
						nlp: {
							...s.nlp,
							pendingAmbiguities: s.nlp.pendingAmbiguities.filter((a) => a.id !== ambiguityId),
							resolvedAmbiguities: newResolvedAmbiguities
						}
					};
				});

				return true;
			} catch (error) {
				update((s) => ({
					...s,
					nlp: {
						...s.nlp,
						error: error instanceof Error ? error.message : 'Failed to resolve ambiguity'
					}
				}));
				return false;
			}
		},

		/**
		 * Dismiss an ambiguity without resolving it.
		 */
		async dismissAmbiguity(ambiguityId: string): Promise<boolean> {
			return this.resolveAmbiguity(ambiguityId, '', 'dismissed');
		},

		/**
		 * Check if there are any blocking ambiguities.
		 */
		hasBlockingAmbiguities(): boolean {
			const state = get({ subscribe });
			return state.nlp.pendingAmbiguities.some((a) => a.severity === 'high');
		},

		/**
		 * Apply all resolved ambiguities to the input text.
		 */
		async applyAmbiguityResolutions(input: string): Promise<string> {
			const state = get({ subscribe });
			const resolutions: Record<string, string> = {};

			state.nlp.resolvedAmbiguities.forEach((resolved, id) => {
				if (resolved.resolution_type !== 'dismissed') {
					resolutions[id] = resolved.resolution;
				}
			});

			if (Object.keys(resolutions).length === 0) {
				return input;
			}

			try {
				return await applyResolutions({ input, resolutions });
			} catch (error) {
				console.error('Failed to apply resolutions:', error);
				return input;
			}
		},

		/**
		 * Clear resolved ambiguities after they've been applied.
		 */
		clearResolvedAmbiguities() {
			update((state) => ({
				...state,
				nlp: {
					...state.nlp,
					resolvedAmbiguities: new Map()
				}
			}));
		},

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
			console.debug('[chatStore] sendMessageWithLlm:', {
				threadId: userMessage.threadId,
				hasAgentSession: !!thread?.metadata.agentSessionId,
				agentSessionId: thread?.metadata.agentSessionId
			});

			if (thread?.metadata.agentSessionId) {
				// Route through agent - the agent will emit events with results
				// We'll add the assistant message when execution_complete event arrives
				console.debug('[chatStore] Routing to agent:', thread.metadata.agentSessionId);
				try {
					await agentStore.sendRequest(thread.metadata.agentSessionId, content);
				} catch (err) {
					console.error('[chatStore] Agent request failed:', err);
					update((s) => ({ ...s, error: `Agent request failed: ${err}` }));
				}
				return userMessage;
			}

			console.debug('[chatStore] Using normal LLM flow');
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
		 * Cancel the current LLM streaming request.
		 */
		async cancelLlmStream(): Promise<void> {
			await llmHandler.cancelStream();
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
		// Utility Methods
		// ====================================================================

		/**
		 * Clear error state.
		 */
		clearError() {
			update((state) => ({ ...state, error: null }));
		},

		/**
1144: 		 * Mark a thread as read (clears unread indicator).
1145: 		 */
1146: 		markThreadRead(threadId: string) {
1147: 			update((state) => {
1148: 				if (!state.unreadThreadIds.includes(threadId)) {
1149: 					return state;
1150: 				}
1151: 				return {
1152: 					...state,
1153: 					unreadThreadIds: clearUnreadForThread(state.unreadThreadIds, threadId)
1154: 				};
1155: 			});
1156: 		},

		/**
		 * Set a thread's workspace root.
		 */
		async setThreadWorkspace(id: string, workspaceRoot: string): Promise<boolean> {
			try {
				update((state) => ({
					...state,
					threads: state.threads.map((t) =>
						t.id === id ? { ...t, metadata: { ...t.metadata, workspaceRoot } } : t
					)
				}));
				return true;
			} catch (error) {
				console.error('Failed to set thread workspace:', error);
				return false;
			}
		},

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

import { createDerivedStores } from './chat/derived-stores';

const derivedStores = createDerivedStores(chatStore);

export const selectedConversation = derivedStores.selectedConversation;
export const selectedThread = derivedStores.selectedThread;
export const currentThreads = derivedStores.currentThreads;
export const activeStreamingThreadIds = derivedStores.activeStreamingThreadIds;
export const unreadThreadIds = derivedStores.unreadThreadIds;
export const messageGroups = derivedStores.messageGroups;
export const conversationGroups = derivedStores.conversationGroups;
export const nlpState = derivedStores.nlpState;
export const llmState = derivedStores.llmState;
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
export { generateId, createNewConversation, createNewThread, createNewMessage } from './chat/utils';
