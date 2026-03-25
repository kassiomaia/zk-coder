/**
 * Streaming Throttle
 * Manages throttled streaming updates to prevent UI freezing.
 */

import type { Message } from '$lib/types/message';

// Pending streaming updates waiting to be flushed
interface PendingStreamingUpdate {
	conversationId: string;
	threadId: string;
	messageId: string;
	content: string;
	status: string;
}

const pendingStreamingUpdates = new Map<string, PendingStreamingUpdate>();
const streamingThrottleTimers = new Map<string, ReturnType<typeof setTimeout>>();
export const STREAM_THROTTLE_MS = 16; // Update UI at ~60fps (16ms) for smoother streaming

/** Callback for agent activity messages */
let onAgentActivity:
	| ((conversationId: string, threadId: string, message: Partial<Message>) => void)
	| null = null;

/**
 * Set the agent activity callback.
 */
export function setOnAgentActivity(
	callback: (conversationId: string, threadId: string, message: Partial<Message>) => void
) {
	onAgentActivity = callback;
}

/**
 * Flush a pending streaming update to the UI.
 */
export function flushStreamingUpdate(sessionId: string) {
	const pending = pendingStreamingUpdates.get(sessionId);
	if (pending && onAgentActivity) {
		onAgentActivity(pending.conversationId, pending.threadId, {
			id: pending.messageId,
			conversationId: pending.conversationId,
			threadId: pending.threadId,
			role: 'assistant',
			kind: 'normal', // Mark as normal so it updates in-place
			content: pending.content,
			timestamp: new Date(),
			status: pending.status as any
		});
		pendingStreamingUpdates.delete(sessionId);
	}
}

/**
 * Schedule a throttled streaming update.
 */
export function scheduleStreamingUpdate(
	sessionId: string,
	conversationId: string,
	threadId: string,
	messageId: string,
	content: string,
	status: string,
	immediate = false
) {
	// Store the update
	pendingStreamingUpdates.set(sessionId, {
		conversationId,
		threadId,
		messageId,
		content,
		status
	});

	// Clear existing timer
	const existingTimer = streamingThrottleTimers.get(sessionId);
	if (existingTimer) {
		clearTimeout(existingTimer);
	}

	if (immediate) {
		// Flush immediately for final chunk
		flushStreamingUpdate(sessionId);
		streamingThrottleTimers.delete(sessionId);
	} else {
		// Schedule throttled update
		const timer = setTimeout(() => {
			flushStreamingUpdate(sessionId);
			streamingThrottleTimers.delete(sessionId);
		}, STREAM_THROTTLE_MS);
		streamingThrottleTimers.set(sessionId, timer);
	}
}

/**
 * Clear all pending streaming updates for a session.
 */
export function clearStreamingUpdates(sessionId: string) {
	const timer = streamingThrottleTimers.get(sessionId);
	if (timer) {
		clearTimeout(timer);
		streamingThrottleTimers.delete(sessionId);
	}
	pendingStreamingUpdates.delete(sessionId);
}
