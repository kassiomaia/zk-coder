import { describe, expect, it } from 'vitest';
import { createInitialState } from '../state';
import type { AgentSession } from '$lib/types/agent';
import { handleFileTouched } from './fileEvents';

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

describe('fileEvents.handleFileTouched', () => {
	it('appends a new file entry when the path is not yet tracked', () => {
		const state = createInitialState();
		state.sessions.set('session-1', makeSession());

		handleFileTouched(state, {
			type: 'file_touched',
			sessionId: 'session-1',
			filePath: 'src/a.ts',
			action: 'write',
			diff: '--- a\n+++ b\n@@ @@\n+x'
		});

		const touched = state.sessions.get('session-1')!.filesTouched!;
		expect(touched).toHaveLength(1);
		expect(touched[0]).toEqual({
			path: 'src/a.ts',
			modified: true,
			action: 'write',
			diff: '--- a\n+++ b\n@@ @@\n+x'
		});
	});

	it('marks write actions as modified = true', () => {
		const state = createInitialState();
		state.sessions.set('session-1', makeSession());

		handleFileTouched(state, {
			type: 'file_touched',
			sessionId: 'session-1',
			filePath: 'src/x.ts',
			action: 'write'
		});

		expect(state.sessions.get('session-1')!.filesTouched![0].modified).toBe(true);
	});

	it('marks delete actions as modified = true', () => {
		const state = createInitialState();
		state.sessions.set('session-1', makeSession());

		handleFileTouched(state, {
			type: 'file_touched',
			sessionId: 'session-1',
			filePath: 'src/x.ts',
			action: 'delete'
		});

		expect(state.sessions.get('session-1')!.filesTouched![0].modified).toBe(true);
	});

	it('marks read actions as modified = false', () => {
		const state = createInitialState();
		state.sessions.set('session-1', makeSession());

		handleFileTouched(state, {
			type: 'file_touched',
			sessionId: 'session-1',
			filePath: 'src/x.ts',
			action: 'read'
		});

		expect(state.sessions.get('session-1')!.filesTouched![0].modified).toBe(false);
	});

	it('updates an existing entry in place instead of duplicating it', () => {
		const state = createInitialState();
		state.sessions.set(
			'session-1',
			makeSession({
				filesTouched: [
					{ path: 'src/a.ts', modified: false, action: 'read' }
				]
			})
		);

		handleFileTouched(state, {
			type: 'file_touched',
			sessionId: 'session-1',
			filePath: 'src/a.ts',
			action: 'write',
			diff: '+y'
		});

		const touched = state.sessions.get('session-1')!.filesTouched!;
		expect(touched).toHaveLength(1);
		expect(touched[0].action).toBe('write');
		expect(touched[0].modified).toBe(true);
		expect(touched[0].diff).toBe('+y');
	});

	it('preserves the prior diff when an update arrives without one', () => {
		const state = createInitialState();
		state.sessions.set(
			'session-1',
			makeSession({
				filesTouched: [
					{ path: 'src/a.ts', modified: true, action: 'write', diff: 'ORIGINAL' }
				]
			})
		);

		handleFileTouched(state, {
			type: 'file_touched',
			sessionId: 'session-1',
			filePath: 'src/a.ts',
			action: 'read'
		});

		expect(state.sessions.get('session-1')!.filesTouched![0].diff).toBe('ORIGINAL');
	});

	it('is a no-op when the session is unknown', () => {
		const state = createInitialState();

		handleFileTouched(state, {
			type: 'file_touched',
			sessionId: 'missing',
			filePath: 'src/a.ts',
			action: 'write'
		});

		expect(state.sessions.size).toBe(0);
	});
});
