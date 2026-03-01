/**
 * LLM API Client
 * TypeScript wrappers for Tauri IPC commands to the LLM backend.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Message } from '$lib/types/message';

// ============================================================================
// Error Handling
// ============================================================================

/**
 * Error class for LLM API errors.
 */
export class LlmApiError extends Error {
	constructor(
		message: string,
		public readonly command: string,
		public readonly cause?: unknown
	) {
		super(message);
		this.name = 'LlmApiError';
	}
}

/**
 * Wraps invoke with error handling.
 */
async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await invoke<T>(command, args);
	} catch (error) {
		throw new LlmApiError(
			`LLM command '${command}' failed: ${error instanceof Error ? error.message : String(error)}`,
			command,
			error
		);
	}
}

// ============================================================================
// Types
// ============================================================================

/**
 * Usage statistics from LLM response.
 */
export interface UsageStats {
	inputTokens: number;
	outputTokens: number;
	totalTokens: number;
}

/**
 * Options for sending a chat message to the LLM.
 */
export interface SendChatMessageOptions {
	/** Conversation ID */
	conversationId: string;
	/** Thread ID (optional) */
	threadId?: string;
	/** The user's message content */
	content: string;
	/** Model ID to use (e.g., "openai-gpt-4") */
	modelId: string;
	/** System prompt (optional) */
	systemPrompt?: string;
	/** Temperature (optional, default 0.7) */
	temperature?: number;
	/** Max tokens (optional, default 4096) */
	maxTokens?: number;
}

/**
 * Response from a non-streaming chat request.
 */
export interface SendChatMessageResponse {
	/** The generated response content */
	content: string;
	/** The model used */
	model: string;
	/** Usage statistics */
	usage?: UsageStats;
	/** The saved assistant message */
	message: ChatMessageResponse;
}

/**
 * Chat message from the backend (matches backend type).
 */
export interface ChatMessageResponse {
	id: string;
	conversationId: string;
	threadId: string | null;
	role: 'user' | 'assistant' | 'system';
	content: string;
	timestamp: string;
	status: 'sending' | 'sent' | 'delivered' | 'error';
	metadata?: {
		intent?: string;
		confidence?: number;
		tokens?: number;
		model?: string;
	};
}

// ============================================================================
// Streaming Event Types
// ============================================================================

/**
 * LLM event types emitted during streaming.
 */
export type LlmEventType = 'started' | 'chunk' | 'complete' | 'error';

/**
 * Base LLM event.
 */
export interface LlmEventBase {
	type: LlmEventType;
	requestId: string;
}

/**
 * Streaming started event.
 */
export interface LlmStartedEvent extends LlmEventBase {
	type: 'started';
	model: string;
}

/**
 * Content chunk event.
 */
export interface LlmChunkEvent extends LlmEventBase {
	type: 'chunk';
	delta: string;
}

/**
 * Streaming complete event.
 */
export interface LlmCompleteEvent extends LlmEventBase {
	type: 'complete';
	content: string;
	usage?: UsageStats;
	finishReason?: string;
}

/**
 * Error event.
 */
export interface LlmErrorEvent extends LlmEventBase {
	type: 'error';
	error: string;
	retryable: boolean;
}

/**
 * Union type for all LLM events.
 */
export type LlmEvent = LlmStartedEvent | LlmChunkEvent | LlmCompleteEvent | LlmErrorEvent;

/**
 * Callback for LLM events.
 */
export type LlmEventCallback = (event: LlmEvent) => void;

// ============================================================================
// Non-Streaming Chat
// ============================================================================

/**
 * Sends a chat message and returns the complete response.
 *
 * @param options - Chat message options
 * @returns The response with content and saved message
 */
export async function sendChatMessage(options: SendChatMessageOptions): Promise<SendChatMessageResponse> {
	return safeInvoke<SendChatMessageResponse>('send_chat_message', {
		conversationId: options.conversationId,
		threadId: options.threadId,
		content: options.content,
		modelId: options.modelId,
		systemPrompt: options.systemPrompt,
		temperature: options.temperature,
		maxTokens: options.maxTokens
	});
}

// ============================================================================
// Streaming Chat
// ============================================================================

/**
 * Options for streaming chat.
 */
export interface StreamChatOptions extends SendChatMessageOptions {
	/** Callback for LLM events */
	onEvent: LlmEventCallback;
}

/**
 * Result of starting a streaming request.
 */
export interface StreamChatResult {
	/** Request ID for tracking */
	requestId: string;
	/** Function to cancel the stream */
	cancel: () => Promise<void>;
	/** Function to unsubscribe from events */
	unsubscribe: UnlistenFn;
}

/**
 * Sends a chat message with streaming response.
 * Events are emitted via the onEvent callback.
 *
 * @param options - Streaming chat options
 * @returns Request ID and cleanup functions
 */
export async function sendChatMessageStreaming(options: StreamChatOptions): Promise<StreamChatResult> {
	// Set up event listener first
	const unsubscribe = await listen<LlmEvent>('llm:event', (event) => {
		options.onEvent(event.payload);
	});

	// Start the streaming request
	try {
		const requestId = await safeInvoke<string>('send_chat_message_streaming', {
			conversationId: options.conversationId,
			threadId: options.threadId,
			content: options.content,
			modelId: options.modelId,
			systemPrompt: options.systemPrompt,
			temperature: options.temperature,
			maxTokens: options.maxTokens
		});

		return {
			requestId,
			cancel: async () => {
				await safeInvoke<void>('cancel_chat_request', { requestId });
			},
			unsubscribe
		};
	} catch (error) {
		// Clean up listener if request fails
		unsubscribe();
		throw error;
	}
}

/**
 * Cancels an in-progress streaming request.
 *
 * @param requestId - The request ID to cancel
 */
export async function cancelChatRequest(requestId: string): Promise<void> {
	await safeInvoke<void>('cancel_chat_request', { requestId });
}

// ============================================================================
// Conversion Helpers
// ============================================================================

/**
 * Converts a backend message response to client Message format.
 */
export function messageResponseToClient(msg: ChatMessageResponse): Message {
	return {
		id: msg.id,
		conversationId: msg.conversationId,
		threadId: msg.threadId ?? '',
		role: msg.role,
		content: msg.content,
		timestamp: new Date(msg.timestamp),
		status: msg.status,
		metadata: msg.metadata
			? {
					intent: msg.metadata.intent,
					confidence: msg.metadata.confidence,
					tokens: msg.metadata.tokens,
					model: msg.metadata.model
				}
			: undefined
	};
}

// ============================================================================
// Event Type Guards
// ============================================================================

/**
 * Type guard for started event.
 */
export function isStartedEvent(event: LlmEvent): event is LlmStartedEvent {
	return event.type === 'started';
}

/**
 * Type guard for chunk event.
 */
export function isChunkEvent(event: LlmEvent): event is LlmChunkEvent {
	return event.type === 'chunk';
}

/**
 * Type guard for complete event.
 */
export function isCompleteEvent(event: LlmEvent): event is LlmCompleteEvent {
	return event.type === 'complete';
}

/**
 * Type guard for error event.
 */
export function isErrorEvent(event: LlmEvent): event is LlmErrorEvent {
	return event.type === 'error';
}
