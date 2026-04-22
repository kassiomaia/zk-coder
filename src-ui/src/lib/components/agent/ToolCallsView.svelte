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

	// Follow new tool calls only while the user is pinned to the bottom. If
	// they have scrolled up to inspect an earlier call, preserve their
	// position — forcing scrollTop on every update made manual scrolling
	// impossible. 24px of slack accounts for sub-pixel rounding.
	let followBottom = true;

	function handleScroll() {
		if (!container) return;
		const distanceFromBottom =
			container.scrollHeight - container.scrollTop - container.clientHeight;
		followBottom = distanceFromBottom <= 24;
	}

	$effect(() => {
		// Re-read the reactive length so the effect re-runs on new calls.
		const count = toolCalls.length;
		if (count > 0 && container && followBottom) {
			// Bypass the container's `scroll-behavior: smooth` for
			// programmatic follow — a smooth animation fires intermediate
			// `scroll` events with distance > 0, which would flip
			// `followBottom` to false mid-animation when another call
			// arrives.
			container.scrollTo({ top: container.scrollHeight, behavior: 'instant' });
		}
	});
</script>

<div
	class="tool-calls-view"
	bind:this={container}
	onscroll={handleScroll}
	role="log"
	aria-label="Tool calls history"
>
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
