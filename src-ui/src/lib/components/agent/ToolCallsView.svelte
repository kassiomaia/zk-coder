<script lang="ts">
	/**
	 * ToolCallsView Component
	 * Displays structured tool calls from agent execution.
	 * Auto-scrolls to bottom on new calls.
	 */
	import type { ToolCallRecord } from '$lib/types/agent';
	import { Terminal } from 'lucide-svelte';
	import ToolCallItem from './ToolCallItem.svelte';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';

	interface Props {
		toolCalls: ToolCallRecord[];
	}

	let { toolCalls }: Props = $props();
	let container: HTMLDivElement | undefined = $state();

	/** Auto-scroll to bottom when new tool calls arrive */
	$effect(() => {
		if (toolCalls.length > 0 && container) {
			container.scrollTop = container.scrollHeight;
		}
	});
</script>

<div class="tool-calls-view" bind:this={container} role="log" aria-label="Tool calls history">
	<div class="tool-calls-list">
		{#each toolCalls as record (record.id)}
			<ToolCallItem {record} />
		{:else}
			<EmptyState icon={Terminal} message="No tool calls yet" />
		{/each}
	</div>
</div>

<style>
	.tool-calls-view {
		flex: 1;
		min-height: 120px;
		overflow-y: auto;
		background: var(--color-bg-secondary);
		margin: 0;
		border-top: 1px solid var(--color-border-primary);
		scroll-behavior: smooth;
	}

	.tool-calls-list {
		display: flex;
		flex-direction: column;
		min-height: 100%;
		padding: var(--space-2) 0;
	}

	/* Scrollbar styling */
	.tool-calls-view::-webkit-scrollbar {
		width: var(--space-1-5);
	}

	.tool-calls-view::-webkit-scrollbar-track {
		background: transparent;
	}

	.tool-calls-view::-webkit-scrollbar-thumb {
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
	}
</style>
