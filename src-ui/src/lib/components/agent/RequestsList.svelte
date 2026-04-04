<script lang="ts">
	/**
	 * RequestsList Component
	 * Displays recent execution steps (LLM requests and tool calls).
	 */
	import type { ExecutionStep } from '$lib/types/agent';
	import { Activity, Cpu } from 'lucide-svelte';

	interface Props {
		steps: ExecutionStep[];
		currentStep?: number;
		maxRecent?: number;
	}

	let { steps, currentStep, maxRecent = 10 }: Props = $props();

	// Show only recent N steps
	let recentSteps = $derived(steps.slice(-maxRecent));

	// Create a unique key for each step (stepNumber alone may have duplicates across sessions)
	function getStepKey(step: ExecutionStep, index: number): string {
		return `${step.stepNumber}-${step.timestamp}-${index}`;
	}

	function getStepActionLabel(step: ExecutionStep): string {
		if (step.action.type === 'llm_request') {
			return step.action.model;
		}
		if (step.action.type === 'tool_call') {
			return step.action.tool;
		}
		return 'Step';
	}

	function getActionType(step: ExecutionStep): string {
		return step.action.type === 'llm_request' ? 'LLM' : 'Tool';
	}

	function formatDuration(ms: number): string {
		if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
		return `${ms}ms`;
	}
</script>

<section class="requests-list">
	{#each recentSteps as step, index (getStepKey(step, index))}
		<div class="request-item" class:is-current={step.stepNumber === currentStep}>
			<span class="request-type" class:llm={step.action.type === 'llm_request'}>
				{getActionType(step)}
			</span>
			<span class="request-label">{getStepActionLabel(step)}</span>
			{#if step.result}
				<span class="request-duration">{formatDuration(step.result.durationMs)}</span>
			{/if}
		</div>
	{/each}
	{#if recentSteps.length === 0}
		<div class="empty-state">
			<p>No requests recorded</p>
		</div>
	{/if}
</section>

<style>
	.requests-list {
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding: var(--space-2) 0;
	}

	.request-item {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-1-5) var(--space-4);
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
		transition: all var(--transition-fast);
		border-left: 2px solid transparent;
	}

	.request-item:hover {
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
	}

	.request-item.is-current {
		color: var(--color-primary-400);
		background: var(--color-interactive-selected);
		border-left-color: var(--color-primary-400);
	}

	.request-type {
		font-size: var(--text-2xs);
		font-weight: var(--font-semibold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-tertiary);
		width: var(--space-8);
		flex-shrink: 0;
	}

	.request-type.llm {
		color: var(--color-primary-500);
		opacity: 0.8;
	}

	.request-label {
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		opacity: 0.9;
	}

	.request-duration {
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
		opacity: 0.6;
		margin-left: auto;
	}

	.empty-state {
		padding: var(--space-6) var(--space-4);
		color: var(--color-text-muted);
		font-size: var(--text-xs);
		text-align: center;
		opacity: 0.5;
	}

	.empty-state p {
		margin: 0;
		font-style: italic;
	}
</style>
