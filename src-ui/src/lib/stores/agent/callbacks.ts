/**
 * Callbacks
 * Manages callbacks for cross-store communication.
 */

import type {Message} from '$lib/types/message';

/** Callback for when agent completes execution - returns (conversationId, threadId, content) */
let onExecutionComplete:
	| ((conversationId: string, threadId: string, content: string) => void)
	| null = null;

/** Callback for agent system messages (e.g., context compaction) - returns (conversationId, message) */
let onSystemMessage: ((conversationId: string, message: string) => void) | null = null;

/** Callback for agent activity messages (thinking, tool_call, terminal) - adds transient messages to chat */
let onAgentActivity:
	| ((conversationId: string, threadId: string, message: Partial<Message>) => void)
	| null = null;

/** Callback to clean up transient messages from chat after execution completes */
let onCleanupTransients:
	| ((conversationId: string, threadId: string, sessionId: string) => void)
	| null = null;

/**
 * Set the execution complete callback.
 */
export function setExecutionCompleteCallback(
	callback: (conversationId: string, threadId: string, content: string) => void
) {
	onExecutionComplete = callback;
}

/**
 * Set the system message callback.
 */
export function setSystemMessageCallback(callback: (conversationId: string, message: string) => void) {
	onSystemMessage = callback;
}

/**
 * Set the agent activity callback.
 */
export function setAgentActivityCallback(
	callback: (conversationId: string, threadId: string, message: Partial<Message>) => void
) {
	onAgentActivity = callback;
}

/**
 * Set the cleanup transients callback.
 */
export function setCleanupTransientCallback(
	callback: (conversationId: string, threadId: string, sessionId: string) => void
) {
	onCleanupTransients = callback;
}

/**
 * Get the execution complete callback.
 */
export function getOnExecutionComplete() {
	return onExecutionComplete;
}

/**
 * Get the system message callback.
 */
export function getOnSystemMessage() {
	return onSystemMessage;
}

/**
 * Get the agent activity callback.
 */
export function getOnAgentActivity() {
	return onAgentActivity;
}

/**
 * Get the cleanup transients callback.
 */
export function getOnCleanupTransients() {
	return onCleanupTransients;
}
