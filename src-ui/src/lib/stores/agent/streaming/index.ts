/**
 * Streaming Module Index
 * Exports all streaming functionality.
 */

import type {Message} from '$lib/types/message';
import {setOnAgentActivity as setStreamingCallback} from './streamingThrottle';
import {setOnAgentActivity as setTerminalCallback} from './terminalBatch';

export {
	setOnAgentActivity,
	flushStreamingUpdate,
	scheduleStreamingUpdate,
	clearStreamingUpdates,
	STREAM_THROTTLE_MS
} from './streamingThrottle';

export {
	setOnAgentActivity as setOnAgentActivityTerminal,
	flushTerminalOutput,
	scheduleTerminalOutput,
	flushPendingTerminalOutput,
	clearTerminalOutput,
	TERMINAL_BATCH_MS
} from './terminalBatch';

/**
 * Initialize streaming with the agent activity callback.
 */
export function initializeStreaming(
	callback: (conversationId: string, threadId: string, message: Partial<Message>) => void
) {
	setStreamingCallback(callback);
	setTerminalCallback(callback);
}
