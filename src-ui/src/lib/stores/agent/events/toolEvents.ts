/**
 * Tool Events
 * Handles tool call and tool result events.
 */

import type {AgentEvent} from '$lib/types/agent';
import type {AgentState} from '../state';

/**
 * Handle tool_call event.
 */
export function handleToolCall(state: AgentState, event: Extract<AgentEvent, { type: 'tool_call' }>) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		const record: import('$lib/types/agent').ToolCallRecord = {
			id: event.toolCallId,
			tool: event.tool,
			arguments: event.arguments,
			timestamp: new Date().toISOString()
		};

		state.sessions.set(event.sessionId, {
			...session,
			toolCalls: [...(session.toolCalls ?? []), record]
		});
	}
}

/**
 * Handle tool_result event.
 */
export function handleToolResult(state: AgentState, event: Extract<AgentEvent, { type: 'tool_result' }>) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		const toolCallId = event.toolCallId;
		const toolCalls = (session.toolCalls ?? []).map((call) => {
			if (call.id === toolCallId) {
				return {
					...call,
					result: event.result,
					isError: event.isError
				};
			}
			return call;
		});

		state.sessions.set(event.sessionId, {
			...session,
			toolCalls
		});
	}
}
