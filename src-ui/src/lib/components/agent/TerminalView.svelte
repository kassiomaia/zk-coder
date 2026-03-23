<script lang="ts">
	/**
	 * TerminalView Component
	 * Displays terminal output from agent command execution.
	 * Auto-scrolls to bottom on new output.
	 */
	import type { TerminalLine } from '$lib/types/agent';

	interface Props {
		output: TerminalLine[];
	}

	let { output }: Props = $props();
	let container: HTMLDivElement | undefined = $state();

	/** Auto-scroll to bottom when new output arrives */
	$effect(() => {
		if (output.length > 0 && container) {
			container.scrollTop = container.scrollHeight;
		}
	});

	function formatTime(isoString: string): string {
		const date = new Date(isoString);
		return date.toLocaleTimeString('en-US', {
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit',
			hour12: false
		});
	}
</script>

<div class="terminal-view" bind:this={container} role="log" aria-label="Terminal output">
	{#each output as line (line.id)}
		<div class="terminal-line {line.type}">
			{#if line.timestamp}
				<span class="timestamp">{formatTime(line.timestamp)}</span>
			{/if}
			<span class="content">{line.content}</span>
		</div>
	{:else}
		<div class="empty-terminal">
			<span>Terminal output will appear here</span>
		</div>
	{/each}
</div>

<style>
	.terminal-view {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--space-2);
		background: var(--color-neutral-950);
		border-radius: var(--radius-md);
		margin: 0 var(--space-4) var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		line-height: var(--leading-relaxed);
	}

	.terminal-line {
		display: flex;
		gap: var(--space-2);
		padding: var(--space-0-5) 0;
	}

	.terminal-line.stdout {
		color: var(--color-text-primary);
	}

	.terminal-line.stderr {
		color: var(--color-error);
	}

	.terminal-line.command {
		color: var(--color-primary-400);
	}

	.terminal-line.info {
		color: var(--color-text-muted);
		font-style: italic;
	}

	.timestamp {
		flex-shrink: 0;
		color: var(--color-text-muted);
		opacity: 0.6;
	}

	.content {
		white-space: pre-wrap;
		word-break: break-all;
	}

	.empty-terminal {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		min-height: 80px;
		color: var(--color-text-muted);
		opacity: 0.5;
	}

	/* Scrollbar styling */
	.terminal-view::-webkit-scrollbar {
		width: 6px;
	}

	.terminal-view::-webkit-scrollbar-track {
		background: transparent;
	}

	.terminal-view::-webkit-scrollbar-thumb {
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
	}
</style>
