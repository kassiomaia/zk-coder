<script lang="ts">
	import { FileText } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import CollapsibleSection from '$lib/components/shared/CollapsibleSection.svelte';

	interface Props {
		expanded: boolean;
		onToggle: () => void;
		fileCount: number;
		children: Snippet;
	}

	let { expanded, onToggle, fileCount, children }: Props = $props();

	let badges = $derived(fileCount > 0 ? [{ label: fileCount.toString() }] : []);
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

	{@render children()}
</CollapsibleSection>

<style>
	:global(.section-icon) {
		opacity: 0.4;
		transition: all var(--transition-fast);
	}
</style>
