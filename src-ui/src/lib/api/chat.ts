/**
 * Chat Persistence API Client
 * TypeScript wrappers for Tauri IPC commands to the chat persistence backend.
 */

import { invoke } from '@tauri-apps/api/core';
import type { Conversation, Thread } from '$lib/types/thread';
import type { Message } from '$lib/types/message';

// ============================================================================
// Error Handling
// ============================================================================

/**
 * Error class for chat API errors.
 */
export class ChatApiError extends Error {
	constructor(
		message: string,
		public readonly command: string,
		public readonly cause?: unknown
	) {
		super(message);
		this.name = 'ChatApiError';
	}
}

/**
 * Wraps invoke with error handling.
 */
async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await invoke<T>(command, args);
	} catch (error) {
		throw new ChatApiError(
			`Chat command '${command}' failed: ${error instanceof Error ? error.message : String(error)}`,
			command,
			error
		);
	}
}

// ============================================================================
// Types
// ============================================================================

/**
 * Chat message from the backend.
 */
export interface ChatMessageResponse {
	id: string;
	conversation_id: string;
	thread_id: string | null;
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

/**
 * Conversation from the backend.
 */
export interface ConversationResponse {
	id: string;
	title: string;
	created_at: string;
	updated_at: string;
	settings: {
		model: string;
		temperature: number;
		max_tokens: number;
		system_prompt?: string;
	};
}

/**
 * Thread from the backend.
 */
export interface ThreadResponse {
	id: string;
	conversation_id: string;
	parent_id: string | null;
	title: string;
	created_at: string;
	updated_at: string;
	message_count: number;
	metadata: {
		resolved: boolean;
		tags: string[];
		priority: 'low' | 'medium' | 'high';
		intent?: string;
		summary?: string;
	};
}

// ============================================================================
// Conversation Commands
// ============================================================================

/**
 * Loads all conversations.
 *
 * @returns Array of conversations
 */
export async function loadConversations(): Promise<Conversation[]> {
	const response = await safeInvoke<ConversationResponse[]>('load_conversations');
	return response.map(convToClient);
}

/**
 * Creates a new conversation.
 *
 * @param title - Conversation title
 * @returns The created conversation
 */
export async function createConversation(title: string): Promise<Conversation> {
	const response = await safeInvoke<ConversationResponse>('create_conversation', { title });
	return convToClient(response);
}

/**
 * Loads a single conversation by ID.
 *
 * @param id - Conversation ID
 * @returns The conversation
 */
export async function loadConversation(id: string): Promise<Conversation> {
	const response = await safeInvoke<ConversationResponse>('load_conversation', { id });
	return convToClient(response);
}

/**
 * Renames a conversation.
 *
 * @param id - Conversation ID
 * @param title - New title
 * @returns The updated conversation
 */
export async function renameConversation(id: string, title: string): Promise<Conversation> {
	const response = await safeInvoke<ConversationResponse>('rename_conversation', { id, title });
	return convToClient(response);
}

/**
 * Deletes a conversation.
 *
 * @param id - Conversation ID
 */
export async function deleteConversation(id: string): Promise<void> {
	await safeInvoke<void>('delete_conversation', { id });
}

// ============================================================================
// Thread Commands
// ============================================================================

/**
 * Loads all threads for a conversation.
 *
 * @param conversationId - Conversation ID
 * @returns Array of threads
 */
export async function loadThreads(conversationId: string): Promise<Thread[]> {
	const response = await safeInvoke<ThreadResponse[]>('load_threads', {
		conversation_id: conversationId
	});
	return response.map(threadToClient);
}

/**
 * Creates a new thread in a conversation.
 *
 * @param conversationId - Conversation ID
 * @param title - Thread title
 * @returns The created thread
 */
export async function createThread(conversationId: string, title: string): Promise<Thread> {
	const response = await safeInvoke<ThreadResponse>('create_thread', {
		conversation_id: conversationId,
		title
	});
	return threadToClient(response);
}

/**
 * Updates a thread's resolved status.
 *
 * @param id - Thread ID
 * @param resolved - Whether the thread is resolved
 */
export async function resolveThread(id: string, resolved: boolean): Promise<void> {
	await safeInvoke<void>('resolve_thread', { id, resolved });
}

/**
 * Deletes a thread.
 *
 * @param id - Thread ID
 */
export async function deleteThread(id: string): Promise<void> {
	await safeInvoke<void>('delete_thread', { id });
}

// ============================================================================
// Message Commands
// ============================================================================

/**
 * Loads all messages for a conversation.
 *
 * @param conversationId - Conversation ID
 * @returns Array of messages
 */
export async function loadMessages(conversationId: string): Promise<Message[]> {
	const response = await safeInvoke<ChatMessageResponse[]>('load_messages', {
		conversation_id: conversationId
	});
	return response.map(msgToClient);
}

/**
 * Loads all messages for a specific thread.
 *
 * @param threadId - Thread ID
 * @returns Array of messages
 */
export async function loadThreadMessages(threadId: string): Promise<Message[]> {
	const response = await safeInvoke<ChatMessageResponse[]>('load_thread_messages', {
		thread_id: threadId
	});
	return response.map(msgToClient);
}

/**
 * Options for sending a message.
 */
export interface SendMessageOptions {
	/** Conversation ID */
	conversationId: string;
	/** Message content */
	content: string;
	/** Role (user, assistant, system) */
	role: 'user' | 'assistant' | 'system';
	/** Optional thread ID */
	threadId?: string;
}

/**
 * Sends a message to a conversation.
 *
 * @param options - Message options
 * @returns The created message
 */
export async function sendMessage(options: SendMessageOptions): Promise<Message> {
	const response = await safeInvoke<ChatMessageResponse>('send_message', {
		conversation_id: options.conversationId,
		content: options.content,
		role: options.role,
		thread_id: options.threadId ?? null
	});
	return msgToClient(response);
}

/**
 * Deletes a message.
 *
 * @param id - Message ID
 */
export async function deleteMessage(id: string): Promise<void> {
	await safeInvoke<void>('delete_message', { id });
}

/**
 * Updates a message's status.
 *
 * @param id - Message ID
 * @param status - New status
 */
export async function updateMessageStatus(
	id: string,
	status: 'sending' | 'sent' | 'delivered' | 'error'
): Promise<void> {
	await safeInvoke<void>('update_message_status', { id, status });
}

// ============================================================================
// Conversion Helpers
// ============================================================================

/**
 * Converts a backend conversation to client format.
 */
function convToClient(conv: ConversationResponse): Conversation {
	return {
		id: conv.id,
		title: conv.title,
		createdAt: new Date(conv.created_at),
		updatedAt: new Date(conv.updated_at),
		threads: [],
		settings: {
			model: conv.settings.model,
			temperature: conv.settings.temperature,
			maxTokens: conv.settings.max_tokens,
			systemPrompt: conv.settings.system_prompt
		}
	};
}

/**
 * Converts a backend thread to client format.
 */
function threadToClient(thread: ThreadResponse): Thread {
	return {
		id: thread.id,
		conversationId: thread.conversation_id,
		parentId: thread.parent_id,
		title: thread.title,
		createdAt: new Date(thread.created_at),
		updatedAt: new Date(thread.updated_at),
		messageCount: thread.message_count,
		metadata: {
			resolved: thread.metadata.resolved,
			tags: thread.metadata.tags,
			priority: thread.metadata.priority,
			intent: thread.metadata.intent,
			summary: thread.metadata.summary
		}
	};
}

/**
 * Converts a backend message to client format.
 */
function msgToClient(msg: ChatMessageResponse): Message {
	return {
		id: msg.id,
		conversationId: msg.conversation_id,
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
