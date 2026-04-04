/**
 * Task Events
 * Handles task and subtask events.
 */

import type { AgentEvent } from '$lib/types/agent';
import { toAgentTask } from '$lib/types/agent';
import type { AgentState } from '../state';

/**
 * Handle task_created event.
 */
export function handleTaskCreated(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'task_created' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		const newTask = toAgentTask(event.task);
		const existingTasks = session.tasks ?? [];
		state.sessions.set(event.sessionId, {
			...session,
			tasks: [...existingTasks, newTask]
		});
	}
}

/**
 * Handle task_plan_generated event.
 * Updates the task with the LLM-generated title and description.
 */
export function handleTaskPlanGenerated(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'task_plan_generated' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session && session.tasks) {
		const updatedTasks = session.tasks.map((task) =>
			task.id === event.taskId
				? {
						...task,
						title: event.title,
						description: event.description,
						isNonTrivial: event.isNonTrivial,
						subtasks: event.subtasks ?? task.subtasks
					}
				: task
		);
		state.sessions.set(event.sessionId, {
			...session,
			tasks: updatedTasks
		});
	}
}

/**
 * Handle task_updated event.
 */
export function handleTaskUpdated(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'task_updated' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session && session.tasks) {
		const updatedTasks = session.tasks.map((task) =>
			task.id === event.taskId ? { ...task, status: event.status } : task
		);
		state.sessions.set(event.sessionId, {
			...session,
			tasks: updatedTasks
		});
	}
}

/**
 * Handle subtask_created event.
 */
export function handleSubtaskCreated(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'subtask_created' }>
) {
	handleSubtaskUpdate(state, event);
}

/**
 * Handle subtask_updated event.
 */
export function handleSubtaskUpdated(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'subtask_updated' }>
) {
	handleSubtaskUpdate(state, event);
}

/**
 * Common handler for subtask events.
 */
function handleSubtaskUpdate(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'subtask_created' | 'subtask_updated' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		const tasks = session.tasks ?? [];
		// Find the task that owns this subtask and add/update
		const updatedTasks = tasks.map((task) => {
			if (task.id === event.subtask.taskId) {
				const existingSubtasks = task.subtasks ?? [];
				const subtaskIndex = existingSubtasks.findIndex((st) => st.id === event.subtask.id);
				if (subtaskIndex >= 0) {
					const newSubtasks = [...existingSubtasks];
					newSubtasks[subtaskIndex] = event.subtask;
					return { ...task, subtasks: newSubtasks };
				} else {
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
}
