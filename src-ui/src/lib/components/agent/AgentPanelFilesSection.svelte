<script lang="ts">
	import { FileText, RefreshCw } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import CollapsibleSection from '$lib/components/shared/CollapsibleSection.svelte';

	interface Props {
		expanded: boolean;
		onToggle: () => void;
		touchedCount: number;
		onRefresh?: () => void;
		children: Snippet;
	}

	let { expanded, onToggle, touchedCount, onRefresh, children }: Props = $props();

	// With lazy-loaded folders there is no reliable "total workspace
	// file count" to show against the touched count, so the badge
	// surfaces only the touched count when non-zero.
	let badges = $derived(touchedCount > 0 ? [{ label: touchedCount.toString() }] : []);

	let spinning = $state(false);

	function handleRefresh(e: MouseEvent) {
		e.stopPropagation();
		if (!onRefresh) return;
		spinning = true;
		window.setTimeout(() => (spinning = false), 600);
		onRefresh();
	}
</script>

<CollapsibleSection
	{expanded}
	{onToggle}
	label="Files"
	sectionId="files-section"
	animationDelay="0.2s"
	{badges}
>
	{#snippet icon()}
		<FileText size={13} strokeWidth={1.5} class="section-icon" />
	{/snippet}

	{#snippet headerAction()}
		{#if onRefresh}
			<button
				type="button"
				class="refresh-btn"
				class:is-spinning={spinning}
				aria-label="Refresh file tree"
				title="Refresh file tree"
				onclick={handleRefresh}
			>
				<RefreshCw size={13} strokeWidth={1.75} />
			</button>
		{/if}
	{/snippet}

	{@render children()}
</CollapsibleSection>

<style>
	:global(.section-icon) {
		opacity: 0.4;
		transition: all var(--transition-fast);
	}

	.refresh-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		padding: 0;
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		cursor: pointer;
		transition:
			color var(--transition-fast),
			background var(--transition-fast),
			border-color var(--transition-fast);
	}

	.refresh-btn:hover {
		color: var(--color-text-primary);
		background: var(--color-interactive-active);
		border-color: var(--color-border-primary);
	}

	.refresh-btn:active {
		transform: translateY(1px);
	}

	.refresh-btn:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: 1px;
	}

	.refresh-btn.is-spinning :global(svg) {
		animation: refresh-spin 600ms var(--ease-out, ease-out);
	}

	@keyframes refresh-spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}
</style>
