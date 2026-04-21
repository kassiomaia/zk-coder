/**
 * Tests for agentStore
 * Comprehensive test coverage for all agent store functionality.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import type { AgentSession, PendingAction } from '$lib/types/agent';
// Now import after mocking
import { agentStore } from './agentStore';

// Create mock functions that will be used in vi.mock.
// `vi.hoisted` is required because `vi.mock` is hoisted above imports;
// without it, modules that statically import `@tauri-apps/api/core`
// resolve the mock before `mockInvoke` is initialised.
const mockListen = vi.hoisted(() => vi.fn());
const mockInvoke = vi.hoisted(() => vi.fn());

// Mock Tauri APIs before importing the store
vi.mock('@tauri-apps/api/event', () => ({
	listen: mockListen
}));

vi.mock('@tauri-apps/api/core', () => ({
	invoke: mockInvoke
}));

// Mock crypto.randomUUID
vi.stubGlobal('crypto', {
	randomUUID: vi.fn(() => 'mock-uuid-1234')
});

describe('agentStore', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();

		// Reset store state
		agentStore.setActiveSession(null);
		agentStore.clearError();
		let state = get(agentStore);
		state.sessions.clear();
		state.pendingActions = [];
		state.activeSessionId = null;
		if (state.panelOpen) {
			agentStore.togglePanel();
		}
	});

	afterEach(() => {
		vi.runOnlyPendingTimers();
		vi.useRealTimers();
	});

	describe('Store Initialization & State', () => {
		it('has correct initial state', () => {
			const state = get(agentStore);
			expect(state.sessions).toBeInstanceOf(Map);
			expect(state.sessions.size).toBe(0);
			expect(state.activeSessionId).toBeNull();
			expect(state.pendingActions).toEqual([]);
			expect(state.panelOpen).toBe(false);
			expect(state.loading).toBe(false);
			expect(state.error).toBeNull();
		});

		it('initialize sets up event listener', async () => {
			mockListen.mockResolvedValue(vi.fn());

			await agentStore.initialize();

			expect(mockListen).toHaveBeenCalledWith('agent:event', expect.any(Function));
		});

		it('destroy cleans up event listener', async () => {
			const unlisten = vi.fn();
			mockListen.mockResolvedValue(unlisten);

			await agentStore.initialize();
			agentStore.destroy();

			expect(unlisten).toHaveBeenCalled();
		});

		it('handles Tauri not available gracefully', async () => {
			mockListen.mockRejectedValue(new Error('Tauri not available'));

			// Verifies graceful degradation — initialize must not throw and must
			// leave the store in a consistent state (no error surfaced to the UI,
			// no stale sessions) when Tauri's event API is unavailable. The previous
			// version of this test spied on `console.debug`, but the store logs via
			// `createLogger`, whose debug channel is a no-op in the vitest mock
			// (`$app/environment.dev === false`). Asserting on logging internals
			// was brittle; asserting on observable state is the contract that
			// actually matters.
			await expect(agentStore.initialize()).resolves.toBeUndefined();

			const state = get(agentStore);
			expect(state.error).toBeNull();
			expect(state.sessions.size).toBe(0);
		});
	});

	describe('Session Management', () => {
		const mockSession: AgentSession = {
			id: 'session-1',
			agentId: 'agent-1',
			agentName: 'Test Agent',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			state: { type: 'launching' },
			workspaceRoot: '/workspace',
			createdAt: '2024-01-01T00:00:00Z',
			updatedAt: '2024-01-01T00:00:00Z'
		};

		it('createSession creates a new session successfully', async () => {
			mockInvoke.mockResolvedValue(mockSession);

			const result = await agentStore.createSession('agent-1', 'conv-1', 'thread-1', '/workspace');

			expect(result).toEqual(mockSession);
			expect(mockInvoke).toHaveBeenCalledWith('create_agent_session', {
				agentId: 'agent-1',
				conversationId: 'conv-1',
				threadId: 'thread-1',
				workspaceRoot: '/workspace'
			});

			const state = get(agentStore);
			expect(state.sessions.get('session-1')).toEqual(mockSession);
			expect(state.activeSessionId).toBe('session-1');
			expect(state.panelOpen).toBe(true);
			expect(state.loading).toBe(false);
		});

		it('createSession handles errors', async () => {
			mockInvoke.mockRejectedValue(new Error('Failed to create session'));

			const result = await agentStore.createSession('agent-1', 'conv-1', 'thread-1');

			expect(result).toBeNull();
			const state = get(agentStore);
			expect(state.error).toBe('Error: Failed to create session');
			expect(state.loading).toBe(false);
		});

		it('loadSession rehydrates cached lightweight sessions with persisted runtime details', async () => {
			mockInvoke.mockResolvedValueOnce(mockSession);
			await agentStore.createSession('agent-1', 'conv-1', 'thread-1');

			mockInvoke.mockClear();
			mockInvoke.mockResolvedValueOnce({
				...mockSession,
				filesTouched: [{ path: '/workspace/src/lib.rs', modified: true, action: 'write' }],
				terminalOutput: [],
				tasks: [],
				toolCalls: [
					{
						id: 'tool-1',
						toolName: 'fs_write_file',
						arguments: { path: '/workspace/src/lib.rs' },
						result: { ok: true },
						isError: false,
						durationMs: 42,
						timestamp: '2024-01-01T00:01:00Z'
					}
				],
				executionSteps: [],
				harnessState: {
					mode: 'worker',
					startupVerification: {
						state: 'passed',
						headline: 'Workspace check passed'
					},
					handoff: {
						lastCompletedIncrement: 'Stabilized tool catalog ordering',
						currentObjective: 'Add typed harness state',
						nextRecommendedAction: 'Update store mapping',
						openIssues: [],
						environmentHealth: 'healthy'
					},
					cleanExitRecorded: false
				}
			});

			const result = await agentStore.loadSession('session-1');

			expect(mockInvoke).toHaveBeenCalledWith('get_session_details', {
				sessionId: 'session-1'
			});
			expect(result?.filesTouched).toHaveLength(1);
			expect(result?.toolCalls?.[0]).toMatchObject({
				tool: 'fs_write_file',
				durationMs: 42,
				timestamp: '2024-01-01T00:01:00Z'
			});
			expect(result?.harnessState).toMatchObject({
				mode: 'worker',
				cleanExitRecorded: false
			});
		});

		it('loadSession returns hydrated session from cache without refetching', async () => {
			mockInvoke.mockResolvedValue({
				...mockSession,
				filesTouched: [],
				terminalOutput: [],
				tasks: [],
				toolCalls: [],
				executionSteps: []
			});

			await agentStore.loadSession('session-1');
			mockInvoke.mockClear();

			const result = await agentStore.loadSession('session-1');

			expect(result?.id).toBe('session-1');
			expect(mockInvoke).not.toHaveBeenCalled();
		});

		it('loadSession accepts sessions without harnessState', async () => {
			mockInvoke.mockResolvedValue({
				...mockSession,
				filesTouched: [],
				terminalOutput: [],
				tasks: [],
				toolCalls: [],
				executionSteps: []
			});

			const result = await agentStore.loadSession('session-1');

			expect(result?.harnessState).toBeUndefined();
		});

		it('updates session harnessState when harness_state_updated event is received', async () => {
			const unlisten = vi.fn();
			mockListen.mockResolvedValue(unlisten);
			mockInvoke.mockResolvedValue(0);
			await agentStore.initialize();

			mockInvoke.mockClear();
			mockInvoke.mockResolvedValue({
				...mockSession,
				filesTouched: [],
				terminalOutput: [],
				tasks: [],
				toolCalls: [],
				executionSteps: []
			});
			await agentStore.loadSession('session-1');

			const eventHandler = mockListen.mock.calls[0]?.[1];
			expect(eventHandler).toBeTypeOf('function');

			eventHandler({
				payload: {
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
				}
			});

			const state = get(agentStore);
			expect(state.sessions.get('session-1')?.harnessState).toMatchObject({
				mode: 'resume_verifier',
				startupVerification: {
					state: 'running',
					headline: 'Checking workspace state'
				},
				cleanExitRecorded: false
			});
		});

		it('loadSession fetches from backend if not loaded', async () => {
			const mockSessionDetails = {
				...mockSession,
				id: 'session-2', // Different ID to avoid conflicts
				tasks: [],
				toolCalls: []
			};
			mockInvoke.mockResolvedValue(mockSessionDetails);

			const result = await agentStore.loadSession('session-2');

			expect(mockInvoke).toHaveBeenCalledWith('get_session_details', {
				sessionId: 'session-2'
			});
			expect(result).toEqual(mockSessionDetails);

			const state = get(agentStore);
			expect(state.sessions.get('session-2')).toEqual(mockSessionDetails);
		});

		it('sendRequest calls backend with correct parameters', async () => {
			mockInvoke.mockResolvedValue(undefined);

			await agentStore.sendRequest('session-1', 'Hello, agent!');

			expect(mockInvoke).toHaveBeenCalledWith('send_agent_request', {
				sessionId: 'session-1',
				content: 'Hello, agent!'
			});
		});

		it('sendRequest handles errors', async () => {
			mockInvoke.mockRejectedValue(new Error('Network error'));

			// Verifies the observable effect of a failed IPC: the error message is
			// surfaced to the store so the UI can display it. The previous version
			// of this test also asserted that `console.error` was called with a
			// specific shape, but the store now logs via `createLogger`, which
			// prefixes the message and redacts Error objects into plain structures
			// — making the console-level assertion brittle without adding coverage
			// beyond what the store state check already provides.
			await agentStore.sendRequest('session-1', 'test');

			const state = get(agentStore);
			expect(state.error).toBe('Error: Network error');
		});

		it('cancelSession marks session as cancelled in store', async () => {
			// Setup: add a session
			mockInvoke.mockResolvedValue(mockSession);
			await agentStore.createSession('agent-1', 'conv-1', 'thread-1');

			// Clear the mock for cancel
			mockInvoke.mockClear();
			mockInvoke.mockResolvedValue(undefined);

			await agentStore.cancelSession('session-1');

			expect(mockInvoke).toHaveBeenCalledWith('cancel_agent_session', {
				sessionId: 'session-1'
			});

			const state = get(agentStore);
			expect(state.sessions.get('session-1')?.state).toEqual({
				type: 'cancelled',
				reason: 'User cancelled'
			});
		});

		it('rollbackSession returns reverted files', async () => {
			const revertedFiles = ['/path/to/file1.ts', '/path/to/file2.ts'];
			mockInvoke.mockResolvedValue(revertedFiles);

			const result = await agentStore.rollbackSession('session-1');

			expect(result).toEqual(revertedFiles);
			expect(mockInvoke).toHaveBeenCalledWith('rollback_session', {
				sessionId: 'session-1'
			});
		});

		it('updateSessionWorkspace updates backend and local state', async () => {
			// Setup: add a session
			mockInvoke.mockResolvedValue(mockSession);
			await agentStore.createSession('agent-1', 'conv-1', 'thread-1', '/old-workspace');

			// Clear mocks
			mockInvoke.mockClear();
			mockInvoke.mockResolvedValue(undefined);

			await agentStore.updateSessionWorkspace('session-1', '/new-workspace');

			expect(mockInvoke).toHaveBeenCalledWith('update_agent_session_workspace', {
				sessionId: 'session-1',
				workspaceRoot: '/new-workspace'
			});

			const state = get(agentStore);
			const session = state.sessions.get('session-1');
			expect(session?.workspaceRoot).toBe('/new-workspace');
		});

		it('updateSessionModel updates backend and local state', async () => {
			// Setup: add a session
			mockInvoke.mockResolvedValue(mockSession);
			await agentStore.createSession('agent-1', 'conv-1', 'thread-1', '/workspace');

			// Clear mocks
			mockInvoke.mockClear();
			mockInvoke.mockResolvedValue(undefined);

			await agentStore.updateSessionModel('session-1', 'claude-opus-4');

			expect(mockInvoke).toHaveBeenCalledWith('update_agent_session_model', {
				sessionId: 'session-1',
				modelId: 'claude-opus-4'
			});

			const state = get(agentStore);
			const session = state.sessions.get('session-1');
			expect(session?.modelId).toBe('claude-opus-4');
		});

		it('getSessionHistory returns session history', async () => {
			const history = [mockSession];
			mockInvoke.mockResolvedValue(history);

			const result = await agentStore.getSessionHistory('conv-1');

			expect(result).toEqual(history);
			expect(mockInvoke).toHaveBeenCalledWith('get_conversation_session_history', {
				conversationId: 'conv-1'
			});
		});

		it('getSessionDetails transforms backend tasks and tool calls', async () => {
			const backendTasks = [
				{
					id: 'task-1',
					sessionId: 'session-1',
					request: { content: 'Test task content' },
					status: 'pending' as const,
					steps: [],
					createdAt: '2024-01-01T00:00:00Z'
				}
			];

			const backendToolCalls = [
				{
					id: 'tool-1',
					toolName: 'read_file',
					arguments: JSON.stringify({ path: '/test' }),
					result: JSON.stringify({ content: 'test' }),
					isError: false,
					timestamp: '2024-01-01T00:00:00Z'
				}
			];

			mockInvoke.mockResolvedValue({
				...mockSession,
				tasks: backendTasks,
				toolCalls: backendToolCalls
			});

			const result = await agentStore.getSessionDetails('session-1');

			expect(mockInvoke).toHaveBeenCalledWith('get_session_details', {
				sessionId: 'session-1'
			});

			expect(result?.tasks?.[0]).toMatchObject({
				id: 'task-1',
				title: 'Test task content',
				status: 'pending'
			});

			expect(result?.toolCalls?.[0]).toMatchObject({
				id: 'tool-1',
				tool: 'read_file',
				arguments: { path: '/test' },
				result: { content: 'test' },
				isError: false
			});
		});
	});

	describe('Action Approval', () => {
		const mockAction: PendingAction = {
			id: 'action-1',
			toolName: 'write_file',
			actionType: { type: 'write_file', path: '/test', sizeBytes: 100 },
			description: 'Write to file',
			arguments: {},
			createdAt: '2024-01-01T00:00:00Z'
		};

		it('approveAction calls backend and removes from pending actions', async () => {
			mockInvoke.mockResolvedValue(undefined);

			// Simulate pending action
			let state = get(agentStore);
			state.pendingActions = [mockAction];

			await agentStore.approveAction('session-1', 'action-1');

			expect(mockInvoke).toHaveBeenCalledWith('approve_agent_action', {
				sessionId: 'session-1',
				actionId: 'action-1'
			});

			state = get(agentStore);
			expect(state.pendingActions).toEqual([]);
		});

		it('denyAction calls backend and removes from pending actions', async () => {
			mockInvoke.mockResolvedValue(undefined);

			// Simulate pending action
			let state = get(agentStore);
			state.pendingActions = [mockAction];

			await agentStore.denyAction('session-1', 'action-1');

			expect(mockInvoke).toHaveBeenCalledWith('deny_agent_action', {
				sessionId: 'session-1',
				actionId: 'action-1'
			});

			state = get(agentStore);
			expect(state.pendingActions).toEqual([]);
		});

		it('handles approval action errors', async () => {
			mockInvoke.mockRejectedValue(new Error('Approval failed'));

			await agentStore.approveAction('session-1', 'action-1');

			const state = get(agentStore);
			expect(state.error).toBe('Error: Approval failed');
		});
	});

	describe('Task & Subtask Management', () => {
		it('createSubtask calls backend with correct parameters', async () => {
			const subtaskId = 'subtask-1';
			mockInvoke.mockResolvedValue(subtaskId);

			const result = await agentStore.createSubtask('task-1', 'Test Subtask', 'Description');

			expect(result).toBe(subtaskId);
			expect(mockInvoke).toHaveBeenCalledWith('create_subtask', {
				taskId: 'task-1',
				title: 'Test Subtask',
				description: 'Description'
			});
		});

		it('updateSubtaskStatus calls backend with JSON status', async () => {
			mockInvoke.mockResolvedValue(undefined);

			await agentStore.updateSubtaskStatus('subtask-1', 'completed');

			expect(mockInvoke).toHaveBeenCalledWith('update_subtask_status', {
				subtaskId: 'subtask-1',
				status: JSON.stringify({ type: 'completed' })
			});
		});
	});

	describe('Agent Management', () => {
		const mockAgents = [
			{
				id: 'agent-1',
				name: 'Code Assistant',
				description: 'Helps with coding',
				systemPrompt: 'You are helpful',
				allowedTools: ['read_file', 'write_file'],
				defaultModelId: 'claude-3',
				trustMode: 'trusted' as const
			}
		];

		it('listAgents returns all agents', async () => {
			mockInvoke.mockResolvedValue(mockAgents);

			const result = await agentStore.listAgents();

			expect(result).toEqual(mockAgents);
			expect(mockInvoke).toHaveBeenCalledWith('list_agents');
		});

		it('getAgent returns specific agent', async () => {
			mockInvoke.mockResolvedValue(mockAgents[0]);

			const result = await agentStore.getAgent('agent-1');

			expect(result).toEqual(mockAgents[0]);
			expect(mockInvoke).toHaveBeenCalledWith('get_agent', { agentId: 'agent-1' });
		});

		it('handles listAgents errors', async () => {
			mockInvoke.mockRejectedValue(new Error('API error'));

			const result = await agentStore.listAgents();

			expect(result).toEqual([]);

			const state = get(agentStore);
			expect(state.error).toBe('Error: API error');
		});
	});

	describe('UI State Management', () => {
		it('togglePanel toggles panelOpen state', () => {
			let state = get(agentStore);
			expect(state.panelOpen).toBe(false);

			agentStore.togglePanel();
			state = get(agentStore);
			expect(state.panelOpen).toBe(true);

			agentStore.togglePanel();
			state = get(agentStore);
			expect(state.panelOpen).toBe(false);
		});

		it('setActiveSession updates activeSessionId', () => {
			agentStore.setActiveSession('session-1');

			const state = get(agentStore);
			expect(state.activeSessionId).toBe('session-1');
		});

		it('setActiveSession can set to null', () => {
			agentStore.setActiveSession('session-1');
			agentStore.setActiveSession(null);

			const state = get(agentStore);
			expect(state.activeSessionId).toBeNull();
		});

		it('clearError clears error state', async () => {
			// Set an error first
			mockInvoke.mockRejectedValue(new Error('Test error'));
			await agentStore.createSession('agent-1', 'conv-1', 'thread-1');

			let state = get(agentStore);
			expect(state.error).toBeTruthy();

			agentStore.clearError();

			state = get(agentStore);
			expect(state.error).toBeNull();
		});
	});

	describe('Agent Event Bus', () => {
		it('no longer exposes the legacy setXxxCallback surface on the agent store', () => {
			expect((agentStore as unknown as Record<string, unknown>).setExecutionCompleteCallback).toBeUndefined();
			expect((agentStore as unknown as Record<string, unknown>).setSystemMessageCallback).toBeUndefined();
			expect((agentStore as unknown as Record<string, unknown>).setAgentActivityCallback).toBeUndefined();
			expect((agentStore as unknown as Record<string, unknown>).setCleanupTransientCallback).toBeUndefined();
			expect((agentStore as unknown as Record<string, unknown>).setPersistAssistantMessageCallback).toBeUndefined();
		});
	});

	describe('Derived Stores', () => {
		const mockSession: AgentSession = {
			id: 'session-1',
			agentId: 'agent-1',
			agentName: 'Test Agent',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			state: { type: 'launching' },
			createdAt: '2024-01-01T00:00:00Z',
			updatedAt: '2024-01-01T00:00:00Z'
		};

		it('activeSession returns the active session or null', async () => {
			const { activeSession } = await import('./agentStore');

			expect(get(activeSession)).toBeNull();

			mockInvoke.mockResolvedValue(mockSession);
			await agentStore.createSession('agent-1', 'conv-1', 'thread-1');

			expect(get(activeSession)).toEqual(mockSession);
		});

		it('allSessions returns sessions as array', async () => {
			const { allSessions } = await import('./agentStore');

			expect(get(allSessions)).toEqual([]);

			mockInvoke.mockResolvedValue(mockSession);
			await agentStore.createSession('agent-1', 'conv-1', 'thread-1');

			const sessions = get(allSessions);
			expect(sessions).toHaveLength(1);
			expect(sessions[0]).toEqual(mockSession);
		});

		it('pendingActions returns pending actions array', async () => {
			const { pendingActions } = await import('./agentStore');

			expect(get(pendingActions)).toEqual([]);

			// Add a pending action to the state
			let state = get(agentStore);
			state.pendingActions = [
				{
					id: 'action-1',
					toolName: 'test',
					actionType: { type: 'read_file', path: '/test' },
					description: 'Test',
					arguments: {},
					createdAt: '2024-01-01T00:00:00Z'
				}
			];

			expect(get(pendingActions)).toHaveLength(1);
		});

		it('agentPanelOpen returns panel open state', async () => {
			const { agentPanelOpen } = await import('./agentStore');

			expect(get(agentPanelOpen)).toBe(false);

			agentStore.togglePanel();

			expect(get(agentPanelOpen)).toBe(true);
		});

		it('agentError returns error state', async () => {
			const { agentError } = await import('./agentStore');

			expect(get(agentError)).toBeNull();

			// Trigger an error
			mockInvoke.mockRejectedValue(new Error('Test error'));
			await agentStore.createSession('agent-1', 'conv-1', 'thread-1');

			expect(get(agentError)).toBe('Error: Test error');
		});
	});

	describe('Error Handling', () => {
		it('handles createSession errors gracefully', async () => {
			mockInvoke.mockRejectedValue(new Error('Connection failed'));

			const result = await agentStore.createSession('agent-1', 'conv-1', 'thread-1');

			expect(result).toBeNull();

			const state = get(agentStore);
			expect(state.error).toBe('Error: Connection failed');
			expect(state.loading).toBe(false);
		});
	});
});
