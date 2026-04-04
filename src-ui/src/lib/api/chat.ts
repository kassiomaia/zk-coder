/**
 * Chat Persistence API Client
 * TypeScript wrappers for Tauri IPC commands to the chat persistence backend.
 */

import { invokeCommand } from './core';
import type { Conversation, Thread } from '$lib/types/thread';
import type { Message } from '$lib/types/message';
import { parseSystemNoticeFromMessage } from '$lib/utils/systemNotices';
import { parseDate } from '$lib/utils/date';

// Re-export error for backwards compatibility
export { ChatApiError } from './errors';

// ============================================================================
// Types
// ============================================================================

/**
 * Chat message from the backend.
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
		feedback?: 'like' | 'dislike';
	};
}

/**
 * Conversation from the backend.
 */
export interface ConversationResponse {
	id: string;
	title: string;
	createdAt: string;
	updatedAt: string;
	archived: boolean;
	settings: {
		model: string;
		temperature: number;
		maxTokens: number;
		systemPrompt?: string;
		workspaceRoot?: string;
	};
}

/**
 * Thread from the backend.
 */
export interface ThreadResponse {
	id: string;
	conversationId: string;
	parentId: string | null;
	title: string;
	createdAt: string;
	updatedAt: string;
	messageCount: number;
	metadata: {
		resolved: boolean;
		archived: boolean;
		tags: string[];
		priority: 'low' | 'medium' | 'high';
		intent?: string;
		summary?: string;
		agentSessionId?: string;
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
	const response = await invokeCommand<ConversationResponse[]>('chat', 'load_conversations');
	return response.map(convToClient);
}

/**
 * Creates a new conversation.
 *
 * @param title - Conversation title
 * @returns The created conversation
 */
export async function createConversation(title: string): Promise<Conversation> {
	const response = await invokeCommand<ConversationResponse>('chat', 'create_conversation', { title });
	return convToClient(response);
}

/**
 * Loads a single conversation by ID.
 *
 * @param id - Conversation ID
 * @returns The conversation
 */
export async function loadConversation(id: string): Promise<Conversation> {
	const response = await invokeCommand<ConversationResponse>('chat', 'load_conversation', { id });
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
	const response = await invokeCommand<ConversationResponse>('chat', 'rename_conversation', { id, title });
	return convToClient(response);
}

/**
 * Deletes a conversation.
 *
 * @param id - Conversation ID
 */
export async function deleteConversation(id: string): Promise<void> {
	await invokeCommand<void>('chat', 'delete_conversation', { id });
}

/**
 * Archives or restores a conversation.
 *
 * @param id - Conversation ID
 * @param archived - Whether the conversation should be archived
 */
export async function archiveConversation(id: string, archived: boolean): Promise<void> {
	await invokeCommand<void>('chat', 'archive_conversation', { id, archived });
}

/**
 * Settings for updating a conversation.
 */
export interface ConversationSettingsUpdate {
	/** AI model to use */
	model: string;
	/** Temperature for response generation */
	temperature: number;
	/** Maximum tokens for responses */
	maxTokens: number;
	/** Custom system prompt */
	systemPrompt?: string;
	/** Workspace directory for agent operations */
	workspaceRoot?: string;
}

/**
 * Updates a conversation's settings.
 *
 * @param id - Conversation ID
 * @param settings - New settings to apply
 * @returns The updated conversation
 */
export async function updateConversationSettings(
	id: string,
	settings: ConversationSettingsUpdate
): Promise<Conversation> {
		const response = await invokeCommand<ConversationResponse>('chat', 'update_conversation_settings', {
		id,
		settings: {
			model: settings.model,
			temperature: settings.temperature,
			maxTokens: settings.maxTokens,
			systemPrompt: settings.systemPrompt,
			workspaceRoot: settings.workspaceRoot
		}
	});
	return convToClient(response);
}

/**
 * Sets a conversation's workspace root.
 *
 * @param id - Conversation ID
 * @param workspaceRoot - Workspace directory path
 * @returns The updated conversation
 */
export async function setConversationWorkspace(
	id: string,
	workspaceRoot: string
): Promise<Conversation> {
	const response = await invokeCommand<ConversationResponse>('chat', 'set_conversation_workspace', {
		id,
		workspaceRoot
	});
	return convToClient(response);
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
	const response = await invokeCommand<ThreadResponse[]>('chat', 'load_threads', { conversationId });
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
	const response = await invokeCommand<ThreadResponse>('chat', 'create_thread', { conversationId, title });
	return threadToClient(response);
}

/**
 * Updates a thread's resolved status.
 *
 * @param id - Thread ID
 * @param resolved - Whether the thread is resolved
 */
export async function resolveThread(id: string, resolved: boolean): Promise<void> {
	await invokeCommand<void>('chat', 'resolve_thread', { id, resolved });
}

/**
 * Archives or restores a thread.
 *
 * @param id - Thread ID
 * @param archived - Whether the thread should be archived
 */
export async function archiveThread(id: string, archived: boolean): Promise<void> {
	await invokeCommand<void>('chat', 'archive_thread', { id, archived });
}

/**
 * Deletes a thread.
 *
 * @param id - Thread ID
 */
export async function deleteThread(id: string): Promise<void> {
	await invokeCommand<void>('chat', 'delete_thread', { id });
}

/**
 * Renames a thread.
 *
 * @param id - Thread ID
 * @param title - New title
 * @returns The updated thread
 */
export async function renameThread(id: string, title: string): Promise<Thread> {
	const response = await invokeCommand<ThreadResponse>('chat', 'rename_thread', { id, title });
	return threadToClient(response);
}

/**
 * Sets or clears the agent session for a thread.
 *
 * @param id - Thread ID
 * @param agentSessionId - Agent session ID, or null to clear
 */
export async function setThreadAgentSession(
	id: string,
	agentSessionId: string | null
): Promise<void> {
	await invokeCommand<void>('chat', 'set_thread_agent_session', { id, agentSessionId });
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
	const response = await invokeCommand<ChatMessageResponse[]>('chat', 'load_messages', { conversationId });
	return response.map(msgToClient);
}

/**
 * Loads all messages for a specific thread.
 *
 * @param threadId - Thread ID
 * @returns Array of messages
 */
export async function loadThreadMessages(threadId: string): Promise<Message[]> {
	const response = await invokeCommand<ChatMessageResponse[]>('chat', 'load_thread_messages', { threadId });
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
	/** Thread ID (required - every message must belong to a thread) */
	threadId: string;
}

/**
 * Sends a message to a conversation.
 *
 * @param options - Message options
 * @returns The created message
 */
export async function sendMessage(options: SendMessageOptions): Promise<Message> {
	const response = await invokeCommand<ChatMessageResponse>('chat', 'send_message', {
		conversationId: options.conversationId,
		content: options.content,
		role: options.role,
		threadId: options.threadId
	});
	return msgToClient(response);
}

/**
 * Options for creating a thread with a message.
 */
export interface CreateThreadWithMessageOptions {
	/** Conversation ID */
	conversationId: string;
	/** Thread title */
	threadTitle: string;
	/** Message content */
	content: string;
	/** Role (user, assistant, system) */
	role: 'user' | 'assistant' | 'system';
}

/**
 * Response from creating a thread with a message.
 */
export interface CreateThreadWithMessageResponse {
	/** The created thread */
	thread: ThreadResponse;
	/** The created message */
	message: ChatMessageResponse;
}

/**
 * Creates a thread with its first message atomically.
 *
 * @param options - Creation options
 * @returns The created thread and message
 */
export async function createThreadWithMessage(
	options: CreateThreadWithMessageOptions
): Promise<{ thread: Thread; message: Message }> {
	const response = await invokeCommand<CreateThreadWithMessageResponse>('chat', 'create_thread_with_message', {
		conversationId: options.conversationId,
		threadTitle: options.threadTitle,
		content: options.content,
		role: options.role
	});
	return {
		thread: threadToClient(response.thread),
		message: msgToClient(response.message)
	};
}

/**
 * Deletes a message.
 *
 * @param id - Message ID
 */
export async function deleteMessage(id: string): Promise<void> {
	await invokeCommand<void>('chat', 'delete_message', { id });
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
	await invokeCommand<void>('chat', 'update_message_status', { id, status });
}

/**
 * Updates a message's feedback (like/dislike).
 *
 * @param id - Message ID
 * @param feedback - Feedback type ('like', 'dislike', or null to clear)
 */
export async function updateMessageFeedback(
	id: string,
	feedback: 'like' | 'dislike' | null
): Promise<void> {
	await invokeCommand<void>('chat', 'update_message_feedback', { id, feedback });
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
		createdAt: parseDate(conv.createdAt) ?? new Date(),
		updatedAt: parseDate(conv.updatedAt) ?? new Date(),
		archived: conv.archived,
		threads: [],
		settings: {
			model: conv.settings.model,
			temperature: conv.settings.temperature,
			maxTokens: conv.settings.maxTokens,
			systemPrompt: conv.settings.systemPrompt,
			workspaceRoot: conv.settings.workspaceRoot
		}
	};
}

/**
 * Converts a backend thread to client format.
 */
function threadToClient(thread: ThreadResponse): Thread {
	return {
		id: thread.id,
		conversationId: thread.conversationId,
		parentId: thread.parentId,
		title: thread.title,
		createdAt: parseDate(thread.createdAt) ?? new Date(),
		updatedAt: parseDate(thread.updatedAt) ?? new Date(),
		messageCount: thread.messageCount,
		metadata: {
			resolved: thread.metadata.resolved,
			archived: thread.metadata.archived,
			tags: thread.metadata.tags,
			priority: thread.metadata.priority,
			intent: thread.metadata.intent,
			summary: thread.metadata.summary,
			agentSessionId: thread.metadata.agentSessionId
		}
	};
}

/**
 * Converts a backend message to client format.
 */
function msgToClient(msg: ChatMessageResponse): Message {
	const parsedSystemNotice = parseSystemNoticeFromMessage(msg.role, msg.content);

	return {
		id: msg.id,
		conversationId: msg.conversationId,
		threadId: msg.threadId ?? '',
		role: msg.role,
		content: msg.content,
		timestamp: parseDate(msg.timestamp) ?? new Date(),
		status: msg.status,
		metadata: msg.metadata
			? {
					intent: msg.metadata.intent,
					confidence: msg.metadata.confidence,
					tokens: msg.metadata.tokens,
					model: msg.metadata.model,
					feedback: msg.metadata.feedback,
					systemNotice: parsedSystemNotice
				}
			: parsedSystemNotice
				? { systemNotice: parsedSystemNotice }
				: undefined
	};
}
