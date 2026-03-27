/**
 * Session Actions
 * Session management functions for the agent store.
 */

import type {AgentSession, BackendAgentTask} from '$lib/types/agent';
import {toAgentTask} from '$lib/types/agent';
import type {AgentState} from '../state';

/**
 * Runtime type guard to validate AgentSession structure.
 * Prevents runtime crashes from malformed backend data.
 */
function isValidAgentSession(obj: unknown): obj is AgentSession {
	if (typeof obj !== 'object' || obj === null) {
		return false;
	}

	const session = obj as Record<string, unknown>;

	// Check required string fields
	const requiredStringFields = ['id', 'agentId', 'conversationId', 'threadId', 'createdAt', 'updatedAt'];
	for (const field of requiredStringFields) {
		if (typeof session[field] !== 'string') {
			console.warn('[agentStore] Invalid session: missing or invalid string field:', field);
			return false;
		}
	}

	// Check state field
	if (typeof session.state !== 'object' || session.state === null) {
		console.warn('[agentStore] Invalid session: missing or invalid state field');
		return false;
	}

	// Optional fields validation
	if (session.tasks !== undefined && !Array.isArray(session.tasks)) {
		console.warn('[agentStore] Invalid session: tasks must be an array');
		return false;
	}

	if (session.toolCalls !== undefined && !Array.isArray(session.toolCalls)) {
		console.warn('[agentStore] Invalid session: toolCalls must be an array');
		return false;
	}

	return true;
}

/**
 * Create a new agent session.
 */
export async function createSession(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
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
		const session = await invoke<Record<string, unknown>>('create_agent_session', {
			agentId,
			conversationId,
			threadId,
			workspaceRoot
		});

		// Validate session structure before using it
		const validatedSession = session as unknown as AgentSession;
		if (!isValidAgentSession(validatedSession)) {
			console.error('[agentStore] Invalid session data received from backend:', session);
			update((s) => ({ ...s, loading: false, error: 'Invalid session data received' }));
			return null;
		}

		console.debug('[agentStore] Session created by backend:', {
			sessionId: validatedSession.id,
			workspaceRoot: validatedSession.workspaceRoot
		});

		update((state) => {
			state.sessions.set(validatedSession.id, validatedSession);
			console.debug('[agentStore] Session stored:', {
				sessionId: validatedSession.id,
				totalSessions: state.sessions.size,
				conversationId: validatedSession.conversationId,
				threadId: validatedSession.threadId
			});
			return {
				...state,
				sessions: new Map(state.sessions),
				activeSessionId: validatedSession.id,
				panelOpen: true,
				loading: false
			};
		});

		return validatedSession;
	} catch (error) {
		update((s) => ({ ...s, loading: false, error: String(error) }));
		return null;
	}
}

/**
 * Send a request to an active agent session.
 */
export async function sendRequest(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string,
	content: string
): Promise<void> {
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
 * Cancel an active session.
 */
export async function cancelSession(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string
): Promise<void> {
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
export async function rollbackSession(sessionId: string): Promise<string[]> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		const reverted = await invoke<string[]>('rollback_session', { sessionId });
		return reverted;
	} catch (error) {
		return [];
	}
}

/**
 * Update the workspace root for a session.
 */
export async function updateSessionWorkspace(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string,
	workspacePath: string
): Promise<void> {
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
 * Get completed session history for a conversation.
 */
export async function getSessionHistory(conversationId: string): Promise<AgentSession[]> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		const history = await invoke<AgentSession[]>('get_conversation_session_history', {
			conversationId
		});
		return history;
	} catch (error) {
		return [];
	}
}

/**
 * Get full details of a specific session.
 */
export async function getSessionDetails(sessionId: string): Promise<AgentSession | null> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		// Backend returns SessionDetailsResponse with flattened session + tasks in backend format
		const raw = await invoke<Record<string, unknown>>('get_session_details', {
			sessionId
		});
		// Transform backend tasks to frontend AgentTask format
		const backendTasks = raw.tasks as BackendAgentTask[] | undefined;
		if (backendTasks && Array.isArray(backendTasks)) {
			(raw as Record<string, unknown>).tasks = backendTasks.map(toAgentTask);
		}
		// Transform backend tool calls to frontend ToolCallRecord format
		const backendToolCalls = raw.toolCalls as Record<string, unknown>[] | undefined;
		if (backendToolCalls && Array.isArray(backendToolCalls)) {
			(raw as Record<string, unknown>).toolCalls = backendToolCalls.map((tc) => {
				// Safely parse arguments with error handling
				let parsedArgs;
				try {
					parsedArgs =
						typeof tc.arguments === 'string'
							? JSON.parse(tc.arguments as string)
							: tc.arguments;
				} catch (e) {
					console.error('[agentStore] Failed to parse tool call arguments:', e);
					parsedArgs = tc.arguments;
				}

				// Safely parse result with error handling
				let parsedResult;
				if (tc.result) {
					try {
						parsedResult =
							typeof tc.result === 'string'
								? JSON.parse(tc.result as string)
								: tc.result;
					} catch (e) {
						console.error('[agentStore] Failed to parse tool call result:', e);
						parsedResult = tc.result;
					}
				} else {
					parsedResult = undefined;
				}

				return {
					id: tc.id,
					tool: tc.toolName,
					arguments: parsedArgs,
					result: parsedResult,
					isError: (tc.isError as boolean) ?? false,
					durationMs: tc.durationMs,
					timestamp: tc.timestamp
				};
			});
		}
		// Validate the session structure before returning
		const session = raw as unknown as AgentSession;
		if (!isValidAgentSession(session)) {
			console.error('[agentStore] Invalid session data received from backend:', session);
			return null;
		}
		return session;
	} catch (error) {
		console.error('[agentStore] Failed to fetch session details:', error);
		return null;
	}
}

/**
 * Load a session from the backend and add it to the store.
 */
export async function loadSession(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string
): Promise<AgentSession | null> {
	console.debug('[agentStore] loadSession called:', sessionId);
	// First check if already loaded
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
		console.warn('[agentStore] Failed to load session or invalid session data:', sessionId);
	}
	return session;
}
