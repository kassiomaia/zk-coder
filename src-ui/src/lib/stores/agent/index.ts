/**
 * Agent Store Module Index
 * Public API exports for backward compatibility.
 */

// Main store export
export { agentStore } from './store';
export type { AgentStore, AgentStoreInstance } from './store';

// State types
export type { AgentState } from './state';

// Derived stores
import { derived } from 'svelte/store';
import { agentStore } from './store';

/** Currently active agent session */
export const activeSession = derived(agentStore, ($state) =>
	$state.activeSessionId ? ($state.sessions.get($state.activeSessionId) ?? null) : null
);

/** All sessions as an array */
export const allSessions = derived(agentStore, ($state) => Array.from($state.sessions.values()));

/** Pending actions for the active session */
export const pendingActions = derived(agentStore, ($state) => $state.pendingActions);

/** Whether the agent panel is open */
export const agentPanelOpen = derived(agentStore, ($state) => $state.panelOpen);

/** Whether the agent store is loading */
export const agentLoading = derived(agentStore, ($state) => $state.loading);

/** Agent store error */
export const agentError = derived(agentStore, ($state) => $state.error);
