/**
 * Agent Store
 * Re-exports from the modular agent store structure.
 *
 * The agent store has been refactored into a modular structure under ./agent/
 * for better maintainability and organization.
 *
 * @module
 */

// Main store and derived stores
export {
	agentStore,
	agentEventBus,
	createAgentEventBus,
	activeSession,
	allSessions,
	pendingActions,
	agentPanelOpen,
	agentLoading,
	agentError
} from './agent/index';

// Type exports for convenience
export type { AgentState } from './agent/state';
export type {
	AgentEventBus,
	AgentEventHandler,
	AgentEventMap,
	AgentEventName,
	SystemMessagePayload
} from './agent/index';
