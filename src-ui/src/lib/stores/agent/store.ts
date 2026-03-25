/**
 * Agent Store
 * Main store module that ties together all agent functionality.
 */

import { writable, get } from 'svelte/store';
import type { AgentEvent } from '$lib/types/agent';
import type { AgentState as CoreAgentState } from './state';
import { createInitialState } from './state';
import * as Actions from './actions';
import * as Events from './events';
import * as Streaming from './streaming';
import * as Callbacks from './callbacks';
import * as UiState from './uiState';
import { createDerivedStores } from './derived';

/**
 * Agent store interface.
 */
export interface AgentStore {
	subscribe: (fn: (value: CoreAgentState) => void) => () => void;
	initialize(): Promise<void>;
	destroy(): void;
	createSession(
		agentId: string,
		conversationId: string,
		threadId: string,
		workspaceRoot?: string
	): Promise<import('$lib/types/agent').AgentSession | null>;
	loadSession(sessionId: string): Promise<import('$lib/types/agent').AgentSession | null>;
	sendRequest(sessionId: string, content: string): Promise<void>;
	approveAction(sessionId: string, actionId: string): Promise<void>;
	denyAction(sessionId: string, actionId: string): Promise<void>;
	cancelSession(sessionId: string): Promise<void>;
	rollbackSession(sessionId: string): Promise<string[]>;
	updateSessionWorkspace(sessionId: string, workspacePath: string): Promise<void>;
	createSubtask(
		taskId: string,
		title: string,
		description?: string,
		order?: number
	): Promise<string | null>;
	updateSubtaskStatus(
		subtaskId: string,
		status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled'
	): Promise<void>;
	getSessionHistory(conversationId: string): Promise<import('$lib/types/agent').AgentSession[]>;
	getSessionDetails(sessionId: string): Promise<import('$lib/types/agent').AgentSession | null>;
	listAgents(): Promise<import('$lib/types/agent').Agent[]>;
	getAgent(agentId: string): Promise<import('$lib/types/agent').Agent | null>;
	togglePanel(): void;
	setActiveSession(sessionId: string | null): void;
	clearError(): void;
	setExecutionCompleteCallback(
		callback: (conversationId: string, threadId: string, content: string) => void
	): void;
	setSystemMessageCallback(callback: (conversationId: string, message: string) => void): void;
	setAgentActivityCallback(
		callback: (conversationId: string, threadId: string, message: Partial<any>) => void
	): void;
	setCleanupTransientCallback(
		callback: (conversationId: string, threadId: string, sessionId: string) => void
	): void;
}

/**
 * Create the agent store.
 */
function createAgentStore(): AgentStore {
	const { subscribe, set, update } = writable<CoreAgentState>(createInitialState());

	let eventUnsubscribe: (() => void) | null = null;

	/**
	 * Initialize the agent store and listen for backend events.
	 */
	async function initialize() {
		try {
			// Clean up existing listener before creating new one to prevent memory leaks
			if (eventUnsubscribe) {
				eventUnsubscribe();
				eventUnsubscribe = null;
			}

			// Dynamic import to avoid SSR issues with Tauri APIs
			const { listen } = await import('@tauri-apps/api/event');
			const unlisten = await listen<AgentEvent>('agent:event', (event) => {
				const state = get({ subscribe });
				const newState = Events.handleAgentEvent(state, event.payload);
				set(newState);
			});
			eventUnsubscribe = unlisten;
		} catch {
			// Tauri not available (running in browser dev mode)
			console.debug('[agentStore] Tauri event listener not available');
		}
	}

	/**
	 * Cleanup event listeners.
	 */
	function destroy() {
		eventUnsubscribe?.();
	}

	/**
	 * Create a new agent session.
	 */
	async function createSession(
		agentId: string,
		conversationId: string,
		threadId: string,
		workspaceRoot?: string
	) {
		const state = get({ subscribe });
		return Actions.createSession(state, update, agentId, conversationId, threadId, workspaceRoot);
	}

	/**
	 * Load a session from the backend and add it to the store.
	 */
	async function loadSession(sessionId: string) {
		const state = get({ subscribe });
		return Actions.loadSession(state, update, sessionId);
	}

	/**
	 * Send a request to an active agent session.
	 */
	async function sendRequest(sessionId: string, content: string) {
		const state = get({ subscribe });
		return Actions.sendRequest(state, update, sessionId, content);
	}

	/**
	 * Approve a pending action.
	 */
	async function approveAction(sessionId: string, actionId: string) {
		const state = get({ subscribe });
		return Actions.approveAction(state, update, sessionId, actionId);
	}

	/**
	 * Deny a pending action.
	 */
	async function denyAction(sessionId: string, actionId: string) {
		const state = get({ subscribe });
		return Actions.denyAction(state, update, sessionId, actionId);
	}

	/**
	 * Cancel an active session.
	 */
	async function cancelSession(sessionId: string) {
		const state = get({ subscribe });
		return Actions.cancelSession(state, update, sessionId);
	}

	/**
	 * Roll back all file changes made during a session.
	 */
	async function rollbackSession(sessionId: string) {
		return Actions.rollbackSession(sessionId);
	}

	/**
	 * Update the workspace root for a session.
	 */
	async function updateSessionWorkspace(sessionId: string, workspacePath: string) {
		const state = get({ subscribe });
		return Actions.updateSessionWorkspace(state, update, sessionId, workspacePath);
	}

	/**
	 * Create a new subtask for task decomposition.
	 */
	async function createSubtask(taskId: string, title: string, description?: string, order?: number) {
		const state = get({ subscribe });
		return Actions.createSubtask(state, update, taskId, title, description, order);
	}

	/**
	 * Update subtask status.
	 */
	async function updateSubtaskStatus(
		subtaskId: string,
		status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled'
	) {
		const state = get({ subscribe });
		return Actions.updateSubtaskStatus(state, update, subtaskId, status);
	}

	/**
	 * Get completed session history for a conversation.
	 */
	async function getSessionHistory(conversationId: string) {
		return Actions.getSessionHistory(conversationId);
	}

	/**
	 * Get full details of a specific session.
	 */
	async function getSessionDetails(sessionId: string) {
		return Actions.getSessionDetails(sessionId);
	}

	/**
	 * List all available agents.
	 */
	async function listAgents() {
		const state = get({ subscribe });
		return Actions.listAgents(state, update);
	}

	/**
	 * Get a specific agent by ID.
	 */
	async function getAgent(agentId: string) {
		const state = get({ subscribe });
		return Actions.getAgent(state, update, agentId);
	}

	/**
	 * Toggle the agent panel visibility.
	 */
	function togglePanel() {
		UiState.togglePanel(update);
	}

	/**
	 * Set the active session.
	 */
	function setActiveSession(sessionId: string | null) {
		UiState.setActiveSession(update, sessionId);
	}

	/**
	 * Clear agent error.
	 */
	function clearError() {
		UiState.clearError(update);
	}

	/**
	 * Set a callback for execution completion.
	 */
	function setExecutionCompleteCallback(
		callback: (conversationId: string, threadId: string, content: string) => void
	) {
		Callbacks.setExecutionCompleteCallback(callback);
	}

	/**
	 * Set a callback for system messages.
	 */
	function setSystemMessageCallback(callback: (conversationId: string, message: string) => void) {
		Callbacks.setSystemMessageCallback(callback);
	}

	/**
	 * Set a callback for agent activity messages.
	 */
	function setAgentActivityCallback(
		callback: (conversationId: string, threadId: string, message: Partial<any>) => void
	) {
		Callbacks.setAgentActivityCallback(callback);
		// Also initialize streaming with this callback
		Streaming.initializeStreaming(callback);
	}

	/**
	 * Set a callback to clean up transient messages.
	 */
	function setCleanupTransientCallback(
		callback: (conversationId: string, threadId: string, sessionId: string) => void
	) {
		Callbacks.setCleanupTransientCallback(callback);
	}

	return {
		subscribe,
		initialize,
		destroy,
		createSession,
		loadSession,
		sendRequest,
		approveAction,
		denyAction,
		cancelSession,
		rollbackSession,
		updateSessionWorkspace,
		createSubtask,
		updateSubtaskStatus,
		getSessionHistory,
		getSessionDetails,
		listAgents,
		getAgent,
		togglePanel,
		setActiveSession,
		clearError,
		setExecutionCompleteCallback,
		setSystemMessageCallback,
		setAgentActivityCallback,
		setCleanupTransientCallback
	};
}

export const agentStore = createAgentStore();
export { createAgentStore };
