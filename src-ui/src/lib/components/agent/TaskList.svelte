<script lang="ts">
	/**
	 * TaskList Component
	 * Displays agent tasks with status indicators and subtasks.
	 */
	import { CheckCircle, Circle, Loader2, XCircle, ChevronRight, ChevronDown } from 'lucide-svelte';
	import type { AgentTask, TaskStatus, SubTask } from '$lib/types/agent';

	interface Props {
		tasks: AgentTask[];
		ontaskclick?: (task: AgentTask) => void;
		onsubtaskclick?: (subtask: SubTask) => void;
	}

	let { tasks, ontaskclick, onsubtaskclick }: Props = $props();

	/** Track which tasks are expanded to show subtasks */
	let expandedTasks = $state<Set<string>>(new Set());

	function getStatusClass(status: TaskStatus): string {
		switch (status) {
			case 'completed':
				return 'completed';
			case 'in_progress':
				return 'running';
			case 'failed':
				return 'failed';
			default:
				return 'pending';
		}
	}

	function toggleTask(taskId: string) {
		if (expandedTasks.has(taskId)) {
			expandedTasks.delete(taskId);
		} else {
			expandedTasks.add(taskId);
		}
		expandedTasks = new Set(expandedTasks);
	}
</script>

<div class="task-list" aria-label="Task list">
	{#each tasks as task (task.id)}
		{@const hasSubtasks = task.subtasks && task.subtasks.length > 0}
		{@const isExpanded = expandedTasks.has(task.id)}
		<div class="task-wrapper">
			<button
				class="task-item"
				class:selected={task.selected}
				onclick={() => ontaskclick?.(task)}
				type="button"
				aria-label="{task.title} - {task.status}"
			>
				<span class="status-icon {getStatusClass(task.status)}">
					{#if task.status === 'in_progress'}
						<Loader2 size={14} class="spinning" />
					{:else if task.status === 'completed'}
						<CheckCircle size={14} />
					{:else if task.status === 'failed'}
						<XCircle size={14} />
					{:else}
						<Circle size={14} />
					{/if}
				</span>
				<span class="task-content">
					<span class="task-title">{task.title}</span>
					{#if task.description}
						<span class="task-description">{task.description}</span>
					{/if}
				</span>
				{#if hasSubtasks}
					<button
						class="expand-toggle"
						class:expanded={isExpanded}
						onclick={(e) => {
							e.stopPropagation();
							toggleTask(task.id);
						}}
						type="button"
						aria-label="Toggle subtasks"
					>
						{#if isExpanded}
							<ChevronDown size={12} />
						{:else}
							<ChevronRight size={12} />
						{/if}
					</button>
				{/if}
			</button>
			{#if hasSubtasks && isExpanded}
				<div class="subtasks">
					{#each task.subtasks as subtask (subtask.id)}
						<button
							class="subtask-item"
							onclick={() => onsubtaskclick?.(subtask)}
							type="button"
							aria-label="{subtask.title} - {subtask.status}"
						>
							<span class="status-icon {getStatusClass(subtask.status)}">
								{#if subtask.status === 'in_progress'}
									<Loader2 size={12} class="spinning" />
								{:else if subtask.status === 'completed'}
									<CheckCircle size={12} />
								{:else if subtask.status === 'failed'}
									<XCircle size={12} />
								{:else}
									<Circle size={12} />
								{/if}
							</span>
							<span class="subtask-content">
								<span class="subtask-title">{subtask.title}</span>
								{#if subtask.description}
									<span class="subtask-description">{subtask.description}</span>
								{/if}
							</span>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{:else}
		<div class="empty-state">
			<p>No tasks yet</p>
		</div>
	{/each}
</div>

<style>
	.task-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-2) var(--space-4);
	}

	.task-wrapper {
		display: flex;
		flex-direction: column;
	}

	.task-item {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
		padding: var(--space-2);
		border-radius: var(--radius-md);
		text-align: left;
		width: 100%;
		background: transparent;
		border: none;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.task-item:hover {
		background: var(--color-interactive-hover);
	}

	.task-item.selected {
		background: var(--color-interactive-selected);
	}

	.task-item:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: -2px;
	}

	.status-icon {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		margin-top: 1px;
	}

	.status-icon.completed {
		color: var(--color-success);
	}

	.status-icon.running {
		color: var(--color-primary-400);
	}

	.status-icon.failed {
		color: var(--color-error);
	}

	.status-icon.running :global(.spinning) {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	.task-content {
		display: flex;
		flex-direction: column;
		gap: var(--space-0-5);
		min-width: 0;
		flex: 1;
	}

	.task-title {
		font-size: var(--text-sm);
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.task-description {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.expand-toggle {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-1);
		margin-left: auto;
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: color var(--transition-fast);
	}

	.expand-toggle:hover {
		color: var(--color-text-primary);
	}

	.expand-toggle:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: -2px;
		border-radius: var(--radius-sm);
	}

	.subtasks {
		display: flex;
		flex-direction: column;
		gap: var(--space-0-5);
		margin-left: var(--space-6);
		padding-left: var(--space-3);
		border-left: 1px solid var(--color-border-subtle);
	}

	.subtask-item {
		display: flex;
		align-items: flex-start;
		gap: var(--space-1-5);
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-sm);
		text-align: left;
		width: 100%;
		background: transparent;
		border: none;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.subtask-item:hover {
		background: var(--color-interactive-hover);
	}

	.subtask-item:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: -2px;
	}

	.subtask-content {
		display: flex;
		flex-direction: column;
		gap: var(--space-0-5);
		min-width: 0;
		flex: 1;
	}

	.subtask-title {
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
	}

	.subtask-description {
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
	}

	.empty-state {
		padding: var(--space-4);
		text-align: center;
		color: var(--color-text-muted);
		font-size: var(--text-xs);
	}
</style>
