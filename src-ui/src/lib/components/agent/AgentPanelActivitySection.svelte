<script lang="ts">
	import { Activity, ChevronRight } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		expanded: boolean;
		onToggle: () => void;
		currentStep?: number;
		totalTokensUsed: number;
		children: Snippet;
	}

	let { expanded, onToggle, currentStep, totalTokensUsed, children }: Props = $props();

	function formatTokenCount(tokens: number): string {
		if (tokens >= 1000) {
			return `${(tokens / 1000).toFixed(1)}k`;
		}
		return tokens.toString();
	}
</script>

<section class="panel-section activity-section" style="animation-delay: 0s">
	<button
		class="section-header"
		onclick={onToggle}
		type="button"
		aria-expanded={expanded}
		aria-controls="activity-section"
	>
		<span class="header-main">
			<span class="chevron-icon" class:is-expanded={expanded}>
				<ChevronRight size={14} strokeWidth={2} />
			</span>
			<span class="header-label">Activity</span>
		</span>

		<div class="header-meta">
			{#if currentStep}
				<span class="badge running">Step {currentStep}</span>
			{/if}
			{#if totalTokensUsed > 0}
				<span class="badge tokens" title="Total tokens used"
					>{formatTokenCount(totalTokensUsed)}</span
				>
			{/if}
			<Activity size={13} strokeWidth={1.5} class="section-icon" />
		</div>
	</button>
	{#if expanded}
		<div id="activity-section" class="section-content-wrapper">
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

	.badge.running {
		background: var(--color-primary-subtle);
		border-color: var(--color-primary-border);
		color: var(--color-primary-400);
	}

	.badge.tokens {
		background: var(--color-neutral-900);
		border-color: var(--color-border-primary);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
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
