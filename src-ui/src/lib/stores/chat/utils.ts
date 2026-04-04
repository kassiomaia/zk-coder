/**
 * Chat Store Utility Functions
 */

import type { Conversation, Thread } from '$lib/types/thread';
import type { Message } from '$lib/types/message';
import { generateUniqueId } from '$lib/utils/idGenerator';

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
