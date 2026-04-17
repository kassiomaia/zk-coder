/**
 * Harness Events
 *
 * Dedicated mapping layer for long-running harness transitions (startup
 * verification, handoff summary, recovery prompts). These mappers are kept
 * separate from sessionEvents.ts so the core session-state mapping stays
 * small and so harness concerns stay close to the typed harness contract.
 *
 * The session store is the authoritative source for durable harness state;
 * chat-side activity is a lightweight follow-up, not the primary rendering
 * surface (the agent panel's summary section owns that).
 */

import type { AgentEvent } from '$lib/types/agent';
import type { AgentState } from '../state';
import { getOnAgentActivity } from '../callbacks';

/**
 * Handle harness_state_updated event.
 * Updates the session's typed harness metadata (mode / startup verification /
 * handoff / clean exit). No chat side-effect — the panel derives its summary
 * section directly from this state.
 */
export function handleHarnessStateUpdated(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'harness_state_updated' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (!session) return;

	state.sessions.set(event.sessionId, {
		...session,
		harnessState: event.harnessState
	});
}

/**
 * Handle session_paused event.
 * Panel-backed state is updated first: the session transitions to `paused`
 * with the pause metadata. The chat recovery card is a supplementary action
 * surface; it is not the source of truth for session state.
 */
export function handleSessionPaused(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'session_paused' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (!session) return;

	state.sessions.set(event.sessionId, {
		...session,
		state: {
			type: 'paused',
			pausedAt: event.pausedAt,
			message: event.message,
			currentSubtaskId: event.currentSubtaskId
		}
	});

	const onAgentActivity = getOnAgentActivity();
	if (onAgentActivity) {
		onAgentActivity(session.conversationId, session.threadId, {
			id: `recovery-${event.sessionId}`,
			conversationId: session.conversationId,
			threadId: session.threadId,
			role: 'assistant',
			kind: 'recovery',
			content: event.message,
			timestamp: new Date(event.pausedAt),
			agentData: {
				sessionId: event.sessionId,
				recoveryResolved: false
			}
		});
	}
}

/**
 * Handle recovery_continue event.
 * Marks the chat recovery card as resolved. The session's lifecycle transition
 * back to `running` is driven by the subsequent `session_state_changed` event,
 * so this mapper is intentionally scoped to the chat-side acknowledgement.
 */
export function handleRecoveryContinue(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'recovery_continue' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (!session) return;

	const onAgentActivity = getOnAgentActivity();
	if (onAgentActivity) {
		onAgentActivity(session.conversationId, session.threadId, {
			id: `recovery-${event.sessionId}`,
			agentData: {
				sessionId: event.sessionId,
				recoveryResolved: true,
				recoveryDecision: 'continued'
			}
		});
	}
}

/**
 * Handle recovery_replan event.
 * Marks the chat recovery card as resolved. Same reasoning as
 * handleRecoveryContinue — lifecycle state updates come from
 * session_state_changed, not from this event.
 */
export function handleRecoveryReplan(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'recovery_replan' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (!session) return;

	const onAgentActivity = getOnAgentActivity();
	if (onAgentActivity) {
		onAgentActivity(session.conversationId, session.threadId, {
			id: `recovery-${event.sessionId}`,
			agentData: {
				sessionId: event.sessionId,
				recoveryResolved: true,
				recoveryDecision: 'replanned'
			}
		});
	}
}
