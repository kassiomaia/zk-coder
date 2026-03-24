<script lang="ts">
	/**
	 * ToolCallsView Component
	 * Displays structured tool calls from agent execution.
	 * Auto-scrolls to bottom on new calls.
	 */
	import type { ToolCallRecord } from '$lib/types/agent';
	import ToolCallItem from './ToolCallItem.svelte';

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
			<div class="empty-state">
				<div class="empty-icon-wrapper">
					<span class="empty-char">_</span>
				</div>
				<span class="empty-text">No tool calls recorded</span>
			</div>
		{/each}
	</div>
</div>

<style>
	.tool-calls-view {
		flex: 1;
		min-height: 120px;
		overflow-y: auto;
		background: var(--color-neutral-950);
		margin: 0;
		border-top: 1px solid var(--color-border-subtle);
		scroll-behavior: smooth;
	}

	.tool-calls-list {
		display: flex;
		flex-direction: column;
		min-height: 100%;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		flex: 1;
		gap: var(--space-3);
		color: var(--color-text-muted);
		opacity: 0.3;
		padding: var(--space-8) 0;
	}

	.empty-icon-wrapper {
		font-size: 24px;
		font-weight: bold;
		animation: blink 1.2s infinite;
	}

	@keyframes blink {
		0%, 100% { opacity: 1; }
		50% { opacity: 0; }
	}

	.empty-text {
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	/* Scrollbar styling */
	.tool-calls-view::-webkit-scrollbar {
		width: 6px;
	}

	.tool-calls-view::-webkit-scrollbar-track {
		background: transparent;
	}

	.tool-calls-view::-webkit-scrollbar-thumb {
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
	}
</style>
