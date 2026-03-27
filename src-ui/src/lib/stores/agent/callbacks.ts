/**
 * Callbacks
 * Manages callbacks for cross-store communication.
 */

import type {Message} from '$lib/types/message';
import type {ArtifactReference} from '$lib/types/agent';

export interface SystemMessagePayload {
	role: 'system';
	content: string;
	metadata?: Message['metadata'];
}

/** Callback for when agent completes execution - returns (conversationId, threadId, content, artifacts) */
let onExecutionComplete:
	| ((sessionId: string, conversationId: string, threadId: string, content: string, artifacts: ArtifactReference[]) => void)
	| null = null;

/** Callback for agent system messages (e.g., context compaction) - returns (conversationId, message) */
let onSystemMessage:
	| ((sessionId: string, conversationId: string, threadId: string, message: SystemMessagePayload) => void)
	| null = null;

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
	callback: (sessionId: string, conversationId: string, threadId: string, content: string, artifacts: ArtifactReference[]) => void
) {
	onExecutionComplete = callback;
}

/**
 * Set the system message callback.
 */
export function setSystemMessageCallback(
	callback: (
		sessionId: string,
		conversationId: string,
		threadId: string,
		message: SystemMessagePayload
	) => void
) {
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
