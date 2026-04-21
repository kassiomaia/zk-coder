/**
 * Tool Events
 * Handles tool call and tool result events.
 */

import type { AgentEvent } from '$lib/types/agent';
import { agentEventBus } from '../eventBus';
import type { AgentState } from '../state';

function getToolMessageId(sessionId: string, toolCallId: string): string {
	return `tool-call-${sessionId}-${toolCallId}`;
}

/**
 * Handle tool_call event.
 */
export function handleToolCall(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'tool_call' }>
) {
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

		agentEventBus.emit('agent-activity', {
			conversationId: session.conversationId,
			threadId: session.threadId,
			message: {
				id: getToolMessageId(event.sessionId, event.toolCallId),
				conversationId: session.conversationId,
				threadId: session.threadId,
				role: 'assistant',
				kind: 'tool_call',
				content: '',
				timestamp: new Date(),
				status: 'sending',
				agentData: {
					sessionId: event.sessionId,
					toolName: event.tool,
					toolArgs: event.arguments,
					toolCallStatus: 'pending',
					stepNumber: event.stepNumber
				}
			}
		});
	}
}

/**
 * Handle tool_result event.
 */
export function handleToolResult(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'tool_result' }>
) {
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

		agentEventBus.emit('agent-activity', {
			conversationId: session.conversationId,
			threadId: session.threadId,
			message: {
				id: getToolMessageId(event.sessionId, event.toolCallId),
				status: event.isError ? 'error' : 'sent',
				agentData: {
					sessionId: event.sessionId,
					toolName: event.tool,
					toolResult: event.result,
					toolError: event.isError,
					toolCallStatus: event.isError ? 'error' : 'success',
					stepNumber: event.stepNumber
				}
			}
		});
	}
}
