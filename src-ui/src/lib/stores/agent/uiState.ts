/**
 * UI State
 * UI state management functions for the agent store.
 */

import type { AgentState } from './state';

/**
 * Toggle the agent panel visibility.
 */
export function togglePanel(update: (fn: (state: AgentState) => AgentState) => void) {
	update((s) => ({ ...s, panelOpen: !s.panelOpen }));
}

/**
 * Set the agent panel visibility directly.
 */
export function setPanel(
	update: (fn: (state: AgentState) => AgentState) => void,
	open: boolean
) {
	update((s) => (s.panelOpen === open ? s : { ...s, panelOpen: open }));
}

/**
 * Set the active session.
 */
export function setActiveSession(
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string | null
) {
	update((s) => ({ ...s, activeSessionId: sessionId }));
}

/**
 * Clear agent error.
 */
export function clearError(update: (fn: (state: AgentState) => AgentState) => void) {
	update((s) => ({ ...s, error: null }));
}
