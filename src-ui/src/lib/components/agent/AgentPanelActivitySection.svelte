<script lang="ts">
	import { Activity } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import CollapsibleSection from '$lib/components/shared/CollapsibleSection.svelte';

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

	let badges = $derived(() => {
		const result = [];
		if (currentStep) {
			result.push({ label: `Step ${currentStep}`, variant: 'running' as const });
		}
		if (totalTokensUsed > 0) {
			result.push({
				label: formatTokenCount(totalTokensUsed),
				variant: 'tokens' as const,
				title: 'Total tokens used'
			});
		}
		return result;
	});
</script>

<CollapsibleSection
	{expanded}
	{onToggle}
	label="Activity"
	sectionId="activity-section"
	animationDelay="0s"
	badges={badges()}
>
	{#snippet icon()}
		<Activity size={13} strokeWidth={1.5} class="section-icon" />
	{/snippet}

	{@render children()}
</CollapsibleSection>

<style>
	:global(.section-icon) {
		opacity: 0.4;
		transition: all var(--transition-fast);
	}
</style>
