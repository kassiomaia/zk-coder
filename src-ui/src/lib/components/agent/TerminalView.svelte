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
	<div class="terminal-content-inner">
		{#each output as line (line.id)}
			<div class="terminal-line {line.type}">
				<span class="line-prefix">
					{#if line.type === 'command'}
						<span class="command-char">❯</span>
					{:else}
						<span class="normal-char"></span>
					{/if}
				</span>
				<div class="line-body">
					<span class="content">{line.content}</span>
					{#if line.timestamp}
						<span class="timestamp-inline" title={new Date(line.timestamp).toLocaleString()}>
							{formatTime(line.timestamp)}
						</span>
					{/if}
				</div>
			</div>
		{:else}
			<div class="empty-terminal">
				<div class="empty-icon-wrapper">
					<span class="empty-char">_</span>
				</div>
				<span class="empty-text">No output recorded</span>
			</div>
		{/each}
	</div>
</div>

<style>
	.terminal-view {
		flex: 1;
		min-height: 120px;
		overflow-y: auto;
		background: var(--color-neutral-950);
		margin: 0;
		font-family: var(--font-mono);
		font-size: 11px;
		line-height: 1.6;
		border-top: 1px solid var(--color-border-subtle);
		scroll-behavior: smooth;
	}

	.terminal-content-inner {
		display: flex;
		flex-direction: column;
		padding: var(--space-4);
		min-height: 100%;
	}

	.terminal-line {
		display: flex;
		gap: var(--space-2);
		padding: 1px 0;
		position: relative;
	}

	.line-prefix {
		flex-shrink: 0;
		width: 12px;
		display: flex;
		align-items: flex-start;
		justify-content: center;
		user-select: none;
	}

	.command-char {
		color: var(--color-primary-500);
		font-weight: bold;
		font-size: 10px;
	}

	.line-body {
		flex: 1;
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		min-width: 0;
		gap: var(--space-4);
	}

	.content {
		white-space: pre-wrap;
		word-break: break-all;
		color: var(--color-neutral-300);
	}

	.terminal-line.stdout .content {
		color: var(--color-neutral-100);
	}

	.terminal-line.stderr .content {
		color: #ffa198;
	}

	.terminal-line.command .content {
		color: var(--color-primary-300);
		font-weight: 500;
	}

	.terminal-line.info .content {
		color: var(--color-text-muted);
		font-style: italic;
		opacity: 0.8;
	}

	.timestamp-inline {
		flex-shrink: 0;
		color: var(--color-text-muted);
		opacity: 0;
		font-size: 9px;
		user-select: none;
		transition: opacity var(--transition-fast);
		padding-top: 2px;
	}

	.terminal-line:hover .timestamp-inline {
		opacity: 0.4;
	}

	.empty-terminal {
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
