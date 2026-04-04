/**
 * State Update Utilities
 * Helper functions for common state mutations in Svelte stores.
 */

import type { Thread, Conversation } from '$lib/types/thread';

/**
 * Options for touch operations.
 */
export interface TouchOptions {
	/** Number to add to message count (positive or negative) */
	messageDelta?: number;
	/** Custom timestamp (defaults to now) */
	timestamp?: Date;
}

/**
 * Updates a thread's timestamp and message count in the threads array.
 */
export function touchThread<T extends { id: string; updatedAt: Date; messageCount: number }>(
	threads: T[],
	threadId: string,
	options: TouchOptions = {}
): T[] {
	const now = options.timestamp ?? new Date();
	const messageDelta = options.messageDelta ?? 0;

	return threads.map((thread) =>
		thread.id === threadId
			? {
					...thread,
					updatedAt: now,
					messageCount: Math.max(0, thread.messageCount + messageDelta)
				}
			: thread
	);
}

/**
 * Updates a conversation's timestamp and optionally a specific thread within it.
 */
export function touchConversationThread<
	TThread extends { id: string; updatedAt: Date; messageCount: number },
	TConversation extends { id: string; updatedAt: Date; threads?: TThread[] }
>(
	conversations: TConversation[],
	conversationId: string,
	threadId: string | null,
	options: TouchOptions = {}
): TConversation[] {
	const now = options.timestamp ?? new Date();
	const messageDelta = options.messageDelta ?? 0;

	return conversations.map((conversation) =>
		conversation.id === conversationId
			? {
					...conversation,
					updatedAt: now,
					threads: conversation.threads
						? conversation.threads.map((thread) =>
								thread.id === threadId
									? {
											...thread,
											updatedAt: now,
											messageCount: Math.max(0, thread.messageCount + messageDelta)
										}
									: thread
							)
						: undefined
				}
			: conversation
	);
}

/**
 * Touches both thread and its parent conversation simultaneously.
 * Returns updated state objects.
 */
export function touchThreadAndConversation<
	TThread extends { id: string; conversationId: string; updatedAt: Date; messageCount: number },
	TConversation extends { id: string; updatedAt: Date; threads?: TThread[] }
>(
	threads: TThread[],
	conversations: TConversation[],
	threadId: string,
	conversationId: string,
	options: TouchOptions = {}
): { threads: TThread[]; conversations: TConversation[] } {
	const now = options.timestamp ?? new Date();
	const messageDelta = options.messageDelta ?? 0;

	const updatedThreads = touchThread(threads, threadId, { timestamp: now, messageDelta });
	const updatedConversations = touchConversationThread(
		conversations,
		conversationId,
		threadId,
		{ timestamp: now, messageDelta }
	);

	return { threads: updatedThreads, conversations: updatedConversations };
}

/**
 * Clears unread status for a specific thread ID from an array of thread IDs.
 */
export function clearUnreadForThread(unreadThreadIds: string[], threadId: string | null): string[] {
	if (!threadId) return unreadThreadIds;
	return unreadThreadIds.filter((id) => id !== threadId);
}

/**
 * Creates an optimistic (temporary) item with a timestamp-based ID.
 */
export function createOptimisticId(): string {
	return `temp-${Date.now()}`;
}