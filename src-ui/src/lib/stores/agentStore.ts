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

	/** Callback for agent system messages (e.g., context compaction) - returns (conversationId, message) */
	let onSystemMessage: ((conversationId: string, message: string) => void) | null = null;

	/** Callback for agent activity messages (thinking, tool_call, terminal) - adds transient messages to chat */
	let onAgentActivity: ((conversationId: string, threadId: string, message: Partial<Message>) => void) | null = null;

	// Streaming throttle state
	const pendingStreamingUpdates = new Map<string, {
		conversationId: string;
		threadId: string;
		messageId: string;
		content: string;
		status: string;
	}>();
	const streamingThrottleTimers = new Map<string, ReturnType<typeof setTimeout>>();
	const STREAM_THROTTLE_MS = 50; // Update UI at most every 50ms during streaming

	/**
	 * Flush a pending streaming update to the UI
	 */
	function flushStreamingUpdate(sessionId: string) {
		const pending = pendingStreamingUpdates.get(sessionId);
		if (pending && onAgentActivity) {
			onAgentActivity(pending.conversationId, pending.threadId, {
				id: pending.messageId,
				conversationId: pending.conversationId,
				threadId: pending.threadId,
				role: 'assistant',
				content: pending.content,
				timestamp: new Date(),
				status: pending.status as any
			});
			pendingStreamingUpdates.delete(sessionId);
		}
	}

	/**
	 * Schedule a throttled streaming update
	 */
	function scheduleStreamingUpdate(
		sessionId: string,
		conversationId: string,
		threadId: string,
		messageId: string,
		content: string,
		status: string,
		immediate = false
	) {
		// Store the update
		pendingStreamingUpdates.set(sessionId, {
			conversationId,
			threadId,
			messageId,
			content,
			status
		});

		// Clear existing timer
		const existingTimer = streamingThrottleTimers.get(sessionId);
		if (existingTimer) {
			clearTimeout(existingTimer);
		}

		if (immediate) {
			// Flush immediately for final chunk
			flushStreamingUpdate(sessionId);
			streamingThrottleTimers.delete(sessionId);
		} else {
			// Schedule throttled update
			const timer = setTimeout(() => {
				flushStreamingUpdate(sessionId);
				streamingThrottleTimers.delete(sessionId);
			}, STREAM_THROTTLE_MS);
			streamingThrottleTimers.set(sessionId, timer);
		}
	}

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
	 * Set a callback for system messages from the agent.
	 * This allows adding informative messages to the chat (e.g., context compaction).
	 */
	function setSystemMessageCallback(callback: (conversationId: string, message: string) => void) {
		onSystemMessage = callback;
	}

	/**
	 * Set a callback for agent activity messages.
	 * This allows transient messages (thinking, tool_call, terminal) to be added to the chat.
	 */
	function setAgentActivityCallback(
		callback: (conversationId: string, threadId: string, message: Partial<Message>) => void
	) {
		onAgentActivity = callback;
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
					// Add to terminal and create terminal output message in chat
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

						// Create terminal output message in chat (transient, auto-dismiss)
						if (onAgentActivity) {
							onAgentActivity(session.conversationId, session.threadId, {
								id: `terminal-${event.sessionId}-${crypto.randomUUID()}`,
								conversationId: session.conversationId,
								threadId: session.threadId,
								role: 'system',
								kind: 'terminal',
								content: event.content,
								agentData: {
									streamType: event.stream,
									sessionId: event.sessionId
								},
								isTransient: true,
								timestamp: new Date()
							});
						}
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
							action: event.action,
							// Include diff if provided by the event
							...(event.diff !== undefined ? { diff: event.diff } : {})
						} as import('$lib/types/agent').TouchedFile;

						let updatedFiles;
						if (fileIndex >= 0) {
							// Update existing file entry, preserving existing diff if new one not provided
							updatedFiles = [...existingFiles];
							updatedFiles[fileIndex] = {
								...newFile,
								// Keep existing diff if the new event doesn't have one
								diff: event.diff ?? existingFiles[fileIndex].diff
							};
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
					// Agent has started execution - update session state and create thinking message
					const session = state.sessions.get(event.sessionId);
					if (session) {
						console.debug('[agentStore] execution_started:', {
							sessionId: event.sessionId,
							conversationId: session.conversationId,
							threadId: session.threadId
						});

						state.sessions.set(event.sessionId, {
							...session,
							state: { type: 'running', currentStep: 0, contextUsage: 0 }
						});

						// Create thinking message in chat with consistent ID for updates
						if (onAgentActivity) {
							console.debug('[agentStore] Creating thinking message');
							onAgentActivity(session.conversationId, session.threadId, {
								id: `thinking-${event.sessionId}`,
								conversationId: session.conversationId,
								threadId: session.threadId,
								role: 'system',
								kind: 'thinking',
								content: 'Thinking...',
								timestamp: new Date(),
								isTransient: false
							});
						}
					}
					break;
				}
				case 'execution_step': {
					// Track execution steps for UI progress display
					const session = state.sessions.get(event.sessionId);
					if (session) {
						const existingSteps = session.executionSteps ?? [];
						// Preserve context usage from current state
						const currentUsage = session.state.type === 'running' ? session.state.contextUsage : 0;
						state.sessions.set(event.sessionId, {
							...session,
							executionSteps: [...existingSteps, event.step],
							state: {
								type: 'running',
								currentStep: event.step.stepNumber,
								contextUsage: currentUsage
							}
						});
					}
					break;
				}
				case 'tool_call': {
					// Log tool call to terminal and create tool call message in chat
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

						// Create tool call message in chat
						// Use a consistent ID that can be updated when tool_result arrives
						const toolCallId = `tool-${event.sessionId}-${event.tool}`;
						if (onAgentActivity) {
							onAgentActivity(session.conversationId, session.threadId, {
								id: toolCallId,
								conversationId: session.conversationId,
								threadId: session.threadId,
								role: 'system',
								kind: 'tool_call',
								content: '',
								agentData: {
									toolName: event.tool,
									toolArgs: event.arguments,
									sessionId: event.sessionId,
									expanded: false,
									// Track that this is a call (not result yet)
									toolCallStatus: 'pending'
								},
								timestamp: new Date()
							});
						}
					}
					break;
				}
				case 'tool_result': {
					// Log tool result to terminal and update tool call message
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

						// Update the existing tool call message with result
						// Use the SAME ID as the tool_call so it updates the existing message
						const toolCallId = `tool-${event.sessionId}-${event.tool}`;
						if (onAgentActivity) {
							onAgentActivity(session.conversationId, session.threadId, {
								id: toolCallId, // Same ID as tool_call to update the existing message
								conversationId: session.conversationId,
								threadId: session.threadId,
								role: 'system',
								kind: 'tool_call',
								content: '',
								agentData: {
									toolName: event.tool,
									toolResult: event.result,
									toolError: event.isError,
									sessionId: event.sessionId,
									toolCallStatus: event.isError ? 'error' : 'success'
								},
								timestamp: new Date()
							});
						}
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
				case 'status_phrase': {
					// Accumulate status phrases for rotation (keep last 10)
					// and update thinking message in chat
					const session = state.sessions.get(event.sessionId);
					if (session) {
						const existingPhrases = session.statusPhrases ?? [];
						// Keep only the last 10 phrases
						const newPhrases = [...existingPhrases, event.message].slice(-10);
						state.sessions.set(event.sessionId, {
							...session,
							statusPhrases: newPhrases
						});

						console.debug('[agentStore] status_phrase:', {
							sessionId: event.sessionId,
							message: event.message,
							conversationId: session.conversationId,
							threadId: session.threadId
						});

						// Update thinking message in chat with latest status
						if (onAgentActivity) {
							onAgentActivity(session.conversationId, session.threadId, {
								id: `thinking-${event.sessionId}`,
								conversationId: session.conversationId,
								threadId: session.threadId,
								role: 'system',
								kind: 'thinking',
								content: event.message,
								timestamp: new Date()
							});
						}
					}
					break;
				}
				case 'streaming_content': {
					// Content streaming in for final response (throttled to prevent UI freezing)
					const session = state.sessions.get(event.sessionId);
					if (session && onAgentActivity) {
						// For streaming content, we use a special message ID for the assistant response
						const assistantMessageId = `streaming-assistant-${event.sessionId}`;

						// Track accumulated content in session state
						const existingContent = session.streamingAssistantContent ?? '';
						const newContent = existingContent + event.chunk;

						if (!event.isFinal) {
							// Update streaming state
							state.sessions.set(event.sessionId, {
								...session,
								streamingAssistantContent: newContent
							});

							// Schedule throttled UI update (non-blocking)
							scheduleStreamingUpdate(
								event.sessionId,
								session.conversationId,
								session.threadId,
								assistantMessageId,
								newContent,
								'sending'
							);
						}
						// is_final: flush immediately with 'sent' status and clear streaming state
						else {
							// Clear any pending throttled update
							const timer = streamingThrottleTimers.get(event.sessionId);
							if (timer) {
								clearTimeout(timer);
								streamingThrottleTimers.delete(event.sessionId);
							}

							// Clear streaming state
							state.sessions.set(event.sessionId, {
								...session,
								streamingAssistantContent: undefined
							});

							// If we have accumulated content, flush immediately
							if (newContent.length > 0) {
								scheduleStreamingUpdate(
									event.sessionId,
									session.conversationId,
									session.threadId,
									assistantMessageId,
									newContent,
									'sent',
									true // immediate flush
								);
							}
						}
					}
					break;
				}
				case 'context_compacted': {
					// Context was compacted - send system message to chat
					if (onSystemMessage) {
						const summary = `🗜️ **Context compacted**\n\n${event.beforeCount} → ${event.afterCount} messages\n${event.beforeTokens.toLocaleString()} → ${event.afterTokens.toLocaleString()} tokens`;
						onSystemMessage(event.conversationId, summary);
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
		setSystemMessageCallback,
		setAgentActivityCallback,
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
