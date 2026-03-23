/**
 * Agent Store
 * State management for agent sessions, pending actions, and backend events.
 * Will connect to Tauri IPC commands once the backend is implemented.
 */
import { writable, derived } from 'svelte/store';
import type {
	AgentSession,
	PendingAction,
	AgentEvent,
	SessionState
} from '$lib/types/agent';

interface AgentState {
	sessions: Map<string, AgentSession>;
	activeSessionId: string | null;
	pendingActions: PendingAction[];
	panelOpen: boolean;
	loading: boolean;
	error: string | null;
}

function createAgentStore() {
	const { subscribe, set, update } = writable<AgentState>({
		sessions: new Map(),
		activeSessionId: null,
		pendingActions: [],
		panelOpen: false,
		loading: false,
		error: null
	});

	let eventUnsubscribe: (() => void) | null = null;

	/**
	 * Initialize the agent store and listen for backend events.
	 * Called once on app startup.
	 */
	async function initialize() {
		try {
			// Dynamic import to avoid SSR issues with Tauri APIs
			const { listen } = await import('@tauri-apps/api/event');
			const unlisten = await listen<AgentEvent>('agent:event', (event) => {
				handleAgentEvent(event.payload);
			});
			eventUnsubscribe = unlisten;
		} catch {
			// Tauri not available (running in browser dev mode)
			console.debug('[agentStore] Tauri event listener not available');
		}
	}

	/**
	 * Handle agent events from the backend.
	 */
	function handleAgentEvent(event: AgentEvent) {
		update((state) => {
			switch (event.type) {
				case 'session_state_changed': {
					const session = state.sessions.get(event.sessionId);
					if (session) {
						state.sessions.set(event.sessionId, {
							...session,
							state: event.state
						});
					}
					break;
				}
				case 'approval_required':
					state.pendingActions = [...state.pendingActions, event.action];
					break;
				case 'approval_approved':
				case 'approval_denied':
					state.pendingActions = state.pendingActions.filter(
						(a) => a.id !== event.actionId
					);
					break;
				case 'execution_error': {
					const session = state.sessions.get(event.sessionId);
					if (session) {
						state.sessions.set(event.sessionId, {
							...session,
							state: { type: 'error', message: event.error, recoverable: false }
						});
					}
					break;
				}
				case 'command_output': {
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
							terminalOutput: [...session.terminalOutput, line]
						});
					}
					break;
				}
				case 'command_finished': {
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
							terminalOutput: [...session.terminalOutput, line]
						});
					}
					break;
				}
				case 'subtask_created':
				case 'subtask_updated': {
					const session = state.sessions.get(event.sessionId);
					if (session && session.tasks.length > 0) {
						// Find the task that owns this subtask and add/update the subtask
						const updatedTasks = session.tasks.map((task) => {
							if (task.id === event.subtask.taskId) {
								const existingSubtasks = task.subtasks ?? [];
								const subtaskIndex = existingSubtasks.findIndex((st) => st.id === event.subtask.id);
								if (subtaskIndex >= 0) {
									// Update existing subtask
									const newSubtasks = [...existingSubtasks];
									newSubtasks[subtaskIndex] = event.subtask;
									return { ...task, subtasks: newSubtasks };
								} else {
									// Add new subtask, maintaining order
									const newSubtasks = [...existingSubtasks, event.subtask].sort(
										(a, b) => a.order - b.order
									);
									return { ...task, subtasks: newSubtasks };
								}
							}
							return task;
						});
						state.sessions.set(event.sessionId, {
							...session,
							tasks: updatedTasks
						});
					}
					break;
				}
			}
			return { ...state, sessions: new Map(state.sessions) };
		});
	}

	/**
	 * Create a new agent session.
	 */
	async function createSession(
		agentId: string,
		conversationId: string,
		threadId: string,
		workspaceRoot?: string
	): Promise<AgentSession | null> {
		update((s) => ({ ...s, loading: true, error: null }));

		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const session = await invoke<AgentSession>('create_agent_session', {
				agentId,
				conversationId,
				threadId,
				workspaceRoot
			});

			update((state) => {
				state.sessions.set(session.id, session);
				return {
					...state,
					sessions: new Map(state.sessions),
					activeSessionId: session.id,
					panelOpen: true,
					loading: false
				};
			});

			return session;
		} catch (error) {
			update((s) => ({ ...s, loading: false, error: String(error) }));
			return null;
		}
	}

	/**
	 * Send a request to an active agent session.
	 * Execution happens asynchronously; results arrive via events.
	 */
	async function sendRequest(sessionId: string, content: string): Promise<void> {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('send_agent_request', { sessionId, content });
		} catch (error) {
			update((s) => ({ ...s, error: String(error) }));
		}
	}

	/**
	 * Approve a pending action.
	 */
	async function approveAction(sessionId: string, actionId: string): Promise<void> {
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
	async function denyAction(sessionId: string, actionId: string): Promise<void> {
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

	/**
	 * Cancel an active session.
	 */
	async function cancelSession(sessionId: string): Promise<void> {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('cancel_agent_session', { sessionId });
			update((state) => {
				state.sessions.delete(sessionId);
				return {
					...state,
					sessions: new Map(state.sessions),
					activeSessionId:
						state.activeSessionId === sessionId ? null : state.activeSessionId
				};
			});
		} catch (error) {
			update((s) => ({ ...s, error: String(error) }));
		}
	}

	/**
	 * Roll back all file changes made during a session.
	 */
	async function rollbackSession(sessionId: string): Promise<string[]> {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const reverted = await invoke<string[]>('rollback_session', { sessionId });
			return reverted;
		} catch (error) {
			update((s) => ({ ...s, error: String(error) }));
			return [];
		}
	}

	/**
	 * Update the workspace root for a session (local state only).
	 */
	function updateSessionWorkspace(sessionId: string, workspacePath: string) {
		update((state) => {
			const session = state.sessions.get(sessionId);
			if (session) {
				state.sessions.set(sessionId, {
					...session,
					workspaceRoot: workspacePath
				});
			}
			return { ...state, sessions: new Map(state.sessions) };
		});
	}

	/**
	 * Create a new subtask for task decomposition.
	 */
	async function createSubtask(
		taskId: string,
		title: string,
		description?: string,
		order?: number
	): Promise<string | null> {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const subtaskId = await invoke<string>('create_subtask', {
				taskId,
				title,
				description
			});
			return subtaskId;
		} catch (error) {
			update((s) => ({ ...s, error: String(error) }));
			return null;
		}
	}

	/**
	 * Update subtask status.
	 */
	async function updateSubtaskStatus(
		subtaskId: string,
		status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled'
	): Promise<void> {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('update_subtask_status', {
				subtaskId,
				status: JSON.stringify({ type: status })
			});
		} catch (error) {
			update((s) => ({ ...s, error: String(error) }));
		}
	}

	/**
	 * Toggle the agent panel visibility.
	 */
	function togglePanel() {
		update((s) => ({ ...s, panelOpen: !s.panelOpen }));
	}

	/**
	 * Set the active session.
	 */
	function setActiveSession(sessionId: string | null) {
		update((s) => ({ ...s, activeSessionId: sessionId }));
	}

	/**
	 * Clear agent error.
	 */
	function clearError() {
		update((s) => ({ ...s, error: null }));
	}

	/**
	 * Cleanup event listeners.
	 */
	function destroy() {
		eventUnsubscribe?.();
	}

	return {
		subscribe,
		initialize,
		createSession,
		sendRequest,
		approveAction,
		denyAction,
		cancelSession,
		rollbackSession,
		updateSessionWorkspace,
		createSubtask,
		updateSubtaskStatus,
		togglePanel,
		setActiveSession,
		clearError,
		destroy
	};
}

export const agentStore = createAgentStore();

/** Currently active agent session */
export const activeSession = derived(agentStore, ($state) =>
	$state.activeSessionId ? $state.sessions.get($state.activeSessionId) ?? null : null
);

/** All sessions as an array */
export const allSessions = derived(agentStore, ($state) =>
	Array.from($state.sessions.values())
);

/** Pending actions for the active session */
export const pendingActions = derived(agentStore, ($state) => $state.pendingActions);

/** Whether the agent panel is open */
export const agentPanelOpen = derived(agentStore, ($state) => $state.panelOpen);

/** Whether the agent store is loading */
export const agentLoading = derived(agentStore, ($state) => $state.loading);

/** Agent store error */
export const agentError = derived(agentStore, ($state) => $state.error);
