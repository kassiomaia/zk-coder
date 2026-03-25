<script lang="ts">
	/**
	 * RequestsList Component
	 * Displays recent execution steps (LLM requests and tool calls).
	 */
	import type { ExecutionStep } from '$lib/types/agent';
	import { Cpu, Activity } from 'lucide-svelte';

	interface Props {
		steps: ExecutionStep[];
		currentStep?: number;
		maxRecent?: number;
	}

	let { steps, currentStep, maxRecent = 10 }: Props = $props();

	// Show only recent N steps
	let recentSteps = $derived(steps.slice(-maxRecent));

	function getStepActionLabel(step: ExecutionStep): string {
		if (step.action.type === 'llm_request') {
			return `LLM: ${step.action.model}`;
		}
		if (step.action.type === 'tool_call') {
			return `Tool: ${step.action.tool}`;
		}
		return 'Step';
	}

	function formatDuration(ms: number): string {
		if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
		return `${ms}ms`;
	}
</script>

<section class="requests-list">
	{#each recentSteps as step (step.stepNumber)}
		<div class="request-item" class:is-current={step.stepNumber === currentStep}>
			{#if step.action.type === 'llm_request'}
				<span class="icon llm">
					<Cpu size={12} />
				</span>
			{:else}
				<span class="icon tool">
					<Activity size={12} />
				</span>
			{/if}
			<span class="request-label">{getStepActionLabel(step)}</span>
			{#if step.result}
				<span class="request-duration">{formatDuration(step.result.durationMs)}</span>
			{/if}
		</div>
	{/each}
	{#if recentSteps.length === 0}
		<div class="empty-state">
			<p>No requests yet</p>
		</div>
	{/if}
</section>

<style>
	.requests-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: var(--space-1) 0 var(--space-2) var(--space-2);
	}

	.request-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-0-5) var(--space-2);
		font-size: 10px;
		color: var(--color-text-muted);
		border-radius: var(--radius-sm);
		transition: background var(--transition-fast);
	}

	.request-item:hover {
		background: var(--color-interactive-hover);
	}

	.request-item.is-current {
		color: var(--color-primary-400);
		background: var(--color-primary-500/10);
	}

	.icon {
		display: flex;
		flex-shrink: 0;
		opacity: 0.6;
	}

	.icon.llm {
		color: var(--color-primary-400);
		opacity: 1;
	}

	.request-label {
		flex: 1;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.request-duration {
		font-size: 9px;
		opacity: 0.6;
		font-family: var(--font-mono);
	}

	.empty-state {
		padding: var(--space-4);
		color: var(--color-text-muted);
		font-size: 11px;
		text-align: center;
		font-style: italic;
		opacity: 0.7;
	}
</style>
