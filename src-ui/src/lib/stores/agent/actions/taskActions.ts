/**
 * Task Actions
 * Task and subtask management functions for the agent store.
 */

import type {AgentState} from '../state';

/**
 * Create a new subtask for task decomposition.
 */
export async function createSubtask(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
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
export async function updateSubtaskStatus(
	state: AgentState,
	update: (fn: (state: AgentState) => AgentState) => void,
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
