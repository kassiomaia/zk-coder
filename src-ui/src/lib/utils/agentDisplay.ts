import type { AgentSession } from '$lib/types/agent';

// Built-in agents whose persisted `agentName` may be the raw agent ID
// (legacy sessions created before the backend started resolving the
// display name from the registry). Centralizing the mapping keeps the
// rename in one spot and avoids leaking IDs into the UI.
const BUILT_IN_AGENT_NAMES: Record<string, string> = {
	'code-agent': 'Agent'
};

/**
 * Resolve the human-facing display name for an agent session.
 *
 * Prefers the backend-supplied `agentName`, but falls back to the
 * built-in mapping when the persisted value is a raw agent ID (true for
 * sessions created before the backend registry lookup was introduced).
 */
export function displayAgentName(session: Pick<AgentSession, 'agentId' | 'agentName'>): string {
	const name = session.agentName?.trim();
	if (name && name !== session.agentId) {
		return name;
	}
	return BUILT_IN_AGENT_NAMES[session.agentId] ?? name ?? 'Agent';
}
