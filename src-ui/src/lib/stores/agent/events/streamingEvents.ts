/**
 * Streaming Events
 * Handles streaming content, status phrase, and context compaction events.
 */

import type { AgentEvent } from '$lib/types/agent';
import type { AgentState } from '../state';
import { clearStreamingUpdates, scheduleStreamingUpdate } from '../streaming';
import { getOnAgentActivity, getOnPersistAssistantMessage, getOnSystemMessage } from '../callbacks';
import { buildContextNoticeContent } from '$lib/utils/systemNotices';
import { createLogger } from '$lib/utils/logger';

const log = createLogger('streamingEvents');

/**
 * Generates a unique, per-event fallback id when the backend omits `messageId`.
 *
 * The legacy behaviour collapsed every such event onto `streaming-assistant-{sessionId}`,
 * which caused successive subtask outputs to overwrite each other in the chat store.
 * This fallback guarantees the frontend never silently merges two unrelated messages;
 * a backend regression that drops `messageId` will show up as extra messages (loud)
 * rather than as lost messages (silent).
 */
function generateOrphanStreamingId(sessionId: string): string {
	const unique =
		typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
			? crypto.randomUUID()
			: `${Date.now()}-${Math.random().toString(36).slice(2)}`;
	return `streaming-orphan-${sessionId}-${unique}`;
}

/**
 * Handle streaming_content event.
 */
export function handleStreamingContent(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'streaming_content' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		let assistantMessageId: string;
		if (event.messageId) {
			assistantMessageId = event.messageId;
		} else {
			assistantMessageId = generateOrphanStreamingId(event.sessionId);
			log.warn(
				'streaming_content event arrived without messageId; using unique fallback id',
				{ sessionId: event.sessionId, assistantMessageId }
			);
		}
		const streamingKey = `${event.sessionId}:${assistantMessageId}`;
		const existingMessages = session.streamingMessages ?? {};
		const existingContent = existingMessages[assistantMessageId] ?? '';
		const newContent = existingContent + event.chunk;

		if (!event.isFinal) {
			state.sessions.set(event.sessionId, {
				...session,
				streamingMessages: {
					...existingMessages,
					[assistantMessageId]: newContent
				}
			});

			scheduleStreamingUpdate(
				streamingKey,
				session.conversationId,
				session.threadId,
				assistantMessageId,
				newContent,
				'sending'
			);
		} else {
			clearStreamingUpdates(streamingKey);
			const { [assistantMessageId]: _removed, ...remainingMessages } = existingMessages;
			state.sessions.set(event.sessionId, {
				...session,
				streamingMessages: Object.keys(remainingMessages).length > 0 ? remainingMessages : undefined
			});

			if (newContent.length > 0) {
				scheduleStreamingUpdate(
					streamingKey,
					session.conversationId,
					session.threadId,
					assistantMessageId,
					newContent,
					'sent',
					true
				);
			}

			if (event.persistMessage && newContent.trim().length > 0) {
				const onPersistAssistantMessage = getOnPersistAssistantMessage();
				if (onPersistAssistantMessage) {
					onPersistAssistantMessage(
						event.sessionId,
						session.conversationId,
						session.threadId,
						assistantMessageId,
						newContent
					);
				}
			}
		}
	}
}

/**
 * Handle status_phrase event.
 */
export function handleStatusPhrase(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'status_phrase' }>
) {
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
				timestamp: new Date(),
				agentData: {
					sessionId: event.sessionId
				}
			});
		}
	}
}

/**
 * Handle context_compacted event.
 */
export function handleContextCompacted(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'context_compacted' }>
) {
	// Defensive no-op: the backend no longer emits zero-delta compaction events,
	// but older backends or replays may still deliver them. Suppress here so the
	// chat never shows a misleading "N -> N tokens" notice.
	if (event.beforeTokens === event.afterTokens) {
		return;
	}

	const session = state.sessions.get(event.sessionId);
	const onSystemMessage = getOnSystemMessage();
	if (onSystemMessage && session) {
		onSystemMessage(event.sessionId, event.conversationId, session.threadId, {
			role: 'system',
			content: buildContextNoticeContent(event),
			metadata: {
				systemNotice: {
					type: 'context_update',
					reason: event.reason,
					beforeCount: event.beforeCount,
					afterCount: event.afterCount,
					beforeTokens: event.beforeTokens,
					afterTokens: event.afterTokens
				}
			}
		});
	}
}

/**
 * Handle agent_narration event.
 * Narrations are persistent natural-language updates that appear in the chat.
 */
export function handleAgentNarration(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'agent_narration' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (!session) return;

	const onAgentActivity = getOnAgentActivity();
	if (onAgentActivity) {
		onAgentActivity(event.conversationId, event.threadId, {
			id: `narration-${event.sessionId}-${Date.now()}`,
			conversationId: event.conversationId,
			threadId: event.threadId,
			role: 'assistant',
			kind: 'narration',
			content: event.content,
			timestamp: new Date(event.timestamp),
			agentData: { sessionId: event.sessionId }
		});
	}
}
