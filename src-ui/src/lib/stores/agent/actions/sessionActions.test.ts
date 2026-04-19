import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createInitialState } from '../state';
import type { AgentSession } from '$lib/types/agent';
import { loadWorkspaceFiles } from './sessionActions';

const hoistedInvoke = vi.hoisted(() => vi.fn());

vi.mock('@tauri-apps/api/core', () => ({
	invoke: hoistedInvoke
}));

function makeSession(overrides: Partial<AgentSession> = {}): AgentSession {
	return {
		id: 'session-1',
		agentId: 'code-agent',
		agentName: 'Code Agent',
		conversationId: 'conv-1',
		threadId: 'thread-1',
		state: { type: 'running', currentStep: 1, contextUsage: 0 },
		createdAt: new Date().toISOString(),
		updatedAt: new Date().toISOString(),
		...overrides
	};
}

describe('loadWorkspaceFiles', () => {
	beforeEach(() => {
		hoistedInvoke.mockReset();
	});

	it('invokes list_workspace_files with the workspace root', async () => {
		const state = createInitialState();
		state.sessions.set('session-1', makeSession());
		const update = vi.fn((fn: (s: typeof state) => typeof state) => {
			fn(state);
		});
		hoistedInvoke.mockResolvedValue({ entries: [], truncated: false });

		await loadWorkspaceFiles(state, update, 'session-1', '/tmp/ws');

		expect(hoistedInvoke).toHaveBeenCalledWith('list_workspace_files', { root: '/tmp/ws' });
	});

	it('writes entries and truncated flag onto the session', async () => {
		const state = createInitialState();
		state.sessions.set('session-1', makeSession());
		const update = vi.fn((fn: (s: typeof state) => typeof state) => {
			fn(state);
		});
		const entries = [
			{ path: 'a.ts', kind: 'file' as const },
			{ path: 'src', kind: 'directory' as const }
		];
		hoistedInvoke.mockResolvedValue({ entries, truncated: true });

		await loadWorkspaceFiles(state, update, 'session-1', '/tmp/ws');

		const session = state.sessions.get('session-1')!;
		expect(session.workspaceFiles).toEqual(entries);
		expect(session.workspaceFilesTruncated).toBe(true);
	});

	it('does not clobber existing session fields unrelated to workspace listing', async () => {
		const state = createInitialState();
		state.sessions.set(
			'session-1',
			makeSession({
				filesTouched: [{ path: 'a.ts', modified: true, action: 'write' }],
				tasks: [{ id: 't1', title: 'x', status: 'pending', subtasks: [] }]
			})
		);
		const update = vi.fn((fn: (s: typeof state) => typeof state) => {
			fn(state);
		});
		hoistedInvoke.mockResolvedValue({ entries: [], truncated: false });

		await loadWorkspaceFiles(state, update, 'session-1', '/tmp/ws');

		const session = state.sessions.get('session-1')!;
		expect(session.filesTouched).toHaveLength(1);
		expect(session.tasks).toHaveLength(1);
	});

	it('is a no-op on unknown session', async () => {
		const state = createInitialState();
		const update = vi.fn((fn: (s: typeof state) => typeof state) => {
			fn(state);
		});
		hoistedInvoke.mockResolvedValue({ entries: [], truncated: false });

		await loadWorkspaceFiles(state, update, 'missing', '/tmp/ws');

		expect(state.sessions.size).toBe(0);
	});

	it('swallows errors without mutating the session', async () => {
		const state = createInitialState();
		state.sessions.set('session-1', makeSession({ workspaceFiles: undefined }));
		const update = vi.fn((fn: (s: typeof state) => typeof state) => {
			fn(state);
		});
		hoistedInvoke.mockRejectedValue(new Error('boom'));

		await expect(loadWorkspaceFiles(state, update, 'session-1', '/tmp/ws')).resolves.toBeUndefined();
		expect(state.sessions.get('session-1')!.workspaceFiles).toBeUndefined();
	});
});
