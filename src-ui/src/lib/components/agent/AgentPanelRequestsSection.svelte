<script lang="ts">
	import { Cpu } from 'lucide-svelte';
	import type { ExecutionStep } from '$lib/types/agent';
	import type { Snippet } from 'svelte';
	import CollapsibleSection from '$lib/components/shared/CollapsibleSection.svelte';

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

	let badges = $derived(steps.length > 0 ? [{ label: steps.length.toString() }] : []);
</script>

<CollapsibleSection
	{expanded}
	{onToggle}
	label="Requests"
	sectionId="requests-section"
	animationDelay="0.1s"
	{badges}
>
	{#snippet icon()}
		<Cpu size={13} strokeWidth={1.5} class="section-icon" />
	{/snippet}

	{@render children()}
</CollapsibleSection>

<style>
	:global(.section-icon) {
		opacity: 0.4;
		transition: all var(--transition-fast);
	}
</style>
