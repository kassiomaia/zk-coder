/**
 * Session Events
 * Handles session-related agent events.
 */

import type { AgentEvent } from '$lib/types/agent';
import type { AgentState } from '../state';
import { getOnAgentActivity, getOnCleanupTransients, getOnExecutionComplete } from '../callbacks';
import { clearStatusPhrases } from '../statusStore';

/**
 * Handle session_state_changed event.
 */
export function handleSessionStateChanged(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'session_state_changed' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		state.sessions.set(event.sessionId, {
			...session,
			state: event.state
		});
	}
}

/**
 * Handle execution_started event.
 */
export function handleExecutionStarted(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'execution_started' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		state.sessions.set(event.sessionId, {
			...session,
			state: { type: 'running', currentStep: 0, contextUsage: 0 },
			executionSteps: [],
			toolCalls: [],
			terminalOutput: [],
			streamingMessages: undefined,
			artifacts: undefined // Clear any artifacts from previous execution
		});
		// Reset the side-channel status store so the indicator does
		// not carry stale phrases from the previous execution.
		clearStatusPhrases(event.sessionId);

		// Create thinking message in chat with consistent ID for updates
		const onAgentActivity = getOnAgentActivity();
		if (onAgentActivity) {
			onAgentActivity(session.conversationId, session.threadId, {
				id: `thinking-${event.sessionId}`,
				conversationId: session.conversationId,
				threadId: session.threadId,
				role: 'assistant',
				kind: 'thinking',
				content: '',
				timestamp: new Date(),
				agentData: {
					sessionId: event.sessionId
				}
			});
		}
	}
}

/**
 * Handle execution_resumed event. Emitted from the backend when
 * bootstrap reuses an existing plan instead of generating a new one.
 * The existing task + subtasks are already loaded in the UI, so this
 * handler only refreshes session lifecycle bookkeeping — it must NOT
 * clear streamingMessages/artifacts/toolCalls since those may be
 * meaningful session history from the interrupted run.
 */
export function handleExecutionResumed(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'execution_resumed' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		state.sessions.set(event.sessionId, {
			...session,
			state: { type: 'running', currentStep: 0, contextUsage: 0 }
		});
		clearStatusPhrases(event.sessionId);

		const onAgentActivity = getOnAgentActivity();
		if (onAgentActivity) {
			onAgentActivity(session.conversationId, session.threadId, {
				id: `thinking-${event.sessionId}`,
				conversationId: session.conversationId,
				threadId: session.threadId,
				role: 'assistant',
				kind: 'thinking',
				content: '',
				timestamp: new Date(),
				agentData: {
					sessionId: event.sessionId
				}
			});
		}
	}
}

/**
 * Handle execution_step event.
 */
export function handleExecutionStep(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'execution_step' }>
) {
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
}

/**
 * Handle execution_complete event.
 */
export function handleExecutionComplete(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'execution_complete' }>
) {
	// Call the callback with conversation/thread IDs from the event
	const onExecutionComplete = getOnExecutionComplete();
	if (onExecutionComplete) {
		const session = state.sessions.get(event.sessionId);
		const artifacts = session?.artifacts ?? [];
		onExecutionComplete(
			event.sessionId,
			event.conversationId,
			event.threadId,
			event.result.content,
			artifacts
		);

		// Clear artifacts from session state after attaching to message
		// (prevents accumulation across multiple executions in the same session)
		if (session && artifacts.length > 0) {
			state.sessions.set(event.sessionId, {
				...session,
				artifacts: undefined
			});
		}
	}

	// Update session state to finished if session exists
	const session = state.sessions.get(event.sessionId);
	if (session) {
		state.sessions.set(event.sessionId, {
			...session,
			state: { type: 'finished', completedAt: new Date().toISOString() }
		});
	}

	// Clean up transient messages (tool_call, thinking) from chat —
	// the Code Agent panel is the authoritative source for tool call history
	const onCleanupTransients = getOnCleanupTransients();
	if (onCleanupTransients && session) {
		onCleanupTransients(session.conversationId, session.threadId, event.sessionId);
	}
}

/**
 * Handle execution_error event.
 */
export function handleExecutionError(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'execution_error' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		state.sessions.set(event.sessionId, {
			...session,
			state: { type: 'error', message: event.error, recoverable: false }
		});
	}

	// Clean up transient messages (tool_call, thinking) on error too
	const onCleanupTransients = getOnCleanupTransients();
	if (onCleanupTransients && session) {
		onCleanupTransients(session.conversationId, session.threadId, event.sessionId);
	}
}

/**
 * Handle approval events (required, approved, denied).
 */
export function handleApprovalRequired(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'approval_required' }>
) {
	state.pendingActions = [...state.pendingActions, event.action];

	// Inject approval card into chat
	const session = state.sessions.get(event.sessionId);
	const onAgentActivity = getOnAgentActivity();
	if (onAgentActivity && session) {
		onAgentActivity(session.conversationId, session.threadId, {
			id: `approval-${event.action.id}`,
			conversationId: session.conversationId,
			threadId: session.threadId,
			role: 'assistant',
			kind: 'approval',
			content: event.action.description,
			timestamp: new Date(),
			agentData: {
				sessionId: event.sessionId,
				pendingAction: event.action,
				approvalResolved: false
			}
		});
	}
}

export function handleApprovalApproved(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'approval_approved' }>
) {
	state.pendingActions = state.pendingActions.filter((a) => a.id !== event.actionId);

	// Update approval card to show approved state
	const onAgentActivity = getOnAgentActivity();
	const session = state.sessions.get(event.sessionId);
	if (onAgentActivity && session) {
		onAgentActivity(session.conversationId, session.threadId, {
			id: `approval-${event.actionId}`,
			agentData: {
				sessionId: event.sessionId,
				approvalResolved: true,
				approvalDecision: 'approved'
			}
		});
	}
}

export function handleApprovalDenied(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'approval_denied' }>
) {
	state.pendingActions = state.pendingActions.filter((a) => a.id !== event.actionId);

	// Update approval card to show denied state
	const onAgentActivity = getOnAgentActivity();
	const session = state.sessions.get(event.sessionId);
	if (onAgentActivity && session) {
		onAgentActivity(session.conversationId, session.threadId, {
			id: `approval-${event.actionId}`,
			agentData: {
				sessionId: event.sessionId,
				approvalResolved: true,
				approvalDecision: 'denied'
			}
		});
	}
}

/**
 * Handle clarification_replied event (agent-clarification-resume plan, Phase 7).
 *
 * Appends the reply to the chat transcript as a user message and
 * clears any transient "awaiting" affordance from the session state.
 */
export function handleClarificationReplied(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'clarification_replied' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (!session) return;

	const onAgentActivity = getOnAgentActivity();
	if (onAgentActivity) {
		onAgentActivity(session.conversationId, session.threadId, {
			id: `clarification-reply-${event.sessionId}-${Date.now()}`,
			conversationId: session.conversationId,
			threadId: session.threadId,
			role: 'user',
			content: event.content,
			timestamp: new Date(),
			agentData: {
				sessionId: event.sessionId,
				clarificationReply: true,
				subtaskId: event.subtaskId
			}
		});
	}
}

/**
 * Handle clarification_timed_out event (agent-clarification-resume plan, Phase 7).
 *
 * Appends a system message explaining the timeout and surfaces a
 * retry affordance via the existing error/paused UX.
 */
export function handleClarificationTimedOut(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'clarification_timed_out' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (!session) return;

	const onAgentActivity = getOnAgentActivity();
	if (onAgentActivity) {
		onAgentActivity(session.conversationId, session.threadId, {
			id: `clarification-timeout-${event.sessionId}-${Date.now()}`,
			conversationId: session.conversationId,
			threadId: session.threadId,
			role: 'system',
			content: `The agent didn't hear back in time — ${event.prompt}`,
			timestamp: new Date(),
			agentData: {
				sessionId: event.sessionId,
				clarificationTimedOut: true,
				subtaskId: event.subtaskId
			}
		});
	}
}

/**
 * Handle clarification_restored event (agent-clarification-resume plan, Phase 7).
 *
 * Fires on app startup when a paused session is rehydrated from
 * persistent storage. Updates the local session state to reflect the
 * pause so the composer + panel render the question inline.
 */
export function handleClarificationRestored(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'clarification_restored' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (!session) return;

	state.sessions.set(event.sessionId, {
		...session,
		state: {
			type: 'awaiting_clarification',
			prompt: event.prompt,
			askedAt: event.askedAt,
			subtaskId: event.subtaskId
		}
	});
}

/**
 * Handle artifact_created event.
 * Stores the artifact reference in the session for later attachment to the final message.
 */
export function handleArtifactCreated(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'artifact_created' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		const existingArtifacts = session.artifacts ?? [];
		state.sessions.set(event.sessionId, {
			...session,
			artifacts: [
				...existingArtifacts,
				{
					id: event.artifactId,
					sessionId: event.sessionId,
					artifactType: event.artifactType as 'Docx' | 'Pptx' | 'Pdf',
					filename: event.filename,
					sizeBytes: event.sizeBytes,
					createdAt: new Date().toISOString()
				}
			]
		});
	}
}

