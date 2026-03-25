/**
 * Session Actions
 * Session management functions for the agent store.
 */

import type { AgentSession } from '$lib/types/agent';
import type { BackendAgentTask } from '$lib/types/agent';
import { toAgentTask } from '$lib/types/agent';
import type { AgentState } from '../state';

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
			(raw as Record<string, unknown>).toolCalls = backendToolCalls.map((tc) => ({
				id: tc.id,
				tool: tc.toolName,
				arguments:
					typeof tc.arguments === 'string'
						? JSON.parse(tc.arguments as string)
						: tc.arguments,
				result: tc.result
					? typeof tc.result === 'string'
						? JSON.parse(tc.result as string)
						: tc.result
					: undefined,
				isError: (tc.isError as boolean) ?? false,
				durationMs: tc.durationMs,
				timestamp: tc.timestamp
			}));
		}
		return raw as unknown as AgentSession;
	} catch (error) {
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
		console.warn('[agentStore] Failed to load session:', sessionId);
	}
	return session;
}
