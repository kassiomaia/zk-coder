/**
 * Approval Actions
 * Action approval functions for the agent store.
 */

import type { AgentState } from '../state';

/**
 * Approve a pending action.
 */
export async function approveAction(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string,
	actionId: string
): Promise<void> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		await invoke('approve_agent_action', { sessionId, actionId });
		update((state) => ({
			...state,
			pendingActions: state.pendingActions.filter((a) => a.id !== actionId)
		}));
	} catch (error) {
		update((s) => ({ ...s, error: String(error) }));
	}
}

/**
 * Deny a pending action.
 */
export async function denyAction(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string,
	actionId: string
): Promise<void> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		await invoke('deny_agent_action', { sessionId, actionId });
		update((state) => ({
			...state,
			pendingActions: state.pendingActions.filter((a) => a.id !== actionId)
		}));
	} catch (error) {
		update((s) => ({ ...s, error: String(error) }));
	}
}
