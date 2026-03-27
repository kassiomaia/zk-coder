import {describe, expect, it} from 'vitest';
import type {AgentSession} from '$lib/types/agent';
import type {Thread} from '$lib/types/thread';
import {getThreadAgentState} from './threadAgentState';

describe('getThreadAgentState', () => {
	const thread: Thread = {
		id: 'thread-1',
		conversationId: 'conv-1',
		parentId: null,
		title: 'Agent Thread',
		createdAt: new Date('2024-01-01T12:00:00Z'),
		updatedAt: new Date('2024-01-01T12:00:00Z'),
		messageCount: 1,
		metadata: {
			resolved: false,
			agentSessionId: 'session-1'
		}
	};

	const session: AgentSession = {
		id: 'session-1',
		agentId: 'code-agent',
		agentName: 'Code Agent',
		conversationId: 'conv-1',
		threadId: 'thread-1',
		state: {
			type: 'running',
			currentStep: 2,
			contextUsage: 22
		},
		executionSteps: [
			{
				stepNumber: 2,
				action: {
					type: 'tool_call',
					tool: 'read_file',
					arguments: { path: '/tmp/file.ts' }
				},
				timestamp: '2024-01-01T12:01:00Z'
			}
		],
		statusPhrases: ['Reading files...'],
		createdAt: '2024-01-01T12:00:00Z',
		updatedAt: '2024-01-01T12:01:00Z'
	};

	it('returns idle state when the thread has no linked agent session', () => {
		const state = getThreadAgentState(
			{
				...thread,
				metadata: { resolved: false }
			},
			[]
		);

		expect(state.session).toBeNull();
		expect(state.status).toBeNull();
		expect(state.isWorking).toBe(false);
		expect(state.sessionId).toBeNull();
	});

	it('returns a working state for the linked running session', () => {
		const state = getThreadAgentState(thread, [session]);

		expect(state.session?.id).toBe('session-1');
		expect(state.sessionId).toBe('session-1');
		expect(state.isWorking).toBe(true);
		expect(state.status?.active).toBe(true);
	});

	it('treats user input required as an active waiting state', () => {
		const waitingState = getThreadAgentState(thread, [
			{
				...session,
				state: {
					type: 'user_input_required',
					pendingActionIds: ['approval-1']
				}
			}
		]);

		expect(waitingState.isWorking).toBe(true);
		expect(waitingState.status?.text).toMatch(/waiting/i);
	});
});
