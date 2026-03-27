/**
 * Derived Stores
 * Derived Svelte stores for the agent store.
 */

import type {Readable} from 'svelte/store';
import {derived} from 'svelte/store';
import type {AgentStore} from './store';
import type {CoreAgentState} from './state';

/**
 * Create derived stores from the agent store.
 * The agentStore parameter must have a subscribe method to be readable.
 */
export function createDerivedStores(agentStore: AgentStore & Readable<CoreAgentState>) {
	/** Currently active agent session */
	const activeSession = derived(agentStore, ($state) =>
		$state.activeSessionId ? ($state.sessions.get($state.activeSessionId) ?? null) : null
	);

	/** All sessions as an array */
	const allSessions = derived(agentStore, ($state) => Array.from($state.sessions.values()));

	/** Pending actions for the active session */
	const pendingActions = derived(agentStore, ($state) => $state.pendingActions);

	/** Whether the agent panel is open */
	const agentPanelOpen = derived(agentStore, ($state) => $state.panelOpen);

	/** Whether the agent store is loading */
	const agentLoading = derived(agentStore, ($state) => $state.loading);

	/** Agent store error */
	const agentError = derived(agentStore, ($state) => $state.error);

	return {
		activeSession,
		allSessions,
		pendingActions,
		agentPanelOpen,
		agentLoading,
		agentError
	};
}
