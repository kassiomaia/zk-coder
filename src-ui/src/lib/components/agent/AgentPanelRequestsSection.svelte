<script lang="ts">
	import { ChevronRight, Cpu } from 'lucide-svelte';
	import type { ExecutionStep } from '$lib/types/agent';
	import type { Snippet } from 'svelte';

	interface Props {
		expanded: boolean;
		onToggle: () => void;
		steps: ExecutionStep[];
		currentStep?: number;
		children: Snippet;
	}

	let { expanded, onToggle, steps, currentStep, children }: Props = $props();

	function getStepActionLabel(step: ExecutionStep): string {
		if (step.action.type === 'llm_request') return `LLM: ${step.action.model}`;
		if (step.action.type === 'tool_call') return `Tool: ${step.action.tool}`;
		return 'Unknown';
	}
</script>

<section class="panel-section requests-section" style="animation-delay: 0.1s">
	<button
		class="section-header"
		onclick={onToggle}
		type="button"
		aria-expanded={expanded}
		aria-controls="requests-section"
	>
		<span class="header-main">
			<span class="chevron-icon" class:is-expanded={expanded}>
				<ChevronRight size={14} strokeWidth={2} />
			</span>
			<span class="header-label">Requests</span>
		</span>

		<div class="header-meta">
			{#if steps.length > 0}
				<span class="badge">{steps.length}</span>
			{/if}
			<Cpu size={13} strokeWidth={1.5} class="section-icon" />
		</div>
	</button>
	{#if expanded}
		<div id="requests-section" class="section-content-wrapper">
			{@render children()}
		</div>
	{/if}
</section>

<style>
	.panel-section {
		border-bottom: 1px solid var(--color-border-primary);
		transition: background var(--transition-fast);
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.header-main {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--color-text-primary);
	}

	.chevron-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		transition: transform var(--transition-smooth);
		color: var(--color-text-muted);
	}

	.chevron-icon.is-expanded {
		transform: rotate(90deg);
	}

	.header-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		letter-spacing: 0.05em;
		color: var(--color-text-tertiary);
		text-transform: uppercase;
	}

	.badge {
		padding: var(--space-px) var(--space-1-5);
		background: var(--color-neutral-800);
		border: 1px solid var(--color-border-primary);
		border-radius: var(--radius-sm);
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		line-height: var(--leading-tight);
		white-space: nowrap;
	}

	.header-meta {
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
		color: var(--color-text-muted);
	}

	:global(.section-icon) {
		opacity: 0.4;
		transition: all var(--transition-fast);
	}

	.section-header:hover :global(.section-icon) {
		opacity: 0.8;
		color: var(--color-text-primary);
	}

	.section-header:hover {
		background: var(--color-interactive-hover);
	}

	.section-header:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: -2px;
	}

	.section-content-wrapper {
		overflow: hidden;
	}
</style>
