/**
 * Derived Stores
 * Derived Svelte stores for the agent store.
 */

import { derived, readable } from 'svelte/store';
import type { Readable } from 'svelte/store';
import type { AgentStore } from './store';
import type { AgentSession, PendingAction } from '$lib/types/agent';

/**
 * Create derived stores from the agent store.
 */
export function createDerivedStores(agentStore: AgentStore) {
	/** Currently active agent session */
	const activeSession = createDerived(agentStore, ($state) =>
		$state.activeSessionId ? ($state.sessions.get($state.activeSessionId) ?? null) : null
	);

	/** All sessions as an array */
	const allSessions = createDerived(agentStore, ($state) => Array.from($state.sessions.values()));

	/** Pending actions for the active session */
	const pendingActions = createDerived(agentStore, ($state) => $state.pendingActions);

	/** Whether the agent panel is open */
	const agentPanelOpen = createDerived(agentStore, ($state) => $state.panelOpen);

	/** Whether the agent store is loading */
	const agentLoading = createDerived(agentStore, ($state) => $state.loading);

	/** Agent store error */
	const agentError = createDerived(agentStore, ($state) => $state.error);

	return {
		activeSession,
		allSessions,
		pendingActions,
		agentPanelOpen,
		agentLoading,
		agentError
	};
}

/**
 * Helper to create a readable store that derives from a writable store's state.
 */
function createDerived<T, U>(
	store: Readable<T>,
	deriveFn: (state: T) => U
): Readable<U> {
	return derived(store, deriveFn);
}
