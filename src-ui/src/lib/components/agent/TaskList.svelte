<script lang="ts">
	/**
	 * TaskList Component
	 * Displays agent tasks with status indicators.
	 */
	import { CheckCircle, Circle, Loader2, XCircle } from 'lucide-svelte';
	import type { AgentTask, TaskStatus } from '$lib/types/agent';

	interface Props {
		tasks: AgentTask[];
		ontaskclick?: (task: AgentTask) => void;
	}

	let { tasks, ontaskclick }: Props = $props();

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
</script>

<div class="task-list" role="list" aria-label="Task list">
	{#each tasks as task (task.id)}
		<button
			class="task-item"
			class:selected={task.selected}
			onclick={() => ontaskclick?.(task)}
			type="button"
			role="listitem"
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
		</button>
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

	.empty-state {
		padding: var(--space-4);
		text-align: center;
		color: var(--color-text-muted);
		font-size: var(--text-xs);
	}
</style>
