/**
 * Agent Actions
 * Agent listing and query functions for the agent store.
 */

import type { AgentState } from '../state';
import type { Agent } from '$lib/types/agent';

/**
 * List all available agents.
 */
export async function listAgents(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void
): Promise<Agent[]> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		const agents = await invoke<Agent[]>('list_agents');
		return agents;
	} catch (error) {
		update((s) => ({ ...s, error: String(error) }));
		return [];
	}
}

/**
 * Get a specific agent by ID.
 */
export async function getAgent(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
	agentId: string
): Promise<Agent | null> {
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		const agent = await invoke<Agent>('get_agent', {
			agentId
		});
		return agent;
	} catch (error) {
		update((s) => ({ ...s, error: String(error) }));
		return null;
	}
}
