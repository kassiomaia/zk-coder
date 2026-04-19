import type { AgentSession } from '$lib/types/agent';
import type { Thread } from '$lib/types/thread';
import type { AgentStatus } from '$lib/utils/agentStatus';
import { getAgentStatus } from '$lib/utils/agentStatus';
import { getStatusPhrases } from '$lib/stores/agent/statusStore';

export interface ThreadAgentState {
	session: AgentSession | null;
	status: AgentStatus | null;
	isWorking: boolean;
	sessionId: string | null;
}

export function getThreadAgentState(
	thread: Thread | undefined,
	sessions: AgentSession[]
): ThreadAgentState {
	const sessionId = thread?.metadata.agentSessionId ?? null;
	if (!sessionId) {
		return {
			session: null,
			status: null,
			isWorking: false,
			sessionId: null
		};
	}

	const session = sessions.find((candidate) => candidate.id === sessionId) ?? null;
	if (!session) {
		return {
			session: null,
			status: null,
			isWorking: false,
			sessionId
		};
	}

	const latestStep = session.executionSteps?.[session.executionSteps.length - 1] ?? null;
	const isTerminalOrPaused =
		session.state.type === 'finished' ||
		session.state.type === 'cancelled' ||
		session.state.type === 'error' ||
		session.state.type === 'paused';
	const currentSubtask = isTerminalOrPaused
		? null
		: session.tasks
				?.flatMap((task) => task.subtasks ?? [])
				.find((subtask) => subtask.status === 'in_progress') ?? null;
	const contextUsage = session.state.type === 'running' ? session.state.contextUsage : 0;
	const lastActivityAt = session.updatedAt ? new Date(session.updatedAt) : null;
	const hasPendingApproval = session.state.type === 'user_input_required';
	// Phrases live in the side-channel `agentStatusPhrases` store
	// rather than on the session, so high-frequency StatusPhrase events
	// don't cascade through every session-store subscriber.
	const statusPhrases = getStatusPhrases(session.id);
	const status = getAgentStatus(
		session.state,
		latestStep,
		currentSubtask,
		hasPendingApproval,
		statusPhrases,
		contextUsage,
		lastActivityAt,
		session.harnessState
	);

	return {
		session,
		status,
		isWorking: status.active,
		sessionId
	};
}
