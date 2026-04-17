import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createInitialState } from '../state';
import type { AgentSession } from '$lib/types/agent';
import type { AgentState } from '../state';
import {
	handleHarnessStateUpdated,
	handleRecoveryContinue,
	handleRecoveryReplan,
	handleSessionPaused
} from './harnessEvents';

const callbackMocks = vi.hoisted(() => ({
	getOnAgentActivity: vi.fn()
}));

vi.mock('../callbacks', () => ({
	getOnAgentActivity: callbackMocks.getOnAgentActivity
}));

function makeSession(overrides: Partial<AgentSession> = {}): AgentSession {
	return {
		id: 'session-1',
		agentId: 'code-agent',
		agentName: 'Code Agent',
		conversationId: 'conv-1',
		threadId: 'thread-1',
		state: { type: 'running', currentStep: 1, contextUsage: 0 },
		createdAt: '2024-01-01T12:00:00Z',
		updatedAt: '2024-01-01T12:00:00Z',
		...overrides
	};
}

function makeState(session: AgentSession = makeSession()): AgentState {
	const state = createInitialState();
	state.sessions.set(session.id, session);
	return state;
}

describe('harnessEvents', () => {
	beforeEach(() => {
		callbackMocks.getOnAgentActivity.mockReset();
		callbackMocks.getOnAgentActivity.mockReturnValue(null);
	});

	describe('handleHarnessStateUpdated (startup verification & handoff)', () => {
		it('stores startup verification running state on the session without touching chat', () => {
			const state = makeState();
			const onAgentActivity = vi.fn();
			callbackMocks.getOnAgentActivity.mockReturnValue(onAgentActivity);

			handleHarnessStateUpdated(state, {
				type: 'harness_state_updated',
				sessionId: 'session-1',
				harnessState: {
					mode: 'resume_verifier',
					startupVerification: {
						state: 'running',
						headline: 'Checking workspace state'
					},
					cleanExitRecorded: false
				}
			});

			expect(state.sessions.get('session-1')?.harnessState).toMatchObject({
				mode: 'resume_verifier',
				startupVerification: {
					state: 'running',
					headline: 'Checking workspace state'
				}
			});
			expect(onAgentActivity).not.toHaveBeenCalled();
		});

		it('replaces prior harness state when a new handoff summary arrives', () => {
			const state = makeState(
				makeSession({
					harnessState: {
						mode: 'worker',
						startupVerification: {
							state: 'passed',
							headline: 'Workspace verified'
						},
						cleanExitRecorded: false
					}
				})
			);

			handleHarnessStateUpdated(state, {
				type: 'harness_state_updated',
				sessionId: 'session-1',
				harnessState: {
					mode: 'worker',
					startupVerification: {
						state: 'passed',
						headline: 'Workspace verified'
					},
					handoff: {
						lastCompletedIncrement: 'Extracted harness event helpers',
						currentObjective: 'Validate Slice 10 workflow',
						nextRecommendedAction: 'Record before/after measurement',
						openIssues: [],
						environmentHealth: 'healthy'
					},
					cleanExitRecorded: false
				}
			});

			const stored = state.sessions.get('session-1')?.harnessState;
			expect(stored?.handoff?.currentObjective).toBe('Validate Slice 10 workflow');
			expect(stored?.handoff?.nextRecommendedAction).toBe('Record before/after measurement');
		});

		it('marks cleanExitRecorded on finished handoff without mutating session state.type', () => {
			const state = makeState(
				makeSession({
					state: { type: 'finished', completedAt: '2024-01-01T13:00:00Z' }
				})
			);

			handleHarnessStateUpdated(state, {
				type: 'harness_state_updated',
				sessionId: 'session-1',
				harnessState: {
					mode: 'worker',
					cleanExitRecorded: true
				}
			});

			const session = state.sessions.get('session-1');
			expect(session?.harnessState?.cleanExitRecorded).toBe(true);
			expect(session?.state.type).toBe('finished');
		});

		it('records startup verification failure headline for the panel to render', () => {
			const state = makeState();

			handleHarnessStateUpdated(state, {
				type: 'harness_state_updated',
				sessionId: 'session-1',
				harnessState: {
					mode: 'resume_verifier',
					startupVerification: {
						state: 'failed',
						headline: 'Workspace root missing',
						details: ['path ~/project no longer exists']
					},
					cleanExitRecorded: false
				}
			});

			const verification = state.sessions.get('session-1')?.harnessState?.startupVerification;
			expect(verification?.state).toBe('failed');
			expect(verification?.headline).toBe('Workspace root missing');
			expect(verification?.details).toEqual(['path ~/project no longer exists']);
		});

		it('is a no-op when the session does not exist', () => {
			const state = createInitialState();

			handleHarnessStateUpdated(state, {
				type: 'harness_state_updated',
				sessionId: 'unknown',
				harnessState: {
					mode: 'worker',
					cleanExitRecorded: false
				}
			});

			expect(state.sessions.size).toBe(0);
		});
	});

	describe('handleSessionPaused (recovery)', () => {
		it('transitions the session to paused and injects an unresolved recovery card', () => {
			const state = makeState();
			const onAgentActivity = vi.fn();
			callbackMocks.getOnAgentActivity.mockReturnValue(onAgentActivity);

			handleSessionPaused(state, {
				type: 'session_paused',
				sessionId: 'session-1',
				message: 'Interrupted while updating status handling',
				pausedAt: '2024-01-01T12:05:00Z',
				currentSubtaskId: 'subtask-9'
			});

			const session = state.sessions.get('session-1');
			expect(session?.state).toEqual({
				type: 'paused',
				pausedAt: '2024-01-01T12:05:00Z',
				message: 'Interrupted while updating status handling',
				currentSubtaskId: 'subtask-9'
			});

			expect(onAgentActivity).toHaveBeenCalledWith('conv-1', 'thread-1', {
				id: 'recovery-session-1',
				conversationId: 'conv-1',
				threadId: 'thread-1',
				role: 'assistant',
				kind: 'recovery',
				content: 'Interrupted while updating status handling',
				timestamp: new Date('2024-01-01T12:05:00Z'),
				agentData: {
					sessionId: 'session-1',
					recoveryResolved: false
				}
			});
		});

		it('preserves prior harnessState so the panel summary survives the pause transition', () => {
			const state = makeState(
				makeSession({
					harnessState: {
						mode: 'worker',
						startupVerification: {
							state: 'passed',
							headline: 'Workspace verified'
						},
						handoff: {
							lastCompletedIncrement: 'Panel-backed recovery',
							currentObjective: 'Land Slice 10',
							nextRecommendedAction: 'Resume FWA.4',
							openIssues: [],
							environmentHealth: 'healthy'
						},
						cleanExitRecorded: false
					}
				})
			);

			handleSessionPaused(state, {
				type: 'session_paused',
				sessionId: 'session-1',
				message: 'Paused before next increment',
				pausedAt: '2024-01-01T12:07:00Z'
			});

			const session = state.sessions.get('session-1');
			expect(session?.state.type).toBe('paused');
			expect(session?.harnessState?.handoff?.currentObjective).toBe('Land Slice 10');
		});

		it('updates session state even when the chat activity callback is unset', () => {
			const state = makeState();
			callbackMocks.getOnAgentActivity.mockReturnValue(null);

			handleSessionPaused(state, {
				type: 'session_paused',
				sessionId: 'session-1',
				message: 'Paused',
				pausedAt: '2024-01-01T12:06:00Z'
			});

			expect(state.sessions.get('session-1')?.state.type).toBe('paused');
		});
	});

	describe('handleRecoveryContinue / handleRecoveryReplan', () => {
		it('marks the recovery card resolved with "continued" decision', () => {
			const state = makeState();
			const onAgentActivity = vi.fn();
			callbackMocks.getOnAgentActivity.mockReturnValue(onAgentActivity);

			handleRecoveryContinue(state, {
				type: 'recovery_continue',
				sessionId: 'session-1'
			});

			expect(onAgentActivity).toHaveBeenCalledWith('conv-1', 'thread-1', {
				id: 'recovery-session-1',
				agentData: {
					sessionId: 'session-1',
					recoveryResolved: true,
					recoveryDecision: 'continued'
				}
			});
		});

		it('marks the recovery card resolved with "replanned" decision', () => {
			const state = makeState();
			const onAgentActivity = vi.fn();
			callbackMocks.getOnAgentActivity.mockReturnValue(onAgentActivity);

			handleRecoveryReplan(state, {
				type: 'recovery_replan',
				sessionId: 'session-1'
			});

			expect(onAgentActivity).toHaveBeenCalledWith('conv-1', 'thread-1', {
				id: 'recovery-session-1',
				agentData: {
					sessionId: 'session-1',
					recoveryResolved: true,
					recoveryDecision: 'replanned'
				}
			});
		});

		it('does not fire chat activity when the session is unknown', () => {
			const state = createInitialState();
			const onAgentActivity = vi.fn();
			callbackMocks.getOnAgentActivity.mockReturnValue(onAgentActivity);

			handleRecoveryContinue(state, { type: 'recovery_continue', sessionId: 'missing' });
			handleRecoveryReplan(state, { type: 'recovery_replan', sessionId: 'missing' });

			expect(onAgentActivity).not.toHaveBeenCalled();
		});
	});
});
