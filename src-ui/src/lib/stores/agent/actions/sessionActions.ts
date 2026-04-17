/**
 * Session Actions
 * Session management functions for the agent store.
 */

import type { AgentSession, BackendAgentTask } from '$lib/types/agent';
import { toAgentTask } from '$lib/types/agent';
import type { AgentState } from '../state';
import { createLogger } from '$lib/utils/logger';

const log = createLogger('agentStore');

function isHydratedSession(session: AgentSession): boolean {
	return (
		session.tasks !== undefined &&
		session.filesTouched !== undefined &&
		session.terminalOutput !== undefined &&
		session.toolCalls !== undefined &&
		session.executionSteps !== undefined &&
		session.statusPhrases !== undefined
	);
}

function mergeSessionDetails(
	existingSession: AgentSession | null,
	hydratedSession: AgentSession
): AgentSession {
	if (!existingSession) {
		return hydratedSession;
	}

	return {
		...existingSession,
		...hydratedSession,
		streamingMessages: existingSession.streamingMessages ?? hydratedSession.streamingMessages,
		artifacts: existingSession.artifacts ?? hydratedSession.artifacts,
		harnessState: existingSession.harnessState ?? hydratedSession.harnessState
	};
}

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
	const requiredStringFields = [
		'id',
		'agentId',
		'conversationId',
		'threadId',
		'createdAt',
		'updatedAt'
	];
	for (const field of requiredStringFields) {
		if (typeof session[field] !== 'string') {
			log.warn('Invalid session: missing or invalid string field:', field);
			return false;
		}
	}

	// Check state field
	if (typeof session.state !== 'object' || session.state === null) {
		log.warn('Invalid session: missing or invalid state field');
		return false;
	}

	// Optional fields validation
	if (session.tasks !== undefined && !Array.isArray(session.tasks)) {
		log.warn('Invalid session: tasks must be an array');
		return false;
	}

	if (session.toolCalls !== undefined && !Array.isArray(session.toolCalls)) {
		log.warn('Invalid session: toolCalls must be an array');
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
	log.debug('[agentStore] createSession called:', {
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
			log.error('[agentStore] Invalid session data received from backend:', session);
			update((s) => ({ ...s, loading: false, error: 'Invalid session data received' }));
			return null;
		}

		log.debug('[agentStore] Session created by backend:', {
			sessionId: validatedSession.id,
			workspaceRoot: validatedSession.workspaceRoot
		});

		update((state) => {
			state.sessions.set(validatedSession.id, validatedSession);
			log.debug('[agentStore] Session stored:', {
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
	log.debug('[agentStore] sendRequest called:', { sessionId, contentLength: content.length });
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		await invoke('send_agent_request', { sessionId, content });
		log.debug('[agentStore] sendRequest succeeded');
	} catch (error) {
		log.error('[agentStore] sendRequest failed:', error);
		update((s) => ({ ...s, error: String(error) }));
	}
}

/**
 * Retry execution with the latest persisted request for a session.
 */
export async function retryLastRequest(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string
): Promise<string | null> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		const resumedSessionId = await invoke<string>('retry_last_agent_request', { sessionId });
		return resumedSessionId;
	} catch (error) {
		update((s) => ({ ...s, error: String(error) }));
		return null;
	}
}

/**
 * Continue execution from a paused session.
 */
export async function continueExecution(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string
): Promise<string | null> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		const resumedSessionId = await invoke<string>('continue_agent_execution', { sessionId });
		return resumedSessionId;
	} catch (error) {
		update((s) => ({ ...s, error: String(error) }));
		return null;
	}
}

/**
 * Replan and continue execution from a paused session.
 */
export async function replanExecution(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string
): Promise<string | null> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		const resumedSessionId = await invoke<string>('replan_agent_execution', { sessionId });
		return resumedSessionId;
	} catch (error) {
		update((s) => ({ ...s, error: String(error) }));
		return null;
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
			const session = state.sessions.get(sessionId);
			if (session) {
				state.sessions.set(sessionId, {
					...session,
					state: { type: 'cancelled', reason: 'User cancelled' }
				});
			}
			return {
				...state,
				sessions: new Map(state.sessions)
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
	log.debug('[agentStore] updateSessionWorkspace called:', { sessionId, workspacePath });
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		await invoke('update_agent_session_workspace', {
			sessionId,
			workspaceRoot: workspacePath
		});
		log.debug('[agentStore] Backend update successful');
		// Update local state after successful backend update
		update((state) => {
			const session = state.sessions.get(sessionId);
			if (session) {
				state.sessions.set(sessionId, {
					...session,
					workspaceRoot: workspacePath
				});
				log.debug('[agentStore] Local state updated');
			} else {
				log.warn('[agentStore] Session not found in local state:', sessionId);
			}
			return { ...state, sessions: new Map(state.sessions) };
		});
		log.debug('[agentStore] Workspace updated successfully');
	} catch (error) {
		log.error('[agentStore] Failed to update workspace:', error);
		update((s) => ({ ...s, error: String(error) }));
	}
}

/**
 * Update the model ID for a session.
 */
export async function updateSessionModel(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	sessionId: string,
	modelId: string
): Promise<void> {
	log.debug('[agentStore] updateSessionModel called:', { sessionId, modelId });
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		await invoke('update_agent_session_model', {
			sessionId,
			modelId
		});
		log.debug('[agentStore] Backend model update successful');
		update((state) => {
			const session = state.sessions.get(sessionId);
			if (session) {
				state.sessions.set(sessionId, {
					...session,
					modelId
				});
				log.debug('[agentStore] Local state updated with new model');
			}
			return { ...state, sessions: new Map(state.sessions) };
		});
	} catch (error) {
		log.error('[agentStore] Failed to update session model:', error);
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
						typeof tc.arguments === 'string' ? JSON.parse(tc.arguments as string) : tc.arguments;
				} catch (e) {
					log.error('[agentStore] Failed to parse tool call arguments:', e);
					parsedArgs = tc.arguments;
				}

				// Safely parse result with error handling
				let parsedResult;
				if (tc.result) {
					try {
						parsedResult =
							typeof tc.result === 'string' ? JSON.parse(tc.result as string) : tc.result;
					} catch (e) {
						log.error('[agentStore] Failed to parse tool call result:', e);
						parsedResult = tc.result;
					}
				} else {
					parsedResult = undefined;
				}

				return {
					id: tc.id,
					tool: (tc.toolName as string) ?? (tc.tool_name as string),
					arguments: parsedArgs,
					result: parsedResult,
					isError: (tc.isError as boolean) ?? false,
					durationMs:
						(tc.durationMs as number | undefined) ?? (tc.duration_ms as number | undefined),
					timestamp: (tc.timestamp as string) ?? new Date().toISOString()
				};
			});
		}
		// Validate the session structure before returning
		const session = raw as unknown as AgentSession;
		if (!isValidAgentSession(session)) {
			log.error('[agentStore] Invalid session data received from backend:', session);
			return null;
		}
		return session;
	} catch (error) {
		log.error('[agentStore] Failed to fetch session details:', error);
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
	log.debug('[agentStore] loadSession called:', sessionId);
	const existingSession = state.sessions.get(sessionId) ?? null;
	if (existingSession && isHydratedSession(existingSession)) {
		log.debug('[agentStore] Session already hydrated:', sessionId);
		return existingSession;
	}

	const hydratedSession = await getSessionDetails(sessionId);
	if (hydratedSession) {
		const mergedSession = mergeSessionDetails(existingSession, hydratedSession);
		update((state) => {
			state.sessions.set(mergedSession.id, mergedSession);
			return { ...state, sessions: new Map(state.sessions) };
		});
		log.debug('[agentStore] Session loaded and added to store:', sessionId);
		return mergedSession;
	} else {
		log.warn('[agentStore] Failed to load session or invalid session data:', sessionId);
	}
	return existingSession;
}
