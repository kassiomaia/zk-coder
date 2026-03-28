<script lang="ts">
    import {
        AlertCircle,
        CheckCircle2,
        ChevronRight,
        Clock,
        Code,
        FilePenLine,
        FileSearch,
        FileText,
        Globe,
        Terminal
    } from 'lucide-svelte';
    import type {ToolCallRecord} from '$lib/types/agent';
    import hljs from 'highlight.js';
    import 'highlight.js/styles/github-dark.css';

    interface Props {
		record: ToolCallRecord;
	}

	let { record }: Props = $props();
	let isOpen = $state(false);

	const toolIconMap: Record<string, any> = {
		'fs_read_file': FileText,
		'fs_write_file': FilePenLine,
		'fs_glob': FileSearch,
		'fs_grep': FileSearch,
		'execute_command': Terminal,
		'web_request': Globe,
		'llm_request': Code
	};

	const ToolIcon = $derived(toolIconMap[record.tool] || Code);

	function formatJSON(obj: any): string {
		try {
			const json = JSON.stringify(obj, null, 2);
			return hljs.highlight(json, { language: 'json' }).value;
		} catch (e) {
			return String(obj);
		}
	}

	function getPrimaryArg(record: ToolCallRecord): string {
		const args = record.arguments;
		if (args.path) return String(args.path);
		if (args.pattern) return String(args.pattern);
		if (args.command) return String(args.command);
		if (args.url) return String(args.url);
		return '';
	}

	function formatTime(isoString: string): string {
		return new Date(isoString).toLocaleTimeString('en-US', {
			hour12: false,
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		});
	}
</script>

<div class="tool-call-item" class:is-open={isOpen} class:has-error={record.isError}>
	<button class="item-header" onclick={() => (isOpen = !isOpen)} type="button">
		<span class="chevron" class:rotated={isOpen}>
			<ChevronRight size={14} />
		</span>
		
		<span class="tool-icon">
			<ToolIcon size={14} />
		</span>

		<span class="tool-name">{record.tool}</span>
		
		{#if getPrimaryArg(record)}
			<span class="primary-arg">{getPrimaryArg(record)}</span>
		{/if}

		<div class="header-meta">
			{#if record.durationMs}
				<span class="latency">
					<Clock size={10} />
					{record.durationMs}ms
				</span>
			{/if}
			
			{#if record.result}
				<span class="status-icon" class:error={record.isError}>
					{#if record.isError}
						<AlertCircle size={14} />
					{:else}
						<CheckCircle2 size={14} />
					{/if}
				</span>
			{/if}
		</div>
	</button>

	{#if isOpen}
		<div class="item-details">
			<div class="detail-section">
				<div class="section-label">Arguments</div>
				<pre class="json-block">{@html formatJSON(record.arguments)}</pre>
			</div>

			{#if record.result}
				<div class="detail-section">
					<div class="section-label">Result</div>
					<pre class="json-block">{@html formatJSON(record.result)}</pre>
				</div>
			{/if}
			
			<div class="timestamp-footer">
				Executed at {formatTime(record.timestamp)}
			</div>
		</div>
	{/if}
</div>

<style>
	.tool-call-item {
		border-bottom: 1px solid var(--color-border-subtle);
		background: var(--color-bg-secondary);
		transition: background var(--transition-fast);
	}

	.tool-call-item:hover {
		background: var(--color-interactive-hover);
	}

	.item-header {
		display: flex;
		align-items: center;
		width: 100%;
		padding: var(--space-2) var(--space-4);
		background: transparent;
		border: none;
		cursor: pointer;
		gap: var(--space-2);
		text-align: left;
	}

	.chevron {
		display: flex;
		color: var(--color-text-muted);
		transition: transform var(--transition-fast);
	}

	.chevron.rotated {
		transform: rotate(90deg);
	}

	.tool-icon {
		display: flex;
		color: var(--color-primary-400);
	}

	.tool-name {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: var(--font-semibold);
		color: var(--color-text-primary);
	}

	.primary-arg {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: var(--space-50);
		opacity: 0.8;
	}

	.header-meta {
		margin-left: auto;
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.latency {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}

	.status-icon {
		display: flex;
		color: var(--color-success);
	}

	.status-icon.error {
		color: var(--color-error);
	}

	.item-details {
		padding: 0 var(--space-4) var(--space-4) var(--space-10);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		border-top: 1px solid var(--color-border-subtle);
		background: var(--color-neutral-950);
	}

	.detail-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.section-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-bold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		padding-top: var(--space-3);
	}

	.json-block {
		margin: 0;
		padding: var(--space-3);
		background: var(--color-neutral-900);
		border-radius: var(--radius-md);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		line-height: var(--leading-normal);
		overflow-x: auto;
		border: 1px solid var(--color-border-subtle);
	}

	.timestamp-footer {
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		opacity: 0.5;
		text-align: right;
	}

	.has-error {
		border-left: 2px solid var(--color-error);
	}
</style>
