<script lang="ts">
	/**
	 * TaskList Component
	 * Displays agent tasks with status indicators and subtasks.
	 */
	import { CheckCircle, Circle, Loader2, XCircle, ChevronRight, ChevronDown, Activity } from 'lucide-svelte';
	import type { AgentTask, TaskStatus, SubTask, ExecutionStep } from '$lib/types/agent';

	interface Props {
		tasks: AgentTask[];
		executionSteps?: ExecutionStep[];
		currentStep?: number;
		ontaskclick?: (task: AgentTask) => void;
		onsubtaskclick?: (subtask: SubTask) => void;
	}

	let { tasks, executionSteps = [], currentStep, ontaskclick, onsubtaskclick }: Props = $props();

	function getStepActionLabel(step: ExecutionStep): string {
		if (step.action.type === 'llm_request') return `LLM: ${step.action.model}`;
		if (step.action.type === 'tool_call') return `Tool: ${step.action.tool}`;
		return 'Step';
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
				onclick={() => ontaskclick?.(task)}
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
			</div>

			{#if isRunning && executionSteps.length > 0}
				<div class="live-steps">
					<div class="steps-connector"></div>
					{#each executionSteps as step}
						<div class="step-mini-item" class:is-current={step.stepNumber === currentStep}>
							<Activity size={10} />
							<span class="step-label">{getStepActionLabel(step)}</span>
							{#if step.result}
								<span class="step-time">{step.result.durationMs}ms</span>
							{/if}
						</div>
					{/each}
				</div>
			{/if}

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
		gap: 0;
		padding: var(--space-1) 0;
	}

	.task-wrapper {
		display: flex;
		flex-direction: column;
		border-left: 2px solid transparent;
		transition: all var(--transition-fast);
	}

	.task-wrapper.is-running {
		background: var(--color-primary-500/5);
		border-left-color: var(--color-primary-500);
	}

	.task-item {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		text-align: left;
		width: 100%;
		background: transparent;
	}

	.task-item.clickable {
		cursor: pointer;
	}

	.task-item.clickable:hover {
		background: var(--color-interactive-hover);
	}

	.live-steps {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin-left: calc(var(--space-4) + 7px);
		padding: var(--space-1) 0 var(--space-3) var(--space-4);
		border-left: 1px solid var(--color-border-subtle);
	}

	.step-mini-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-2);
		font-size: 10px;
		color: var(--color-text-muted);
		border-radius: var(--radius-sm);
		transition: all var(--transition-fast);
	}

	.step-mini-item.is-current {
		color: var(--color-primary-400);
		background: var(--color-primary-500/10);
	}

	.step-label {
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.step-time {
		font-size: 9px;
		opacity: 0.6;
	}

	.task-title {
		font-size: var(--text-xs);
		font-weight: 500;
		color: var(--color-text-primary);
		line-height: 1.4;
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
	}

	.task-item.clickable {
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.task-item.clickable:hover {
		background: var(--color-interactive-hover);
	}

	.task-item.clickable:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: -2px;
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
		padding: var(--space-4) var(--space-4) var(--space-6);
		text-align: left;
		color: var(--color-text-muted);
		font-size: 11px;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.empty-state p {
		margin: 0;
		opacity: 0.7;
		font-style: italic;
	}

	.empty-state::after {
		content: 'Waiting for instructions to begin execution.';
		font-size: 10px;
		opacity: 0.4;
	}
</style>
