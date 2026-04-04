<script lang="ts">
	/**
	 * TaskList Component
	 * Displays agent tasks with status indicators and subtasks.
	 */
	import { CheckCircle, ChevronRight, Circle, Loader2, ListTodo, XCircle } from 'lucide-svelte';
	import type { AgentTask, SubTask, TaskStatus } from '$lib/types/agent';
	import { SvelteSet } from 'svelte/reactivity';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';

	interface Props {
		tasks: AgentTask[];
		ontaskclick?: (task: AgentTask) => void;
		onsubtaskclick?: (subtask: SubTask) => void;
	}

	let { tasks, ontaskclick, onsubtaskclick }: Props = $props();

	/** Track expanded state for tasks with subtasks */
	let expandedTasks = new SvelteSet<string>();

	function toggleTask(taskId: string) {
		if (expandedTasks.has(taskId)) {
			expandedTasks.delete(taskId);
		} else {
			expandedTasks.add(taskId);
		}
	}

	function getStatusClass(status: TaskStatus): string {
		switch (status) {
			case 'completed':
				return 'completed';
			case 'in_progress':
				return 'running';
			case 'failed':
				return 'failed';
			default:
				return '';
		}
	}
</script>

<div class="task-list" aria-label="Task list">
	{#each tasks as task (task.id)}
		{@const hasSubtasks = task.subtasks && task.subtasks.length > 0}
		{@const isExpanded = expandedTasks.has(task.id) || task.status === 'in_progress'}
		{@const isRunning = task.status === 'in_progress'}

		<div class="task-wrapper" class:is-running={isRunning}>
			<div
				class="task-item"
				class:selected={task.selected}
				class:clickable={ontaskclick !== undefined}
				role="button"
				tabindex={ontaskclick ? 0 : -1}
				aria-label="{task.title} - {task.status}"
				onclick={() => {
					if (hasSubtasks) toggleTask(task.id);
					ontaskclick?.(task);
				}}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault();
						if (hasSubtasks) toggleTask(task.id);
						ontaskclick?.(task);
					}
				}}
			>
				<span class="status-icon {getStatusClass(task.status)}">
					{#if task.status === 'in_progress'}
						<Loader2 size={14} strokeWidth={2} class="spinning" />
					{:else if task.status === 'completed'}
						<CheckCircle size={14} strokeWidth={2} />
					{:else if task.status === 'failed'}
						<XCircle size={14} strokeWidth={2} />
					{:else}
						<Circle size={14} strokeWidth={2} />
					{/if}
				</span>

				<span class="task-content">
					<span class="task-title">{task.title}</span>
					{#if task.description}
						<span class="task-description">{task.description}</span>
					{/if}
				</span>

				{#if hasSubtasks}
					<span class="expand-icon" class:is-expanded={isExpanded}>
						<ChevronRight size={14} strokeWidth={2} />
					</span>
				{/if}
			</div>

			<!-- Subtasks -->
			{#if hasSubtasks && isExpanded}
				<div class="subtasks">
					{#each task.subtasks as subtask (subtask.id)}
						<button class="subtask-item" onclick={() => onsubtaskclick?.(subtask)} type="button">
							<span class="status-icon {getStatusClass(subtask.status)}">
								{#if subtask.status === 'in_progress'}
									<Loader2 size={12} strokeWidth={2} class="spinning" />
								{:else if subtask.status === 'completed'}
									<CheckCircle size={12} strokeWidth={2} />
								{:else if subtask.status === 'failed'}
									<XCircle size={12} strokeWidth={2} />
								{:else}
									<Circle size={12} strokeWidth={2} />
								{/if}
							</span>
							<span class="subtask-content">
								<span class="subtask-title">{subtask.title}</span>
							</span>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{:else}
		<EmptyState icon={ListTodo} message="Ready to start" />
	{/each}
</div>

<style>
	.task-list {
		display: flex;
		flex-direction: column;
		padding: var(--space-2) 0;
	}

	.task-wrapper {
		display: flex;
		flex-direction: column;
		border-left: 2px solid transparent;
		transition: all var(--transition-fast);
		position: relative;
	}

	.task-item {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		text-align: left;
		width: 100%;
		background: transparent;
		cursor: pointer;
		border: none;
		transition: background var(--transition-fast);
	}

	.task-item:hover {
		background: var(--color-interactive-hover);
	}

	.task-item.selected {
		background: var(--color-interactive-selected);
	}

	.status-icon {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		margin-top: var(--space-0-5);
		transition: color var(--transition-fast);
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

	.status-icon :global(.spinning) {
		animation: spin 1.5s linear infinite;
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
		min-width: 0;
		flex: 1;
		gap: var(--space-0-5);
	}

	.task-title {
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-primary);
		line-height: var(--leading-snug);
	}

	.task-description {
		font-size: var(--text-2xs);
		color: var(--color-text-tertiary);
		white-space: normal;
		line-height: var(--leading-snug);
	}

	.expand-icon {
		margin-left: auto;
		color: var(--color-text-muted);
		transition: transform var(--transition-smooth);
		opacity: 0.5;
	}

	.expand-icon.is-expanded {
		transform: rotate(90deg);
		opacity: 0.8;
	}

	.subtasks {
		display: flex;
		flex-direction: column;
		gap: var(--space-px);
		margin-left: var(--space-7);
		padding: var(--space-1) 0 var(--space-2) var(--space-2);
		border-left: 1px solid var(--color-border-primary);
	}

	.subtask-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1-5) var(--space-3);
		border-radius: var(--radius-sm);
		text-align: left;
		width: 100%;
		background: transparent;
		border: none;
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.subtask-item:hover {
		background: var(--color-interactive-hover);
		padding-left: var(--space-4);
	}

	.subtask-title {
		font-size: var(--text-2xs);
		color: var(--color-text-secondary);
		font-weight: var(--font-normal);
	}

	/* Removed running-indicator shimmer animation */
</style>
