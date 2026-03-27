import {beforeEach, describe, expect, it, vi} from 'vitest';
import {createInitialState} from '../state';
import type {AgentSession} from '$lib/types/agent';
import {handleToolCall, handleToolResult} from './toolEvents';

const callbackMocks = vi.hoisted(() => ({
	getOnAgentActivity: vi.fn()
}));

vi.mock('../callbacks', () => ({
	getOnAgentActivity: callbackMocks.getOnAgentActivity
}));

function makeSession(): AgentSession {
	return {
		id: 'session-1',
		agentId: 'code-agent',
		agentName: 'Code Agent',
		conversationId: 'conv-1',
		threadId: 'thread-1',
		state: {type: 'running', currentStep: 1, contextUsage: 0},
		createdAt: new Date().toISOString(),
		updatedAt: new Date().toISOString()
	};
}

describe('toolEvents', () => {
	beforeEach(() => {
		callbackMocks.getOnAgentActivity.mockReset();
	});

	it('creates an assistant-visible tool activity message for tool_call events', () => {
		const state = createInitialState();
		state.sessions.set('session-1', makeSession());
		const onAgentActivity = vi.fn();
		callbackMocks.getOnAgentActivity.mockReturnValue(onAgentActivity);

		handleToolCall(state, {
			type: 'tool_call',
			sessionId: 'session-1',
			toolCallId: 'call-1',
			tool: 'fs_read_file',
			arguments: {path: 'src/main.ts'},
			content: 'Let me read `src/main.ts` and inspect its contents.'
		});

		expect(onAgentActivity).toHaveBeenCalledWith('conv-1', 'thread-1', {
			id: 'tool-call-session-1-call-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			role: 'assistant',
			kind: 'tool_call',
			content: '',
			timestamp: expect.any(Date),
			status: 'sending',
			agentData: {
				sessionId: 'session-1',
				toolName: 'fs_read_file',
				toolArgs: {path: 'src/main.ts'},
				toolCallStatus: 'pending'
			}
		});
	});

	it('updates the same assistant-visible tool activity message on tool_result', () => {
		const state = createInitialState();
		state.sessions.set('session-1', makeSession());
		const onAgentActivity = vi.fn();
		callbackMocks.getOnAgentActivity.mockReturnValue(onAgentActivity);

		handleToolResult(state, {
			type: 'tool_result',
			sessionId: 'session-1',
			toolCallId: 'call-1',
			tool: 'fs_list_files',
			result: {path: 'src', count: 4},
			isError: false,
			content: 'I inspected `src` and found 4 item(s).'
		});

		expect(onAgentActivity).toHaveBeenCalledWith('conv-1', 'thread-1', {
			id: 'tool-call-session-1-call-1',
			status: 'sent',
			agentData: {
				sessionId: 'session-1',
				toolName: 'fs_list_files',
				toolResult: {path: 'src', count: 4},
				toolError: false,
				toolCallStatus: 'success'
			}
		});
	});
});
