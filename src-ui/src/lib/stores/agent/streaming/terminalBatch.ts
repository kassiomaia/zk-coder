/**
 * Terminal Output Batching
 * Batches terminal output lines to prevent flooding the chat.
 */

import type {Message} from '$lib/types/message';

// Pending terminal output waiting to be batched
interface PendingTerminalOutput {
	conversationId: string;
	threadId: string;
	lines: string[];
	timer: ReturnType<typeof setTimeout> | null;
}

const pendingTerminalOutput = new Map<string, PendingTerminalOutput>();
export const TERMINAL_BATCH_MS = 100; // Batch terminal output for 100ms

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
 * Flush pending terminal output to the UI as a single message.
 */
export function flushTerminalOutput(sessionId: string) {
	const pending = pendingTerminalOutput.get(sessionId);
	if (pending && pending.lines.length > 0 && onAgentActivity) {
		// Combine all lines into a single message
		const combinedContent = pending.lines.join('\n');
		onAgentActivity(pending.conversationId, pending.threadId, {
			id: `terminal-${sessionId}-${Date.now()}`,
			conversationId: pending.conversationId,
			threadId: pending.threadId,
			role: 'assistant',
			kind: 'terminal',
			content: combinedContent,
			agentData: {
				sessionId
			},
			timestamp: new Date()
		});
		// Clear pending lines
		pending.lines = [];
	}
}

/**
 * Schedule a batched terminal output update.
 */
export function scheduleTerminalOutput(
	sessionId: string,
	conversationId: string,
	threadId: string,
	line: string
) {
	let pending = pendingTerminalOutput.get(sessionId);
	if (!pending) {
		pending = {
			conversationId,
			threadId,
			lines: [],
			timer: null
		};
		pendingTerminalOutput.set(sessionId, pending);
	}

	// Add the line
	pending.lines.push(line);

	// Clear existing timer
	if (pending.timer) {
		clearTimeout(pending.timer);
	}

	// Schedule batched update
	pending.timer = setTimeout(() => {
		flushTerminalOutput(sessionId);
		if (pending) pending.timer = null;
	}, TERMINAL_BATCH_MS);
}

/**
 * Flush any pending terminal output immediately.
 */
export function flushPendingTerminalOutput(sessionId: string) {
	const pending = pendingTerminalOutput.get(sessionId);
	if (pending?.timer) {
		clearTimeout(pending.timer);
		pending.timer = null;
	}
	flushTerminalOutput(sessionId);
}

/**
 * Clear all pending terminal output for a session.
 */
export function clearTerminalOutput(sessionId: string) {
	const pending = pendingTerminalOutput.get(sessionId);
	if (pending?.timer) {
		clearTimeout(pending.timer);
	}
	pendingTerminalOutput.delete(sessionId);
}
