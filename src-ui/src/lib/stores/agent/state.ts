/**
 * Agent State
 * Core state interface and initial state factory for the agent store.
 */

import type { AgentSession } from '$lib/types/agent';
import type { PendingAction } from '$lib/types/agent';

/**
 * Core state for the agent store.
 */
export interface AgentState {
	sessions: Map<string, AgentSession>;
	activeSessionId: string | null;
	pendingActions: PendingAction[];
	panelOpen: boolean;
	loading: boolean;
	error: string | null;
}

/**
 * Alias for AgentState for use in other modules.
 */
export type CoreAgentState = AgentState;

/**
 * Create the initial state for a new agent store.
 */
export function createInitialState(): AgentState {
	return {
		sessions: new Map(),
		activeSessionId: null,
		pendingActions: [],
		panelOpen: false,
		loading: false,
		error: null
	};
}
