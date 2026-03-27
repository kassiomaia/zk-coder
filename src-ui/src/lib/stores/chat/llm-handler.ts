/**
 * LLM Integration Handler
 *
 * Handles LLM streaming, cancellation, and event processing.
 * Extracted from chat.ts for maintainability.
 */

import {type Updater} from 'svelte/store';
import type {Message} from '$lib/types/message';
import {
    isChunkEvent,
    isCompleteEvent,
    isErrorEvent,
    isStartedEvent,
    type LlmEvent,
    sendChatMessageStreaming,
    type StreamChatResult
} from '$lib/api/llm';
import {configStore} from '../configStore';
import type {ChatState, LlmRequestOptions, StreamingContext, ThreadStreamingState} from './types';

// ============================================================================
// Types
// ============================================================================

/**
 * Callback to update the store state.
 */
export type StoreUpdater = (updater: Updater<ChatState>) => void;

/**
 * Result of initializing an LLM stream.
 */
export interface LlmStreamInit {
	assistantMessageId: string;
	optimisticAssistant: Message;
	streamingContext: StreamingContext;
}

/**
 * Isolated state for a single streaming session.
 * This is created fresh for each stream and captured in the event handler closure.
 */
interface StreamSessionState {
	requestId: string | null;
	expectedRequestId: string | null;
	bufferedEvents: LlmEvent[];
	localContent: string;
	startedAt: Date;
}

interface StreamEventHandler {
	onEvent: (event: LlmEvent) => void;
	bindRequestId: (requestId: string) => void;
}

// Allow slower providers/models enough time before declaring stream inactivity.
// Backend/provider request timeouts and explicit cancellation still bound total wait.
const STREAM_INACTIVITY_TIMEOUT_MS = 150_000;
const MAX_BUFFERED_EVENTS = 300;

// ============================================================================
// LLM Handler Class
// ============================================================================

/**
 * Manages LLM streaming operations for the chat store.
 */
export class LlmHandler {
	private update: StoreUpdater;
	private subscribe: () => ChatState;
	private activeStreams = new Map<string, StreamChatResult>();
	private streamTimeouts = new Map<string, ReturnType<typeof setTimeout>>();

	constructor(update: StoreUpdater, subscribe: () => ChatState) {
		this.update = update;
		this.subscribe = subscribe;
	}

	/**
	 * Check if any stream is active.
	 */
	isStreaming(): boolean {
		return this.activeStreams.size > 0;
	}

	/**
	 * Get streaming content for the currently selected thread.
	 */
	getStreamingContent(): string {
		const state = this.subscribe();
		if (!state.selectedThreadId) return '';
		return state.threadStreaming[state.selectedThreadId]?.streamingContent ?? '';
	}

	/**
	 * Build global LLM state for the currently selected thread from thread-level state.
	 */
	private buildSelectedLlmState(
		state: ChatState,
		threadStreaming: Record<string, ThreadStreamingState>
	): ChatState['llm'] {
		if (!state.selectedThreadId) {
			return { isStreaming: false, requestId: null, streamingContent: '', model: null };
		}

		const selected = threadStreaming[state.selectedThreadId];
		if (!selected) {
			return { isStreaming: false, requestId: null, streamingContent: '', model: null };
		}

		return {
			isStreaming: selected.isStreaming,
			requestId: selected.requestId,
			streamingContent: selected.streamingContent,
			model: selected.model
		};
	}

	/**
	 * Remove a thread from unread list.
	 */
	private clearUnread(unreadThreadIds: string[], threadId: string): string[] {
		return unreadThreadIds.filter((id) => id !== threadId);
	}

	private clearStreamTimeout(requestId: string): void {
		const timeout = this.streamTimeouts.get(requestId);
		if (timeout) {
			clearTimeout(timeout);
			this.streamTimeouts.delete(requestId);
		}
	}

	private armStreamTimeout(
		requestId: string,
		assistantMessageId: string,
		streamingContext: StreamingContext
	): void {
		this.clearStreamTimeout(requestId);
		const timeout = setTimeout(() => {
			this.handleStreamTimeout(requestId, assistantMessageId, streamingContext);
		}, STREAM_INACTIVITY_TIMEOUT_MS);
		this.streamTimeouts.set(requestId, timeout);
	}

	private handleStreamTimeout(
		requestId: string,
		assistantMessageId: string,
		streamingContext: StreamingContext
	): void {
		const stream = this.activeStreams.get(requestId);
		if (stream) {
			stream.cancel().catch((error) => {
				console.error(`Failed to cancel timed out stream ${requestId}:`, error);
			});
			stream.unsubscribe();
			this.activeStreams.delete(requestId);
		}
		this.clearStreamTimeout(requestId);

		this.update((s) => {
			const threadId = streamingContext.threadId;
			if (!threadId) {
				return { ...s, error: 'Streaming timed out. Please try again.' };
			}

			const currentStream = s.threadStreaming[threadId];
			if (!currentStream || currentStream.assistantMessageId !== assistantMessageId) {
				return s;
			}

			const nextThreadStreaming = { ...s.threadStreaming };
			delete nextThreadStreaming[threadId];
			const isSelectedThread = s.selectedThreadId === threadId;

			return {
				...s,
				messages: isSelectedThread
					? s.messages.map((message) =>
							message.id === assistantMessageId ? { ...message, status: 'error' as const } : message
						)
					: s.messages,
				threadStreaming: nextThreadStreaming,
				llm: this.buildSelectedLlmState(s, nextThreadStreaming),
				error: 'Streaming timed out. Please try again.'
			};
		});
	}

	/**
	 * Ensure message/thread/conversation metadata stays fresh after new activity.
	 */
	private touchThreadAndConversation(
		state: ChatState,
		threadId: string,
		conversationId: string,
		options?: { messageDelta?: number; timestamp?: Date }
	): ChatState {
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
	}

	/**
	 * Validate that LLM can start streaming.
	 * Returns error message if validation fails, null if OK.
	 */
	validateCanStream(options?: LlmRequestOptions): string | null {
		const state = this.subscribe();

		if (!state.selectedConversationId) {
			return 'No conversation selected';
		}

		if (state.selectedThreadId && state.threadStreaming[state.selectedThreadId]?.isStreaming) {
			return 'This thread is already processing a response';
		}

		// Check model availability
		const modelToUse = options?.modelId ?? configStore.getDefault('code');
		if (!modelToUse) {
			const availableModels = configStore.getAvailableModels('code');
			if (availableModels.length === 0) {
				return 'No model configured. Please go to Settings > Providers and configure an API provider first.';
			}
			return 'No default model set for code operations. Please go to Settings > Models and set a default model for the "Code" operation.';
		}

		// Validate explicit model exists
		if (options?.modelId) {
			const availableModels = configStore.getAvailableModels('code');
			const modelExists = availableModels.some((model) => model.id === options.modelId);
			if (!modelExists) {
				return `Model '${options.modelId}' not found. Please configure a valid model.`;
			}
		}

		return null;
	}

	/**
	 * Initialize streaming - creates optimistic assistant message.
	 */
	initStreaming(userMessage: Message, options?: LlmRequestOptions): LlmStreamInit | null {
		const state = this.subscribe();
		const conversationId = userMessage.conversationId || state.selectedConversationId;
		const threadId = userMessage.threadId || state.selectedThreadId;

		if (!conversationId || !threadId) {
			return null;
		}

		const assistantMessageId = `streaming-${crypto.randomUUID()}`;
		const optimisticAssistant: Message = {
			id: assistantMessageId,
			conversationId,
			threadId,
			role: 'assistant',
			content: '',
			timestamp: new Date(),
			status: 'sending'
		};

		const streamingContext: StreamingContext = {
			conversationId,
			threadId
		};

		this.update((s) => {
			const nextThreadStreaming: Record<string, ThreadStreamingState> = {
				...s.threadStreaming,
				[threadId]: {
					conversationId,
					assistantMessageId,
					isStreaming: true,
					requestId: null,
					streamingContent: '',
					model: options?.modelId ?? null
				}
			};

			const isSelectedContext =
				s.selectedConversationId === conversationId && s.selectedThreadId === threadId;

			return {
				...s,
				messages: isSelectedContext ? [...s.messages, optimisticAssistant] : s.messages,
				unreadThreadIds: this.clearUnread(s.unreadThreadIds, threadId),
				threadStreaming: nextThreadStreaming,
				llm: this.buildSelectedLlmState(s, nextThreadStreaming)
			};
		});

		return { assistantMessageId, optimisticAssistant, streamingContext };
	}

	/**
	 * Create an event handler with isolated streaming state.
	 */
	createEventHandler(
		assistantMessageId: string,
		streamingContext: StreamingContext
	): StreamEventHandler {
		const sessionState: StreamSessionState = {
			requestId: null,
			expectedRequestId: null,
			bufferedEvents: [],
			localContent: '',
			startedAt: new Date()
		};

		const handleMatchedEvent = (event: LlmEvent): void => {
			if (isStartedEvent(event)) {
				sessionState.requestId = event.requestId;
				this.armStreamTimeout(event.requestId, assistantMessageId, streamingContext);
				this.update((s) => {
					const threadId = streamingContext.threadId;
					if (!threadId) return s;

					const existing = s.threadStreaming[threadId];
					const nextThreadStreaming: Record<string, ThreadStreamingState> = {
						...s.threadStreaming,
						[threadId]: {
							conversationId: streamingContext.conversationId,
							assistantMessageId,
							isStreaming: true,
							requestId: event.requestId,
							streamingContent: existing?.streamingContent ?? '',
							model: event.model
						}
					};

					return {
						...s,
						threadStreaming: nextThreadStreaming,
						llm: this.buildSelectedLlmState(s, nextThreadStreaming)
					};
				});
			} else if (isChunkEvent(event)) {
				if (event.requestId) {
					this.armStreamTimeout(event.requestId, assistantMessageId, streamingContext);
				}
				sessionState.localContent += event.delta;
				const contentToUpdate = sessionState.localContent;

				this.update((s) => {
					const threadId = streamingContext.threadId;
					if (!threadId) return s;

					const existing = s.threadStreaming[threadId];
					const nextThreadStreaming: Record<string, ThreadStreamingState> = {
						...s.threadStreaming,
						[threadId]: {
							conversationId: streamingContext.conversationId,
							assistantMessageId,
							isStreaming: true,
							requestId: sessionState.requestId ?? existing?.requestId ?? null,
							streamingContent: contentToUpdate,
							model: existing?.model ?? null
						}
					};

					const isSelectedThread = s.selectedThreadId === threadId;

					let nextMessages = s.messages;
					if (isSelectedThread) {
						const messageExists = s.messages.some((message) => message.id === assistantMessageId);
						nextMessages = messageExists
							? s.messages.map((message) =>
									message.id === assistantMessageId
										? { ...message, content: contentToUpdate, status: 'sending' as const }
										: message
								)
							: [
									...s.messages,
									{
										id: assistantMessageId,
										conversationId: streamingContext.conversationId,
										threadId,
										role: 'assistant',
										content: contentToUpdate,
										timestamp: sessionState.startedAt,
										status: 'sending'
									}
								];
					}

					return {
						...s,
						messages: nextMessages,
						threadStreaming: nextThreadStreaming,
						llm: this.buildSelectedLlmState(s, nextThreadStreaming)
					};
				});
			} else if (isCompleteEvent(event)) {
				this.clearStreamTimeout(sessionState.requestId ?? event.requestId);
				this.update((s) => {
					const threadId = streamingContext.threadId;
					if (!threadId) return s;

					const nextThreadStreaming = { ...s.threadStreaming };
					const streamState = nextThreadStreaming[threadId];
					delete nextThreadStreaming[threadId];

					const isSelectedThread = s.selectedThreadId === threadId;

					const nextMessages = isSelectedThread
						? s.messages.some((message) => message.id === assistantMessageId)
							? s.messages.map((message) =>
									message.id === assistantMessageId
										? {
												...message,
												content: event.content,
												status: 'sent' as const,
												metadata: event.usage
													? {
															tokens: event.usage.totalTokens,
															model: streamState?.model ?? undefined
														}
													: undefined
											}
										: message
								)
							: [
									...s.messages,
									{
										id: assistantMessageId,
										conversationId: streamingContext.conversationId,
										threadId,
										role: 'assistant' as const,
										content: event.content,
										timestamp: sessionState.startedAt,
										status: 'sent' as const,
										metadata: event.usage
											? {
													tokens: event.usage.totalTokens,
													model: streamState?.model ?? undefined
												}
											: undefined
									}
								]
						: s.messages;

					const unreadThreadIds = isSelectedThread
						? this.clearUnread(s.unreadThreadIds, threadId)
						: s.unreadThreadIds.includes(threadId)
							? s.unreadThreadIds
							: [threadId, ...s.unreadThreadIds];

					const touched = this.touchThreadAndConversation(
						{
							...s,
							messages: nextMessages,
							unreadThreadIds,
							threadStreaming: nextThreadStreaming
						},
						threadId,
						streamingContext.conversationId,
						{ messageDelta: event.content.trim().length > 0 ? 1 : 0 }
					);

					return {
						...touched,
						llm: this.buildSelectedLlmState(touched, nextThreadStreaming)
					};
				});

				this.cleanupStream(sessionState.requestId ?? event.requestId);
			} else if (isErrorEvent(event)) {
				this.clearStreamTimeout(sessionState.requestId ?? event.requestId);
				this.update((s) => {
					const threadId = streamingContext.threadId;
					if (!threadId) return { ...s, error: event.error };

					const nextThreadStreaming = { ...s.threadStreaming };
					delete nextThreadStreaming[threadId];

					const isSelectedThread = s.selectedThreadId === threadId;

					return {
						...s,
						messages: isSelectedThread
							? s.messages.map((message) =>
									message.id === assistantMessageId
										? { ...message, status: 'error' as const }
										: message
								)
							: s.messages,
						threadStreaming: nextThreadStreaming,
						llm: this.buildSelectedLlmState(s, nextThreadStreaming),
						error: event.error
					};
				});

				this.cleanupStream(sessionState.requestId ?? event.requestId);
			}
		};

		const onEvent = (event: LlmEvent): void => {
			if (sessionState.requestId !== null) {
				if (event.requestId !== sessionState.requestId) return;
				handleMatchedEvent(event);
				return;
			}

			if (sessionState.expectedRequestId === null) {
				if (!event.requestId) return;
				if (sessionState.bufferedEvents.length >= MAX_BUFFERED_EVENTS) {
					sessionState.bufferedEvents.shift();
				}
				sessionState.bufferedEvents.push(event);
				return;
			}

			if (event.requestId !== sessionState.expectedRequestId) return;

			sessionState.requestId = sessionState.expectedRequestId;
			handleMatchedEvent(event);
		};

		const bindRequestId = (requestId: string): void => {
			if (sessionState.requestId && sessionState.requestId !== requestId) {
				return;
			}

			sessionState.expectedRequestId = requestId;
			sessionState.requestId = requestId;
			this.armStreamTimeout(requestId, assistantMessageId, streamingContext);

			if (sessionState.bufferedEvents.length === 0) return;

			const buffered = sessionState.bufferedEvents;
			sessionState.bufferedEvents = [];

			for (const event of buffered) {
				if (event.requestId !== requestId) continue;
				handleMatchedEvent(event);
			}
		};

		return { onEvent, bindRequestId };
	}

	/**
	 * Start the LLM stream.
	 */
	async startStream(
		content: string,
		assistantMessageId: string,
		streamingContext: StreamingContext,
		options?: LlmRequestOptions
	): Promise<StreamChatResult> {
		const eventHandler = this.createEventHandler(assistantMessageId, streamingContext);

		const result = await sendChatMessageStreaming({
			conversationId: streamingContext.conversationId,
			threadId: streamingContext.threadId ?? undefined,
			content,
			...(options?.modelId && { modelId: options.modelId }),
			systemPrompt: options?.systemPrompt,
			temperature: options?.temperature,
			maxTokens: options?.maxTokens,
			onEvent: eventHandler.onEvent
		});

		this.activeStreams.set(result.requestId, result);
		eventHandler.bindRequestId(result.requestId);

		this.update((s) => {
			const threadId = streamingContext.threadId;
			if (!threadId) return s;

			const existing = s.threadStreaming[threadId];
			if (!existing) return s;

			const nextThreadStreaming: Record<string, ThreadStreamingState> = {
				...s.threadStreaming,
				[threadId]: {
					...existing,
					requestId: result.requestId
				}
			};

			return {
				...s,
				threadStreaming: nextThreadStreaming,
				llm: this.buildSelectedLlmState(s, nextThreadStreaming)
			};
		});

		return result;
	}

	/**
	 * Cancel all active streams.
	 */
	async cancelStream(): Promise<void> {
		for (const [requestId, stream] of this.activeStreams) {
			try {
				await stream.cancel();
			} catch (error) {
				console.error(`Failed to cancel LLM stream ${requestId}:`, error);
			} finally {
				stream.unsubscribe();
				this.clearStreamTimeout(requestId);
			}
		}

		this.activeStreams.clear();
		this.streamTimeouts.clear();

		this.update((s) => ({
			...s,
			threadStreaming: {},
			llm: { isStreaming: false, requestId: null, streamingContent: '', model: null }
		}));
	}

	/**
	 * Cancel all streams without updating store state.
	 */
	async cancelStreamSilent(): Promise<void> {
		for (const [requestId, stream] of this.activeStreams) {
			try {
				await stream.cancel();
			} catch (error) {
				console.error(`Failed to cancel stream ${requestId}:`, error);
			} finally {
				stream.unsubscribe();
				this.clearStreamTimeout(requestId);
			}
		}

		this.activeStreams.clear();
		this.streamTimeouts.clear();
	}

	/**
	 * Cleanup stream listener and registry entry.
	 */
	private cleanupStream(requestId: string): void {
		this.clearStreamTimeout(requestId);
		const stream = this.activeStreams.get(requestId);
		if (!stream) return;

		stream.unsubscribe();
		this.activeStreams.delete(requestId);
	}

	/**
	 * Handle error during stream startup.
	 */
	handleStreamError(assistantMessageId: string, error: unknown): void {
		this.update((s) => {
			let targetThreadId: string | null = null;
			for (const [threadId, stream] of Object.entries(s.threadStreaming)) {
				if (stream.assistantMessageId === assistantMessageId) {
					targetThreadId = threadId;
					break;
				}
			}

			if (!targetThreadId) {
				return {
					...s,
					error: error instanceof Error ? error.message : 'Failed to get LLM response'
				};
			}

			const nextThreadStreaming = { ...s.threadStreaming };
			delete nextThreadStreaming[targetThreadId];

			return {
				...s,
				messages: s.messages.map((message) =>
					message.id === assistantMessageId ? { ...message, status: 'error' as const } : message
				),
				threadStreaming: nextThreadStreaming,
				llm: this.buildSelectedLlmState(s, nextThreadStreaming),
				error: error instanceof Error ? error.message : 'Failed to get LLM response'
			};
		});
	}

	/**
	 * Reset handler state.
	 */
	reset(): void {
		for (const [requestId, stream] of this.activeStreams) {
			stream.cancel().catch(console.error);
			stream.unsubscribe();
			this.clearStreamTimeout(requestId);
		}
		this.activeStreams.clear();
		this.streamTimeouts.clear();
	}
}
