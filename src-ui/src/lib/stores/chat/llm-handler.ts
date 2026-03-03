/**
 * LLM Integration Handler
 *
 * Handles LLM streaming, cancellation, and event processing.
 * Extracted from chat.ts for maintainability.
 */

import { get, type Updater, type Writable } from 'svelte/store';
import type { Message } from '$lib/types/message';
import {
	sendChatMessageStreaming,
	isStartedEvent,
	isChunkEvent,
	isCompleteEvent,
	isErrorEvent,
	type LlmEvent,
	type StreamChatResult
} from '$lib/api/llm';
import { configStore } from '../configStore';
import type { ChatState, LlmRequestOptions, StreamingContext } from './types';

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
 * Active stream tracking.
 */
export interface ActiveStreamTracker {
	stream: StreamChatResult | null;
	requestId: string | null;
	localContent: string;
}

// ============================================================================
// LLM Handler Class
// ============================================================================

/**
 * Manages LLM streaming operations for the chat store.
 */
export class LlmHandler {
	private update: StoreUpdater;
	private subscribe: () => ChatState;
	private activeStream: ActiveStreamTracker = {
		stream: null,
		requestId: null,
		localContent: ''
	};

	constructor(
		update: StoreUpdater,
		subscribe: () => ChatState
	) {
		this.update = update;
		this.subscribe = subscribe;
	}

	/**
	 * Get the active stream for external access.
	 */
	getActiveStream(): StreamChatResult | null {
		return this.activeStream.stream;
	}

	/**
	 * Set the active stream.
	 */
	setActiveStream(stream: StreamChatResult | null): void {
		this.activeStream.stream = stream;
	}

	/**
	 * Check if streaming is active.
	 */
	isStreaming(): boolean {
		return this.subscribe().llm.isStreaming;
	}

	/**
	 * Get streaming content.
	 */
	getStreamingContent(): string {
		return this.subscribe().llm.streamingContent;
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

		if (state.llm.isStreaming) {
			return 'Already streaming';
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
			const modelExists = availableModels.some((m) => m.id === options.modelId);
			if (!modelExists) {
				return `Model '${options.modelId}' not found. Please configure a valid model.`;
			}
		}

		return null;
	}

	/**
	 * Initialize streaming - creates optimistic assistant message.
	 */
	initStreaming(content: string, options?: LlmRequestOptions): LlmStreamInit | null {
		const state = this.subscribe();
		const assistantMessageId = `streaming-${crypto.randomUUID()}`;

		const optimisticAssistant: Message = {
			id: assistantMessageId,
			conversationId: state.selectedConversationId!,
			threadId: state.selectedThreadId ?? '',
			role: 'assistant',
			content: '',
			timestamp: new Date(),
			status: 'sending'
		};

		const streamingContext: StreamingContext = {
			conversationId: state.selectedConversationId!,
			threadId: state.selectedThreadId
		};

		// Add optimistic message and set streaming state
		this.update((s) => ({
			...s,
			messages: [...s.messages, optimisticAssistant],
			llm: {
				isStreaming: true,
				requestId: null,
				streamingContent: '',
				model: options?.modelId ?? null
			}
		}));

		return { assistantMessageId, optimisticAssistant, streamingContext };
	}

	/**
	 * Handle LLM events during streaming.
	 */
	handleEvent(
		event: LlmEvent,
		assistantMessageId: string,
		streamingContext: StreamingContext
	): void {
		// Filter events by requestId
		if (this.activeStream.requestId !== null && event.requestId !== this.activeStream.requestId) {
			console.log('[Stream] Ignoring event from different request:', {
				eventRequestId: event.requestId,
				currentRequestId: this.activeStream.requestId
			});
			return;
		}

		// Validate context hasn't changed
		const currentState = this.subscribe();
		if (
			currentState.selectedConversationId !== streamingContext.conversationId ||
			currentState.selectedThreadId !== streamingContext.threadId
		) {
			console.log('[Stream] Ignoring event - context changed');
			return;
		}

		if (isStartedEvent(event)) {
			this.activeStream.requestId = event.requestId;
			this.update((s) => ({
				...s,
				llm: { ...s.llm, requestId: event.requestId, model: event.model }
			}));
		} else if (isChunkEvent(event)) {
			this.activeStream.localContent += event.delta;
			const contentToUpdate = this.activeStream.localContent;

			this.update((s) => {
				const messageExists = s.messages.some((m) => m.id === assistantMessageId);
				if (!messageExists) {
					console.warn('[Stream] Message not found:', assistantMessageId);
					return s;
				}

				return {
					...s,
					llm: { ...s.llm, streamingContent: contentToUpdate },
					messages: s.messages.map((m) =>
						m.id === assistantMessageId ? { ...m, content: contentToUpdate } : m
					)
				};
			});
		} else if (isCompleteEvent(event)) {
			this.update((s) => ({
				...s,
				messages: s.messages.map((m) =>
					m.id === assistantMessageId
						? {
								...m,
								content: event.content,
								status: 'sent' as const,
								metadata: event.usage
									? { tokens: event.usage.totalTokens, model: s.llm.model ?? undefined }
									: undefined
							}
						: m
				),
				llm: { isStreaming: false, requestId: null, streamingContent: '', model: null }
			}));
			this.cleanupStream();
		} else if (isErrorEvent(event)) {
			this.update((s) => ({
				...s,
				messages: s.messages.map((m) =>
					m.id === assistantMessageId ? { ...m, status: 'error' as const } : m
				),
				llm: { isStreaming: false, requestId: null, streamingContent: '', model: null },
				error: event.error
			}));
			this.cleanupStream();
		}
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
		const result = await sendChatMessageStreaming({
			conversationId: streamingContext.conversationId,
			threadId: streamingContext.threadId ?? undefined,
			content,
			...(options?.modelId && { modelId: options.modelId }),
			systemPrompt: options?.systemPrompt,
			temperature: options?.temperature,
			maxTokens: options?.maxTokens,
			onEvent: (event) => this.handleEvent(event, assistantMessageId, streamingContext)
		});

		this.activeStream.stream = result;
		this.activeStream.localContent = '';

		// Update with request ID
		this.update((s) => ({
			...s,
			llm: { ...s.llm, requestId: result.requestId }
		}));

		return result;
	}

	/**
	 * Cancel the current stream.
	 */
	async cancelStream(): Promise<void> {
		if (!this.activeStream.stream) return;

		try {
			await this.activeStream.stream.cancel();
			this.activeStream.stream.unsubscribe();
		} catch (error) {
			console.error('Failed to cancel LLM stream:', error);
		} finally {
			this.cleanupStream();
			this.update((s) => ({
				...s,
				llm: { isStreaming: false, requestId: null, streamingContent: '', model: null }
			}));
		}
	}

	/**
	 * Cancel stream without updating state (for conversation switches).
	 */
	async cancelStreamSilent(): Promise<void> {
		if (!this.activeStream.stream) return;

		try {
			await this.activeStream.stream.cancel();
			this.activeStream.stream.unsubscribe();
		} catch (error) {
			console.error('Failed to cancel stream:', error);
		} finally {
			this.cleanupStream();
		}
	}

	/**
	 * Cleanup stream state.
	 */
	private cleanupStream(): void {
		this.activeStream.stream = null;
		this.activeStream.requestId = null;
		this.activeStream.localContent = '';
	}

	/**
	 * Handle error during streaming.
	 */
	handleStreamError(assistantMessageId: string, error: unknown): void {
		this.update((s) => ({
			...s,
			messages: s.messages.map((m) =>
				m.id === assistantMessageId ? { ...m, status: 'error' as const } : m
			),
			llm: { isStreaming: false, requestId: null, streamingContent: '', model: null },
			error: error instanceof Error ? error.message : 'Failed to get LLM response'
		}));
		this.cleanupStream();
	}

	/**
	 * Reset handler state.
	 */
	reset(): void {
		if (this.activeStream.stream) {
			this.activeStream.stream.cancel().catch(console.error);
			this.activeStream.stream.unsubscribe();
		}
		this.cleanupStream();
	}
}
