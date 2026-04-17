import { describe, expect, it } from 'vitest';
import type { AgentSession } from '$lib/types/agent';
import type { Thread } from '$lib/types/thread';
import { getThreadAgentState } from './threadAgentState';

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

	it('ignores stale execution history when the session is idle', () => {
		const idleState = getThreadAgentState(thread, [
			{
				...session,
				state: {
					type: 'wait_for_request',
					lastActionAt: '2024-01-01T12:02:00Z'
				}
			}
		]);

		expect(idleState.isWorking).toBe(false);
		expect(idleState.status?.text).toBe('Ready');
	});

	it('surfaces resume verifier state for paused sessions with harness metadata', () => {
		const pausedState = getThreadAgentState(thread, [
			{
				...session,
				state: {
					type: 'paused',
					pausedAt: '2024-01-01T12:02:00Z',
					message: 'Interrupted while updating status handling'
				},
				harnessState: {
					mode: 'resume_verifier',
					startupVerification: {
						state: 'idle',
						headline: 'Workspace should be verified before resuming'
					},
					cleanExitRecorded: false
				}
			}
		]);

		expect(pausedState.isWorking).toBe(false);
		expect(pausedState.status?.text).toBe('Workspace should be verified before resuming');
	});

	it('surfaces startup verification progress before generic running text', () => {
		const verifyingState = getThreadAgentState(thread, [
			{
				...session,
				harnessState: {
					mode: 'resume_verifier',
					startupVerification: {
						state: 'running',
						headline: 'Checking workspace state'
					},
					cleanExitRecorded: false
				}
			}
		]);

		expect(verifyingState.isWorking).toBe(true);
		expect(verifyingState.status?.text).toBe('Checking workspace state');
	});

	it('shows resuming-from-handoff while a resume_verifier session is running', () => {
		const resumingState = getThreadAgentState(thread, [
			{
				...session,
				state: {
					type: 'running',
					currentStep: 1,
					contextUsage: 0.2
				},
				harnessState: {
					mode: 'resume_verifier',
					startupVerification: {
						state: 'passed',
						headline: 'Workspace verified'
					},
					cleanExitRecorded: false
				}
			}
		]);

		expect(resumingState.isWorking).toBe(true);
		expect(resumingState.status?.text).toBe('Workspace verified');
		expect(resumingState.status?.icon).toBe('scan');
	});

	it('shows resume-blocked when paused with failed verification', () => {
		const blockedState = getThreadAgentState(thread, [
			{
				...session,
				state: {
					type: 'paused',
					pausedAt: '2024-01-01T12:02:00Z',
					message: 'Interrupted'
				},
				harnessState: {
					mode: 'resume_verifier',
					startupVerification: {
						state: 'failed',
						headline: 'Missing workspace root'
					},
					cleanExitRecorded: false
				}
			}
		]);

		expect(blockedState.isWorking).toBe(false);
		expect(blockedState.status?.text).toBe('Resume blocked: Missing workspace root');
	});

	it('surfaces environment-blocked when paused with blocked environment health', () => {
		const envBlocked = getThreadAgentState(thread, [
			{
				...session,
				state: {
					type: 'paused',
					pausedAt: '2024-01-01T12:02:00Z',
					message: 'Interrupted'
				},
				harnessState: {
					mode: 'resume_verifier',
					startupVerification: {
						state: 'passed',
						headline: 'Workspace verified'
					},
					handoff: {
						lastCompletedIncrement: 'Stabilized tool catalog ordering',
						currentObjective: 'Add typed harness state',
						nextRecommendedAction: 'Investigate external dependency',
						openIssues: ['External service unreachable'],
						environmentHealth: 'blocked'
					},
					cleanExitRecorded: false
				}
			}
		]);

		expect(envBlocked.isWorking).toBe(false);
		expect(envBlocked.status?.text).toMatch(/environment blocked/i);
	});

	it('shows clean-handoff-saved for finished sessions with clean exit recorded', () => {
		const handedOff = getThreadAgentState(thread, [
			{
				...session,
				state: {
					type: 'finished',
					completedAt: '2024-01-01T12:10:00Z'
				},
				harnessState: {
					mode: 'worker',
					cleanExitRecorded: true
				}
			}
		]);

		expect(handedOff.isWorking).toBe(false);
		expect(handedOff.status?.text).toBe('Clean handoff saved');
		expect(handedOff.status?.icon).toBe('check');
	});
});
