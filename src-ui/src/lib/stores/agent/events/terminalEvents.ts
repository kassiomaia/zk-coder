/**
 * Terminal Events
 * Handles terminal output events.
 */

import type { AgentEvent } from '$lib/types/agent';
import type { AgentState } from '../state';

/**
 * Handle command_output event.
 */
export function handleCommandOutput(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'command_output' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		const line: import('$lib/types/agent').TerminalLine = {
			id: crypto.randomUUID(),
			type: event.stream,
			content: event.content,
			timestamp: new Date().toISOString()
		};
		state.sessions.set(event.sessionId, {
			...session,
			terminalOutput: [...(session.terminalOutput ?? []), line]
		});
	}
}

/**
 * Handle command_finished event.
 */
export function handleCommandFinished(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'command_finished' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		const line: import('$lib/types/agent').TerminalLine = {
			id: crypto.randomUUID(),
			type: 'info',
			content: `Process exited with code ${event.exitCode ?? 'unknown'}`,
			timestamp: new Date().toISOString()
		};
		state.sessions.set(event.sessionId, {
			...session,
			terminalOutput: [...(session.terminalOutput ?? []), line]
		});
	}
}
