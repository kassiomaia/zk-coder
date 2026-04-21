/**
 * Chat Store Utility Functions
 */

import type { Conversation, Thread } from '$lib/types/thread';
import type { Message } from '$lib/types/message';
import { generateUniqueId } from '$lib/utils/idGenerator';
import type { ChatState } from './types';

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Creates a new conversation object.
 */
export function createNewConversation(title: string = 'New Conversation'): Conversation {
	const now = new Date();
	return {
		id: generateUniqueId('conv'),
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
		id: generateUniqueId('thread'),
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
		id: generateUniqueId('msg'),
		conversationId,
		threadId,
		role,
		content,
		timestamp: new Date(),
		status: 'sending'
	};
}

// ============================================================================
// Title Derivation
// ============================================================================

/**
 * Derives a thread title from message content.
 * Uses first ~50 characters, cleaned up for display.
 */
export function deriveThreadTitle(content: string): string {
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
}

// ============================================================================
// Sorting Helpers
// ============================================================================

/**
 * Sort threads by most recent first.
 */
export function sortThreadsByRecent<T extends { updatedAt: string | Date }>(threads: T[]): T[] {
	return [...threads].sort(
		(a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
	);
}

// ============================================================================
// State Mutation Helpers
// ============================================================================

/**
 * Bumps the `updatedAt` of a thread and its parent conversation, and
 * optionally adjusts the thread's `messageCount`.
 *
 * Returns a new `ChatState` — caller is responsible for passing it into
 * the store's `update`.
 */
export function touchThreadAndConversation(
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
