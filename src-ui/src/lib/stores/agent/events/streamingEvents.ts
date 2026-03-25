/**
 * Streaming Events
 * Handles streaming content, status phrase, and context compaction events.
 */

import type { AgentEvent } from '$lib/types/agent';
import type { AgentState } from '../state';
import { scheduleStreamingUpdate, clearStreamingUpdates } from '../streaming';
import { getOnSystemMessage, getOnAgentActivity } from '../callbacks';

/**
 * Handle streaming_content event.
 */
export function handleStreamingContent(state: AgentState, event: Extract<AgentEvent, { type: 'streaming_content' }>) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		// For streaming content, we use a special message ID for the assistant response
		const assistantMessageId = `streaming-assistant-${event.sessionId}`;

		// Track accumulated content in session state
		const existingContent = session.streamingAssistantContent ?? '';
		const newContent = existingContent + event.chunk;

		if (!event.isFinal) {
			// Update streaming state
			state.sessions.set(event.sessionId, {
				...session,
				streamingAssistantContent: newContent
			});

			// Schedule throttled UI update (non-blocking)
			scheduleStreamingUpdate(
				event.sessionId,
				session.conversationId,
				session.threadId,
				assistantMessageId,
				newContent,
				'sending'
			);
		}
		// is_final: flush immediately with 'sent' status and clear streaming state
		else {
			// Clear any pending throttled update
			clearStreamingUpdates(event.sessionId);

			// Clear streaming state
			state.sessions.set(event.sessionId, {
				...session,
				streamingAssistantContent: undefined
			});

			// If we have accumulated content, flush immediately
			if (newContent.length > 0) {
				scheduleStreamingUpdate(
					event.sessionId,
					session.conversationId,
					session.threadId,
					assistantMessageId,
					newContent,
					'sent',
					true // immediate flush
				);
			}
		}
	}
}

/**
 * Handle status_phrase event.
 */
export function handleStatusPhrase(state: AgentState, event: Extract<AgentEvent, { type: 'status_phrase' }>) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		// Accumulate status phrases for rotation (keep last 10)
		const existingPhrases = session.statusPhrases ?? [];
		// Keep only the last 10 phrases
		const newPhrases = [...existingPhrases, event.message].slice(-10);
		state.sessions.set(event.sessionId, {
			...session,
			statusPhrases: newPhrases
		});

		// Update thinking message in chat with latest status
		const onAgentActivity = getOnAgentActivity();
		if (onAgentActivity) {
			onAgentActivity(session.conversationId, session.threadId, {
				id: `thinking-${event.sessionId}`,
				conversationId: session.conversationId,
				threadId: session.threadId,
				role: 'assistant',
				kind: 'thinking',
				content: event.message,
				reasoning: event.message, // Map to reasoning for better visibility
				timestamp: new Date()
			});
		}
	}
}

/**
 * Handle context_compacted event.
 */
export function handleContextCompacted(state: AgentState, event: Extract<AgentEvent, { type: 'context_compacted' }>) {
	const onSystemMessage = getOnSystemMessage();
	if (onSystemMessage) {
		const summary = `🗜️ **Context compacted**\n\n${event.beforeCount} → ${event.afterCount} messages\n${event.beforeTokens.toLocaleString()} → ${event.afterTokens.toLocaleString()} tokens`;
		onSystemMessage(event.conversationId, summary);
	}
}
