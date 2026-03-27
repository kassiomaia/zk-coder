/**
 * Session Events
 * Handles session-related agent events.
 */

import type {AgentEvent} from '$lib/types/agent';
import type {AgentState} from '../state';
import {getOnAgentActivity, getOnCleanupTransients, getOnExecutionComplete} from '../callbacks';

/**
 * Handle session_state_changed event.
 */
export function handleSessionStateChanged(state: AgentState, event: Extract<AgentEvent, { type: 'session_state_changed' }>) {
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
export function handleExecutionStarted(state: AgentState, event: Extract<AgentEvent, { type: 'execution_started' }>) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		state.sessions.set(event.sessionId, {
			...session,
			state: { type: 'running', currentStep: 0, contextUsage: 0 },
			executionSteps: [],
			toolCalls: [],
			terminalOutput: [],
			statusPhrases: [],
			streamingAssistantContent: undefined,
			artifacts: undefined // Clear any artifacts from previous execution
		});

		// Create thinking message in chat with consistent ID for updates
		const onAgentActivity = getOnAgentActivity();
		if (onAgentActivity) {
			onAgentActivity(session.conversationId, session.threadId, {
				id: `thinking-${event.sessionId}`,
				conversationId: session.conversationId,
				threadId: session.threadId,
				role: 'assistant',
				kind: 'thinking',
				content: 'Thinking...',
				timestamp: new Date()
			});
		}
	}
}

/**
 * Handle execution_step event.
 */
export function handleExecutionStep(state: AgentState, event: Extract<AgentEvent, { type: 'execution_step' }>) {
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
export function handleExecutionComplete(state: AgentState, event: Extract<AgentEvent, { type: 'execution_complete' }>) {
	// Call the callback with conversation/thread IDs from the event
	const onExecutionComplete = getOnExecutionComplete();
	if (onExecutionComplete) {
		const session = state.sessions.get(event.sessionId);
		const artifacts = session?.artifacts ?? [];
		onExecutionComplete(event.sessionId, event.conversationId, event.threadId, event.result.content, artifacts);

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
export function handleExecutionError(state: AgentState, event: Extract<AgentEvent, { type: 'execution_error' }>) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		state.sessions.set(event.sessionId, {
			...session,
			state: { type: 'error', message: event.error, recoverable: false }
		});
	}
}

/**
 * Handle approval events (required, approved, denied).
 */
export function handleApprovalRequired(state: AgentState, event: Extract<AgentEvent, { type: 'approval_required' }>) {
	state.pendingActions = [...state.pendingActions, event.action];
}

export function handleApprovalApproved(state: AgentState, event: Extract<AgentEvent, { type: 'approval_approved' }>) {
	state.pendingActions = state.pendingActions.filter((a) => a.id !== event.actionId);
}

export function handleApprovalDenied(state: AgentState, event: Extract<AgentEvent, { type: 'approval_denied' }>) {
	state.pendingActions = state.pendingActions.filter((a) => a.id !== event.actionId);
}

/**
 * Handle artifact_created event.
 * Stores the artifact reference in the session for later attachment to the final message.
 */
export function handleArtifactCreated(state: AgentState, event: Extract<AgentEvent, { type: 'artifact_created' }>) {
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
