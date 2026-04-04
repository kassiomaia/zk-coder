<script lang="ts">
	/**
	 * CollapsibleSection Component
	 * Collapsible panel section with header, badges, icon, and expandable content.
	 * Used in AgentPanel for Requests, Files, and Activity sections.
	 */
	import { ChevronRight } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	interface Badge {
		label: string;
		variant?: 'default' | 'running' | 'tokens';
		title?: string;
	}

	interface Props {
		/** Whether the section is expanded */
		expanded: boolean;
		/** Toggle handler */
		onToggle: () => void;
		/** Section label */
		label: string;
		/** Icon to display */
		icon: Snippet;
		/** Optional badges to display */
		badges?: Badge[];
		/** Unique ID for aria-controls */
		sectionId: string;
		/** Animation delay (CSS var) */
		animationDelay?: string;
		/** Additional CSS class */
		class?: string;
		/** Content to render when expanded */
		children: Snippet;
	}

	let {
		expanded,
		onToggle,
		label,
		icon,
		badges = [],
		sectionId,
		animationDelay = '0s',
		class: className = '',
		children
	}: Props = $props();
</script>

<section class="collapsible-section {className}" style="animation-delay: {animationDelay}">
	<button
		class="section-header"
		onclick={onToggle}
		type="button"
		aria-expanded={expanded}
		aria-controls={sectionId}
	>
		<span class="header-main">
			<span class="chevron-icon" class:is-expanded={expanded}>
				<ChevronRight size={14} strokeWidth={2} />
			</span>
			<span class="header-label">{label}</span>
		</span>

		<div class="header-meta">
			{#each badges as badge}
				<span
					class="badge"
					class:running={badge.variant === 'running'}
					class:tokens={badge.variant === 'tokens'}
					title={badge.title}
				>
					{badge.label}
				</span>
			{/each}
			{@render icon()}
		</div>
	</button>
	{#if expanded}
		<div id={sectionId} class="section-content-wrapper">
			{@render children()}
		</div>
	{/if}
</section>

<style>
	.collapsible-section {
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
