/**
 * Agent Store
 * State management for agent sessions, pending actions, and backend events.
 * Will connect to Tauri IPC commands once the backend is implemented.
 */
import { writable, derived, get } from 'svelte/store';
import type { AgentSession, PendingAction, AgentEvent, SessionState } from '$lib/types/agent';
import type { Message } from '$lib/types/message';

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

	/** Callback for when agent completes execution - returns (conversationId, threadId, content) */
	let onExecutionComplete:
		| ((conversationId: string, threadId: string, content: string) => void)
		| null = null;

	/**
	 * Set a callback for execution completion.
	 * This allows the chat store to add the assistant message when the agent finishes.
	 */
	function setExecutionCompleteCallback(
		callback: (conversationId: string, threadId: string, content: string) => void
	) {
		onExecutionComplete = callback;
	}

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
		console.debug('[agentStore] Received event:', event.type, event);
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
					state.pendingActions = state.pendingActions.filter((a) => a.id !== event.actionId);
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
							terminalOutput: [...(session.terminalOutput ?? []), line]
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
							terminalOutput: [...(session.terminalOutput ?? []), line]
						});
					}
					break;
				}
				case 'subtask_created':
				case 'subtask_updated': {
					const session = state.sessions.get(event.sessionId);
					if (session && (session.tasks?.length ?? 0) > 0) {
						// Find the task that owns this subtask and add/update the subtask
						const updatedTasks = (session.tasks ?? []).map((task) => {
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
				case 'file_touched': {
					const session = state.sessions.get(event.sessionId);
					if (session) {
						const existingFiles = session.filesTouched ?? [];
						// Check if this file is already tracked
						const fileIndex = existingFiles.findIndex((f) => f.path === event.filePath);
						const newFile = {
							path: event.filePath,
							modified: event.action !== 'read',
							action: event.action
						} as import('$lib/types/agent').TouchedFile;

						let updatedFiles;
						if (fileIndex >= 0) {
							// Update existing file entry
							updatedFiles = [...existingFiles];
							updatedFiles[fileIndex] = newFile;
						} else {
							// Add new file
							updatedFiles = [...existingFiles, newFile];
						}

						state.sessions.set(event.sessionId, {
							...session,
							filesTouched: updatedFiles
						});
					}
					break;
				}
				case 'execution_started': {
					// Agent has started execution - update session state
					const session = state.sessions.get(event.sessionId);
					if (session) {
						state.sessions.set(event.sessionId, {
							...session,
							state: { type: 'running', currentStep: 0, maxSteps: 6 }
						});
					}
					break;
				}
				case 'execution_step': {
					// Track execution steps for UI progress display
					const session = state.sessions.get(event.sessionId);
					if (session) {
						const existingSteps = session.executionSteps ?? [];
						state.sessions.set(event.sessionId, {
							...session,
							executionSteps: [...existingSteps, event.step],
							state: {
								type: 'running',
								currentStep: event.step.stepNumber,
								maxSteps: 6
							}
						});
					}
					break;
				}
				case 'tool_call': {
					// Log tool call to terminal
					const session = state.sessions.get(event.sessionId);
					if (session) {
						const argsStr = JSON.stringify(event.arguments);
						const line: import('$lib/types/agent').TerminalLine = {
							id: crypto.randomUUID(),
							type: 'info',
							content: `→ ${event.tool}(${argsStr.length > 100 ? argsStr.slice(0, 100) + '...' : argsStr})`,
							timestamp: new Date().toISOString()
						};
						state.sessions.set(event.sessionId, {
							...session,
							terminalOutput: [...(session.terminalOutput ?? []), line]
						});
					}
					break;
				}
				case 'tool_result': {
					// Log tool result to terminal
					const session = state.sessions.get(event.sessionId);
					if (session) {
						const status = event.isError ? '❌' : '✓';
						const resultStr = JSON.stringify(event.result);
						const line: import('$lib/types/agent').TerminalLine = {
							id: crypto.randomUUID(),
							type: event.isError ? 'stderr' : 'stdout',
							content: `${status} ${event.tool}: ${resultStr.length > 200 ? resultStr.slice(0, 200) + '...' : resultStr}`,
							timestamp: new Date().toISOString()
						};
						state.sessions.set(event.sessionId, {
							...session,
							terminalOutput: [...(session.terminalOutput ?? []), line]
						});
					}
					break;
				}
				case 'execution_complete': {
					console.debug('[agentStore] execution_complete:', {
						sessionId: event.sessionId,
						conversationId: event.conversationId,
						threadId: event.threadId,
						contentLength: event.result.content?.length ?? 0,
						content: event.result.content
					});

					// Call the callback with conversation/thread IDs from the event
					if (onExecutionComplete) {
						console.debug('[agentStore] Calling executionComplete callback');
						onExecutionComplete(event.conversationId, event.threadId, event.result.content);
					}

					// Update session state to finished if session exists
					const session = state.sessions.get(event.sessionId);
					if (session) {
						state.sessions.set(event.sessionId, {
							...session,
							state: { type: 'finished', completedAt: new Date().toISOString() }
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
		console.debug('[agentStore] createSession called:', {
			agentId,
			conversationId,
			threadId,
			workspaceRoot
		});
		update((s) => ({ ...s, loading: true, error: null }));

		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const session = await invoke<AgentSession>('create_agent_session', {
				agentId,
				conversationId,
				threadId,
				workspaceRoot
			});

			console.debug('[agentStore] Session created by backend:', {
				sessionId: session.id,
				workspaceRoot: session.workspaceRoot
			});

			update((state) => {
				state.sessions.set(session.id, session);
				console.debug('[agentStore] Session stored:', {
					sessionId: session.id,
					totalSessions: state.sessions.size,
					conversationId: session.conversationId,
					threadId: session.threadId
				});
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
		console.debug('[agentStore] sendRequest called:', { sessionId, contentLength: content.length });
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('send_agent_request', { sessionId, content });
			console.debug('[agentStore] sendRequest succeeded');
		} catch (error) {
			console.error('[agentStore] sendRequest failed:', error);
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
					activeSessionId: state.activeSessionId === sessionId ? null : state.activeSessionId
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
	 * Update the workspace root for a session.
	 */
	async function updateSessionWorkspace(sessionId: string, workspacePath: string) {
		console.debug('[agentStore] updateSessionWorkspace called:', { sessionId, workspacePath });
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('update_agent_session_workspace', {
				sessionId,
				workspaceRoot: workspacePath
			});
			console.debug('[agentStore] Backend update successful');
			// Update local state after successful backend update
			update((state) => {
				const session = state.sessions.get(sessionId);
				if (session) {
					state.sessions.set(sessionId, {
						...session,
						workspaceRoot: workspacePath
					});
					console.debug('[agentStore] Local state updated');
				} else {
					console.warn('[agentStore] Session not found in local state:', sessionId);
				}
				return { ...state, sessions: new Map(state.sessions) };
			});
			console.debug('[agentStore] Workspace updated successfully');
		} catch (error) {
			console.error('[agentStore] Failed to update workspace:', error);
			update((s) => ({ ...s, error: String(error) }));
		}
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
	 * Get completed session history for a conversation.
	 */
	async function getSessionHistory(
		conversationId: string
	): Promise<import('$lib/types/agent').AgentSession[]> {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const history = await invoke<import('$lib/types/agent').AgentSession[]>(
				'get_conversation_session_history',
				{ conversationId }
			);
			return history;
		} catch (error) {
			update((s) => ({ ...s, error: String(error) }));
			return [];
		}
	}

	/**
	 * Get full details of a specific session.
	 */
	async function getSessionDetails(
		sessionId: string
	): Promise<import('$lib/types/agent').AgentSession | null> {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const session = await invoke<import('$lib/types/agent').AgentSession>('get_session_details', {
				sessionId
			});
			return session;
		} catch (error) {
			update((s) => ({ ...s, error: String(error) }));
			return null;
		}
	}

	/**
	 * Toggle the agent panel visibility.
	 */
	function togglePanel() {
		update((s) => ({ ...s, panelOpen: !s.panelOpen }));
	}

	/**
	 * Load a session from the backend and add it to the store.
	 * Returns the session if loaded successfully, null otherwise.
	 */
	async function loadSession(sessionId: string): Promise<AgentSession | null> {
		console.debug('[agentStore] loadSession called:', sessionId);
		// First check if already loaded
		const state = get({ subscribe });
		if (state.sessions.has(sessionId)) {
			console.debug('[agentStore] Session already loaded:', sessionId);
			return state.sessions.get(sessionId) ?? null;
		}

		// Fetch from backend
		const session = await getSessionDetails(sessionId);
		if (session) {
			update((state) => {
				state.sessions.set(session.id, session);
				return { ...state, sessions: new Map(state.sessions) };
			});
			console.debug('[agentStore] Session loaded and added to store:', sessionId);
		} else {
			console.warn('[agentStore] Failed to load session:', sessionId);
		}
		return session;
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
	 * List all available agents.
	 */
	async function listAgents(): Promise<import('$lib/types/agent').Agent[]> {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const agents = await invoke<import('$lib/types/agent').Agent[]>('list_agents');
			return agents;
		} catch (error) {
			update((s) => ({ ...s, error: String(error) }));
			return [];
		}
	}

	/**
	 * Get a specific agent by ID.
	 */
	async function getAgent(agentId: string): Promise<import('$lib/types/agent').Agent | null> {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			const agent = await invoke<import('$lib/types/agent').Agent>('get_agent', {
				agentId
			});
			return agent;
		} catch (error) {
			update((s) => ({ ...s, error: String(error) }));
			return null;
		}
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
		setExecutionCompleteCallback,
		createSession,
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
		loadSession,
		listAgents,
		getAgent,
		togglePanel,
		setActiveSession,
		clearError,
		destroy
	};
}

export const agentStore = createAgentStore();

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
